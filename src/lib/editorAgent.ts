import {
  calculateProjectDuration,
  closeTimelineTrackGaps,
  cloneProject,
  createTimelineClip,
  isPrimaryTimelineTrack,
} from "./editorProject";
import type {
  AgentEditOperation,
  AgentEditPlan,
  AgentPlanPreview,
  AgentProjectSnapshot,
} from "../types/agent";
import type {
  EditorProject,
  MediaAsset,
  TimelineClip,
  TimelineTrack,
  TimelineTrackType,
} from "../types/editor";

const MIN_CLIP_DURATION_MS = 100;

export class AgentPlanValidationError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "AgentPlanValidationError";
  }
}

export function createAgentProjectSnapshot(options: {
  project: EditorProject;
  editorVersion: number;
  playhead: number;
  selectedClipId?: string;
  selectedAssetIds: string[];
}): AgentProjectSnapshot {
  const { project } = options;

  return {
    projectId: project.id,
    projectName: project.name,
    editorVersion: options.editorVersion,
    durationMs: secondsToMs(project.duration),
    playheadMs: secondsToMs(options.playhead),
    selectedClipId: options.selectedClipId || undefined,
    selectedAssetIds: [...options.selectedAssetIds],
    mainTrackMagnetEnabled: project.mainTrackMagnetEnabled,
    assets: project.assets.map((asset) => ({
      id: asset.id,
      name: asset.name,
      type: asset.type,
      durationMs: secondsToMs(asset.duration),
    })),
    tracks: project.tracks.map((track) => ({
      id: track.id,
      label: track.label,
      type: track.type,
      locked: track.locked,
      clips: track.clips.map((clip) => {
        const asset = findAsset(project, clip.assetId);
        const sourceInMs = secondsToMs(clip.trimStart);
        const sourceDurationMs = secondsToMs(clip.duration * Math.max(clip.speed, 0.01));
        return {
          id: clip.id,
          assetId: clip.assetId,
          name: clip.name,
          type: clip.type,
          trackId: clip.trackId,
          timelineStartMs: secondsToMs(clip.start),
          timelineDurationMs: secondsToMs(clip.duration),
          sourceInMs,
          sourceOutMs: Math.min(sourceInMs + sourceDurationMs, secondsToMs(asset.duration)),
          speed: clip.speed,
        };
      }),
    })),
    characterVoiceProfiles: (project.dynamicComic?.characterVoiceProfiles ?? []).map((profile) => ({
      id: profile.id,
      name: profile.name,
      color: profile.color,
      voice: profile.voice,
      defaultEmotion: profile.defaultEmotion,
      defaultSpeed: profile.defaultSpeed,
    })),
    dynamicComicShots: (project.dynamicComic?.shots ?? []).map((shot) => ({
      id: shot.id,
      order: shot.order,
      characterId: shot.characterId,
      emotion: shot.emotion,
      speechSpeed: shot.speechSpeed,
    })),
  };
}

export function previewAgentEditPlan(
  project: EditorProject,
  plan: AgentEditPlan,
  currentEditorVersion: number,
): AgentPlanPreview {
  if (plan.projectId !== project.id) {
    throw new AgentPlanValidationError("该计划不属于当前工程。");
  }

  if (plan.baseEditorVersion !== currentEditorVersion) {
    throw new AgentPlanValidationError("计划生成后时间线已经变化，请重新生成。");
  }

  if (plan.operations.length === 0) {
    throw new AgentPlanValidationError("计划中没有可执行的剪辑操作。");
  }

  const workingProject = cloneProject(project);
  const descriptions: string[] = [];
  const affectedTrackIds = new Set<string>();
  let selectedClipId: string | undefined;

  for (let index = 0; index < plan.operations.length; index += 1) {
    const operation = plan.operations[index];
    const result = applyOperation(workingProject, operation, index);
    result.affectedTrackIds.forEach((trackId) => affectedTrackIds.add(trackId));
    descriptions.push(result.description);
    selectedClipId = result.selectedClipId ?? selectedClipId;
  }

  normalizeTimeline(workingProject);

  return {
    project: workingProject,
    descriptions,
    affectedTrackIds: [...affectedTrackIds],
    selectedClipId,
  };
}

function applyOperation(project: EditorProject, operation: AgentEditOperation, operationIndex: number) {
  switch (operation.type) {
    case "addAssetRange":
      return addAssetRange(project, operation, operationIndex);
    case "keepClipSourceRange":
      return keepClipSourceRange(project, operation);
    case "removeClipSourceRange":
      return removeClipSourceRange(project, operation, operationIndex);
    case "splitClipAtTimeline":
      return splitClipAtTimeline(project, operation, operationIndex);
    case "deleteClip":
      return deleteClip(project, operation);
    case "moveClip":
      return moveClip(project, operation);
  }
}

function addAssetRange(
  project: EditorProject,
  operation: Extract<AgentEditOperation, { type: "addAssetRange" }>,
  operationIndex: number,
) {
  const asset = findAsset(project, operation.assetId);
  const track = findTrack(project, operation.trackId);
  assertTrackUnlocked(track);
  assertTrackAcceptsAsset(track, asset);
  const range = validateSourceRange(asset, operation.sourceInMs, operation.sourceOutMs);
  const timelineStartMs = operation.timelineStartMs === undefined
    ? secondsToMs(track.clips.reduce((end, clip) => Math.max(end, clip.start + clip.duration), 0))
    : nonNegativeInteger(operation.timelineStartMs, "时间线起点");
  const clipId = uniqueClipId(project, `agent-${operation.id || operationIndex}`);
  const clip = createTimelineClip({
    id: clipId,
    assetId: asset.id,
    trackId: track.id,
    name: asset.name,
    type: clipTypeForTrack(track),
    start: msToSeconds(timelineStartMs),
    duration: msToSeconds(range.outMs - range.inMs),
    volume: asset.type === "audio" ? 0.82 : 1,
    opacity: track.type === "overlay" ? 0.72 : 1,
  });
  clip.trimStart = msToSeconds(range.inMs);
  clip.trimEnd = Math.max(0, asset.duration - msToSeconds(range.outMs));

  if (project.mainTrackMagnetEnabled && isPrimaryTimelineTrack(track)) {
    insertIntoMagneticTrack(track, clip, msToSeconds(timelineStartMs));
  } else {
    track.clips.push(clip);
    sortTrack(track);
  }

  return {
    description: `把 ${asset.name} 的 ${formatAgentTime(range.inMs)}–${formatAgentTime(range.outMs)} 加入 ${track.label}`,
    affectedTrackIds: [track.id],
    selectedClipId: clip.id,
  };
}

function keepClipSourceRange(
  project: EditorProject,
  operation: Extract<AgentEditOperation, { type: "keepClipSourceRange" }>,
) {
  const located = findLocatedClip(project, operation.clipId);
  assertTrackUnlocked(located.track);
  const asset = findAsset(project, located.clip.assetId);
  const range = validateSourceRange(asset, operation.sourceInMs, operation.sourceOutMs);
  const speed = positiveSpeed(located.clip.speed);

  located.clip.trimStart = msToSeconds(range.inMs);
  located.clip.trimEnd = Math.max(0, asset.duration - msToSeconds(range.outMs));
  located.clip.duration = msToSeconds(range.outMs - range.inMs) / speed;

  return {
    description: `保留 ${located.clip.name} 的 ${formatAgentTime(range.inMs)}–${formatAgentTime(range.outMs)}`,
    affectedTrackIds: [located.track.id],
    selectedClipId: located.clip.id,
  };
}

function removeClipSourceRange(
  project: EditorProject,
  operation: Extract<AgentEditOperation, { type: "removeClipSourceRange" }>,
  operationIndex: number,
) {
  const located = findLocatedClip(project, operation.clipId);
  assertTrackUnlocked(located.track);
  const asset = findAsset(project, located.clip.assetId);
  const removeRange = validateSourceRange(asset, operation.sourceInMs, operation.sourceOutMs);
  const clipRange = visibleSourceRangeMs(located.clip, asset);

  if (removeRange.inMs < clipRange.inMs || removeRange.outMs > clipRange.outMs) {
    throw new AgentPlanValidationError(`删除区间必须位于片段 ${located.clip.name} 当前可见的源区间内。`);
  }

  const leftDurationMs = removeRange.inMs - clipRange.inMs;
  const rightDurationMs = clipRange.outMs - removeRange.outMs;
  const speed = positiveSpeed(located.clip.speed);
  const replacements: TimelineClip[] = [];

  if (leftDurationMs >= MIN_CLIP_DURATION_MS) {
    const left = cloneClip(located.clip);
    left.duration = msToSeconds(leftDurationMs) / speed;
    left.trimEnd = Math.max(0, asset.duration - msToSeconds(removeRange.inMs));
    replacements.push(left);
  }

  if (rightDurationMs >= MIN_CLIP_DURATION_MS) {
    const right = cloneClip(located.clip);
    right.id = uniqueClipId(project, `agent-${operation.id || operationIndex}-right`);
    right.name = `${located.clip.name} · B`;
    right.trimStart = msToSeconds(removeRange.outMs);
    right.trimEnd = Math.max(0, asset.duration - msToSeconds(clipRange.outMs));
    right.duration = msToSeconds(rightDurationMs) / speed;
    right.start = located.clip.start + msToSeconds(leftDurationMs) / speed;
    replacements.push(right);
  }

  located.track.clips.splice(located.index, 1, ...replacements);
  sortTrack(located.track);

  return {
    description: `删除 ${located.clip.name} 的 ${formatAgentTime(removeRange.inMs)}–${formatAgentTime(removeRange.outMs)}`,
    affectedTrackIds: [located.track.id],
    selectedClipId: replacements[0]?.id,
  };
}

function splitClipAtTimeline(
  project: EditorProject,
  operation: Extract<AgentEditOperation, { type: "splitClipAtTimeline" }>,
  operationIndex: number,
) {
  const located = findLocatedClip(project, operation.clipId);
  assertTrackUnlocked(located.track);
  const timelineTimeMs = nonNegativeInteger(operation.timelineTimeMs, "分割时间");
  const clipStartMs = secondsToMs(located.clip.start);
  const clipEndMs = secondsToMs(located.clip.start + located.clip.duration);

  if (timelineTimeMs - clipStartMs < MIN_CLIP_DURATION_MS || clipEndMs - timelineTimeMs < MIN_CLIP_DURATION_MS) {
    throw new AgentPlanValidationError(`分割点不在片段 ${located.clip.name} 的有效内部区间。`);
  }

  const firstTimelineDurationMs = timelineTimeMs - clipStartMs;
  const secondTimelineDurationMs = clipEndMs - timelineTimeMs;
  const second = cloneClip(located.clip);
  second.id = uniqueClipId(project, `agent-${operation.id || operationIndex}-split`);
  second.name = `${located.clip.name} · B`;
  second.start = msToSeconds(timelineTimeMs);
  second.duration = msToSeconds(secondTimelineDurationMs);
  second.trimStart += msToSeconds(firstTimelineDurationMs) * positiveSpeed(located.clip.speed);
  located.clip.duration = msToSeconds(firstTimelineDurationMs);
  located.track.clips.splice(located.index + 1, 0, second);

  return {
    description: `在时间线 ${formatAgentTime(timelineTimeMs)} 分割 ${located.clip.name}`,
    affectedTrackIds: [located.track.id],
    selectedClipId: second.id,
  };
}

function deleteClip(
  project: EditorProject,
  operation: Extract<AgentEditOperation, { type: "deleteClip" }>,
) {
  const located = findLocatedClip(project, operation.clipId);
  assertTrackUnlocked(located.track);
  located.track.clips.splice(located.index, 1);

  return {
    description: `删除片段 ${located.clip.name}`,
    affectedTrackIds: [located.track.id],
    selectedClipId: undefined,
  };
}

function moveClip(
  project: EditorProject,
  operation: Extract<AgentEditOperation, { type: "moveClip" }>,
) {
  const located = findLocatedClip(project, operation.clipId);
  const targetTrack = findTrack(project, operation.trackId);
  const asset = findAsset(project, located.clip.assetId);
  assertTrackUnlocked(located.track);
  assertTrackUnlocked(targetTrack);
  assertTrackAcceptsAsset(targetTrack, asset);
  const timelineStartMs = nonNegativeInteger(operation.timelineStartMs, "移动目标时间");

  located.track.clips.splice(located.index, 1);
  located.clip.trackId = targetTrack.id;
  located.clip.start = msToSeconds(timelineStartMs);

  if (project.mainTrackMagnetEnabled && isPrimaryTimelineTrack(targetTrack)) {
    insertIntoMagneticTrack(targetTrack, located.clip, located.clip.start);
  } else {
    targetTrack.clips.push(located.clip);
    sortTrack(targetTrack);
  }

  return {
    description: `把 ${located.clip.name} 移到 ${targetTrack.label} 的 ${formatAgentTime(timelineStartMs)}`,
    affectedTrackIds: [...new Set([located.track.id, targetTrack.id])],
    selectedClipId: located.clip.id,
  };
}

function normalizeTimeline(project: EditorProject) {
  for (const track of project.tracks) {
    if (project.mainTrackMagnetEnabled && isPrimaryTimelineTrack(track)) {
      closeTimelineTrackGaps(track);
    } else {
      sortTrack(track);
    }
  }

  project.duration = calculateProjectDuration(project.tracks);
  project.updatedAt = new Date().toISOString();
}

function validateSourceRange(asset: MediaAsset, sourceInMs: number, sourceOutMs: number) {
  const inMs = nonNegativeInteger(sourceInMs, "源入点");
  const outMs = nonNegativeInteger(sourceOutMs, "源出点");
  const assetDurationMs = secondsToMs(asset.duration);

  if (assetDurationMs <= 0) {
    throw new AgentPlanValidationError(`素材 ${asset.name} 的时长未知，暂时无法精确剪辑。`);
  }

  if (outMs - inMs < MIN_CLIP_DURATION_MS) {
    throw new AgentPlanValidationError(`素材 ${asset.name} 的保留区间过短或顺序无效。`);
  }

  if (outMs > assetDurationMs) {
    throw new AgentPlanValidationError(`时间 ${formatAgentTime(outMs)} 超出素材 ${asset.name} 的时长。`);
  }

  return { inMs, outMs };
}

function visibleSourceRangeMs(clip: TimelineClip, asset: MediaAsset) {
  const inMs = secondsToMs(clip.trimStart);
  const requestedOutMs = inMs + secondsToMs(clip.duration * positiveSpeed(clip.speed));
  return {
    inMs,
    outMs: Math.min(requestedOutMs, secondsToMs(asset.duration - clip.trimEnd)),
  };
}

function findAsset(project: EditorProject, assetId: string) {
  const asset = project.assets.find((item) => item.id === assetId);

  if (!asset) {
    throw new AgentPlanValidationError(`找不到素材 ${assetId}。`);
  }

  return asset;
}

function findTrack(project: EditorProject, trackId: string) {
  const track = project.tracks.find((item) => item.id === trackId);

  if (!track) {
    throw new AgentPlanValidationError(`找不到轨道 ${trackId}。`);
  }

  return track;
}

function findLocatedClip(project: EditorProject, clipId: string) {
  for (const track of project.tracks) {
    const index = track.clips.findIndex((clip) => clip.id === clipId);

    if (index !== -1) {
      return { track, index, clip: track.clips[index] };
    }
  }

  throw new AgentPlanValidationError(`找不到片段 ${clipId}。`);
}

function assertTrackUnlocked(track: TimelineTrack) {
  if (track.locked) {
    throw new AgentPlanValidationError(`轨道 ${track.label} 已锁定。`);
  }
}

function assertTrackAcceptsAsset(track: TimelineTrack, asset: MediaAsset) {
  const compatibleTypes: TimelineTrackType[] = asset.type === "audio"
    ? ["audio"]
    : asset.type === "caption"
      ? ["caption"]
      : ["video", "overlay"];

  if (!compatibleTypes.includes(track.type)) {
    throw new AgentPlanValidationError(`素材 ${asset.name} 与轨道 ${track.label} 不兼容。`);
  }
}

function clipTypeForTrack(track: TimelineTrack): TimelineTrackType {
  return track.type;
}

function insertIntoMagneticTrack(track: TimelineTrack, clip: TimelineClip, requestedStart: number) {
  const clips = [...track.clips].sort((left, right) => left.start - right.start);
  const insertionIndex = clips.findIndex((item) => requestedStart < item.start + item.duration / 2);

  if (insertionIndex === -1) {
    clips.push(clip);
  } else {
    clips.splice(insertionIndex, 0, clip);
  }

  track.clips = clips;
  closeTimelineTrackGaps(track);
}

function sortTrack(track: TimelineTrack) {
  track.clips.sort((left, right) => left.start - right.start);
}

function uniqueClipId(project: EditorProject, seed: string) {
  const existingIds = new Set(project.tracks.flatMap((track) => track.clips.map((clip) => clip.id)));
  let candidate = `clip-${seed.replace(/[^a-zA-Z0-9_-]/g, "-")}`;
  let suffix = 1;

  while (existingIds.has(candidate)) {
    candidate = `clip-${seed.replace(/[^a-zA-Z0-9_-]/g, "-")}-${suffix}`;
    suffix += 1;
  }

  return candidate;
}

function cloneClip(clip: TimelineClip): TimelineClip {
  return JSON.parse(JSON.stringify(clip)) as TimelineClip;
}

function positiveSpeed(speed: number) {
  if (!Number.isFinite(speed) || speed <= 0) {
    throw new AgentPlanValidationError("片段速度无效。");
  }

  return speed;
}

function nonNegativeInteger(value: number, label: string) {
  if (!Number.isInteger(value) || value < 0) {
    throw new AgentPlanValidationError(`${label}必须是非负整数毫秒。`);
  }

  return value;
}

function secondsToMs(seconds: number) {
  return Math.round(Math.max(0, seconds) * 1000);
}

function msToSeconds(milliseconds: number) {
  return milliseconds / 1000;
}

export function formatAgentTime(milliseconds: number) {
  const totalSeconds = Math.max(0, Math.round(milliseconds / 1000));
  const hours = Math.floor(totalSeconds / 3600);
  const minutes = Math.floor((totalSeconds % 3600) / 60);
  const seconds = totalSeconds % 60;

  if (hours > 0) {
    return `${hours}:${minutes.toString().padStart(2, "0")}:${seconds.toString().padStart(2, "0")}`;
  }

  return `${minutes}:${seconds.toString().padStart(2, "0")}`;
}
