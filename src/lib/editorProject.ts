import type {
  AudioBeatMarker,
  ClipEffect,
  ClipTransform,
  DynamicComicCameraMotion,
  DynamicComicProject,
  DynamicComicShot,
  EditSession,
  EditorProject,
  EditorSessionSeed,
  ExportPreset,
  MediaAsset,
  TextClipStyle,
  TimelineClip,
  TimelineTrack,
  TimelineTrackType,
} from "../types/editor";

export const TIMELINE_BASE_PIXELS_PER_SECOND = 12;
export const TIMELINE_MIN_SCALE = 0.12;
export const TIMELINE_MAX_SCALE = 3.2;
export const TIMELINE_BUTTON_SCALE_STEP = 0.15;
const DEFAULT_TEXT_CLIP_STYLE: TextClipStyle = {
  fontFamily: "Arial",
  fontSize: 52,
  color: "#FFFFFF",
  letterSpacing: 0,
  lineHeight: 1.2,
  backgroundEnabled: false,
  backgroundColor: "#000000",
  backgroundWidth: 30,
  backgroundHeight: 42,
  backgroundXOffset: 0,
  backgroundYOffset: 0,
  backgroundCornerRadius: 0,
};

export const exportPresets: ExportPreset[] = [
  { id: "standard-1080", label: "1080p 标准", format: "mp4", resolution: "1080p", fps: 30, quality: "standard" },
  { id: "high-1080", label: "1080p 高质量", format: "mp4", resolution: "1080p", fps: 60, quality: "high" },
  { id: "draft-720", label: "720p 草稿", format: "webm", resolution: "720p", fps: 24, quality: "draft" },
  { id: "cinema-4k", label: "4K 成片", format: "mov", resolution: "4k", fps: 30, quality: "high" },
];

export function createDefaultTransform(): ClipTransform {
  return {
    x: 0,
    y: 0,
    scale: 1,
    rotation: 0,
    opacity: 1,
  };
}

export function createDefaultEffects(): ClipEffect[] {
  return [
    { id: "effect-color-base", type: "color", label: "基础调色", intensity: 32 },
    { id: "effect-soft-cut", type: "transition", label: "柔和转场", intensity: 18 },
  ];
}

export function createDefaultTextClipStyle(): TextClipStyle {
  return { ...DEFAULT_TEXT_CLIP_STYLE };
}

export function createDefaultDynamicComicCameraMotion(): DynamicComicCameraMotion {
  return {
    preset: "static",
    start: { x: 0.5, y: 0.5, scale: 1 },
    end: { x: 0.5, y: 0.5, scale: 1 },
    easing: "easeInOut",
  };
}

export function normalizeTextClipStyle(style?: Partial<TextClipStyle>): TextClipStyle {
  return {
    ...DEFAULT_TEXT_CLIP_STYLE,
    ...style,
  };
}

export function createSeededEditSession(seed: EditorSessionSeed = {}): EditSession {
  const now = new Date().toISOString();
  const sourceNodeId = seed.sourceNodeId;

  const tracks: TimelineTrack[] = [
    createTrack("track-overlay", "overlay", "叠加轨"),
    createTrack("track-video", "video", "视频轨"),
    createTrack("track-caption", "caption", "字幕轨"),
    createTrack("track-audio", "audio", "音频轨"),
  ];

  const project: EditorProject = {
    id: `project-${Date.now()}`,
    name: sourceNodeId ? `剪辑会话 · ${sourceNodeId}` : "Linglux 剪辑工程",
    sourceNodeId,
    mode: seed.mode === "dynamicComic" ? "dynamicComic" : "timeline",
    dynamicComic: seed.mode === "dynamicComic" ? { shots: [] } : undefined,
    assets: [],
    tracks,
    mainTrackMagnetEnabled: true,
    duration: 0,
    fps: 30,
    resolution: { width: 1920, height: 1080 },
    createdAt: now,
    updatedAt: now,
  };

  return {
    id: `session-${Date.now()}`,
    sourceNodeId,
    project,
    savedAt: now,
    isDirty: false,
  };
}

export function cloneProject(project: EditorProject): EditorProject {
  return normalizeEditorProject(JSON.parse(JSON.stringify(project)) as EditorProject);
}

export function normalizeEditorProject(project: EditorProject): EditorProject {
  project.mode = project.mode === "dynamicComic" ? "dynamicComic" : "timeline";

  if (project.mode === "dynamicComic") {
    project.dynamicComic = normalizeDynamicComicProject(project.dynamicComic);
  } else {
    delete project.dynamicComic;
  }

  project.mainTrackMagnetEnabled = project.mainTrackMagnetEnabled !== false;

  for (const track of project.tracks) {
    track.visible = track.visible !== false;
    track.mediaEnabled = track.mediaEnabled !== false;

    for (const clip of track.clips) {
      clip.visible = clip.visible !== false;

      if (clip.type === "caption") {
        clip.textStyle = normalizeTextClipStyle(clip.textStyle);
      }

      if (clip.type === "audio") {
        const beatMarkers = normalizeAudioBeatMarkers(clip.beatMarkers, clip.duration);

        if (beatMarkers.length > 0) {
          clip.beatMode = "auto";
          clip.beatMarkers = beatMarkers;
        } else {
          delete clip.beatMode;
          delete clip.beatMarkers;
        }
      } else {
        delete clip.beatMode;
        delete clip.beatMarkers;
      }
    }

    if (project.mainTrackMagnetEnabled && isPrimaryTimelineTrack(track)) {
      closeTimelineTrackGaps(track);
    }
  }

  if (!project.tracks.some((track) => track.clips.length > 0)) {
    project.duration = 0;
  }

  return project;
}

function normalizeDynamicComicProject(dynamicComic?: Partial<DynamicComicProject>): DynamicComicProject {
  const shots = Array.isArray(dynamicComic?.shots) ? dynamicComic.shots : [];
  const usedIds = new Set<string>();

  return {
    shots: shots
      .map((shot) => normalizeDynamicComicShot(shot, usedIds))
      .sort((left, right) => left.order - right.order),
  };
}

function normalizeDynamicComicShot(shot: Partial<DynamicComicShot>, usedIds: Set<string>): DynamicComicShot {
  const duration = finiteAtLeast(shot.duration, 0);
  const candidateId = nonEmptyString(shot.id);
  const id = candidateId && !usedIds.has(candidateId) ? candidateId : createUniqueDynamicComicShotId(usedIds);
  usedIds.add(id);

  return {
    id,
    order: finiteIntegerAtLeast(shot.order, 0),
    visualAssetId: nonEmptyString(shot.visualAssetId),
    visualClipId: nonEmptyString(shot.visualClipId),
    duration,
    focus: {
      x: finiteBetween(shot.focus?.x, 0, 1, 0.5),
      y: finiteBetween(shot.focus?.y, 0, 1, 0.5),
    },
    characterId: nonEmptyString(shot.characterId),
    dialogue: typeof shot.dialogue === "string" ? shot.dialogue : "",
    emotion: nonEmptyString(shot.emotion),
    pauseBefore: finiteAtLeast(shot.pauseBefore, 0),
    pauseAfter: finiteAtLeast(shot.pauseAfter, 0),
    cameraMotion: normalizeDynamicComicCameraMotion(shot.cameraMotion),
    transition: nonEmptyString(shot.transition),
    soundEffectAssetIds: Array.isArray(shot.soundEffectAssetIds)
      ? shot.soundEffectAssetIds.map(nonEmptyString).filter((id): id is string => Boolean(id))
      : [],
  };
}

function createUniqueDynamicComicShotId(usedIds: Set<string>) {
  let id: string;

  do {
    id = `shot-${globalThis.crypto?.randomUUID?.() ?? `${Date.now()}-${Math.random().toString(36).slice(2)}`}`;
  } while (usedIds.has(id));

  return id;
}

function normalizeDynamicComicCameraMotion(motion?: Partial<DynamicComicCameraMotion>): DynamicComicCameraMotion {
  const defaults = createDefaultDynamicComicCameraMotion();
  const presets = new Set<DynamicComicCameraMotion["preset"]>([
    "static",
    "pushIn",
    "pullOut",
    "panLeft",
    "panRight",
    "panUp",
    "panDown",
    "impactPush",
  ]);
  const easings = new Set<DynamicComicCameraMotion["easing"]>(["linear", "easeIn", "easeOut", "easeInOut"]);

  return {
    preset: motion?.preset && presets.has(motion.preset) ? motion.preset : defaults.preset,
    start: normalizeDynamicComicFrame(motion?.start, defaults.start),
    end: normalizeDynamicComicFrame(motion?.end, defaults.end),
    easing: motion?.easing && easings.has(motion.easing) ? motion.easing : defaults.easing,
  };
}

function normalizeDynamicComicFrame(
  frame: Partial<DynamicComicCameraMotion["start"]> | undefined,
  fallback: DynamicComicCameraMotion["start"],
) {
  return {
    x: finiteBetween(frame?.x, 0, 1, fallback.x),
    y: finiteBetween(frame?.y, 0, 1, fallback.y),
    scale: finiteAtLeast(frame?.scale, 0.01, fallback.scale),
  };
}

function nonEmptyString(value: unknown) {
  return typeof value === "string" && value.trim() ? value.trim() : undefined;
}

function finiteAtLeast(value: unknown, minimum: number, fallback = minimum) {
  return typeof value === "number" && Number.isFinite(value) ? Math.max(value, minimum) : fallback;
}

function finiteIntegerAtLeast(value: unknown, minimum: number, fallback = minimum) {
  return typeof value === "number" && Number.isFinite(value) ? Math.max(Math.trunc(value), minimum) : fallback;
}

function finiteBetween(value: unknown, minimum: number, maximum: number, fallback: number) {
  return typeof value === "number" && Number.isFinite(value) ? Math.min(Math.max(value, minimum), maximum) : fallback;
}

function normalizeAudioBeatMarkers(markers: AudioBeatMarker[] | undefined, duration: number) {
  if (!Array.isArray(markers) || duration <= 0) {
    return [];
  }

  return markers
    .filter((marker) => Number.isFinite(marker.time) && marker.time >= 0 && marker.time <= duration)
    .map((marker) => ({
      time: Number(marker.time.toFixed(3)),
      intensity: Number(Math.min(Math.max(Number.isFinite(marker.intensity) ? marker.intensity : 0.65, 0.25), 1).toFixed(2)),
    }))
    .sort((left, right) => left.time - right.time);
}

export function isPrimaryTimelineTrack(track: TimelineTrack) {
  return track.type === "video";
}

export function closeTimelineTrackGaps(track: TimelineTrack) {
  track.clips.sort((left, right) => left.start - right.start);

  let nextStart = 0;
  let changed = false;

  for (const clip of track.clips) {
    if (Math.abs(clip.start - nextStart) > 0.001) {
      clip.start = nextStart;
      changed = true;
    }

    nextStart += clip.duration;
  }

  return changed;
}

export function calculateProjectDuration(tracks: TimelineTrack[]) {
  const duration = tracks.flatMap((track) => track.clips).reduce((max, clip) => Math.max(max, clip.start + clip.duration), 0);

  return Math.max(duration, 0);
}

export function formatTimecode(seconds: number) {
  const safeSeconds = Math.max(seconds, 0);
  const minutes = Math.floor(safeSeconds / 60);
  const wholeSeconds = Math.floor(safeSeconds % 60);
  const frames = Math.floor((safeSeconds - Math.floor(safeSeconds)) * 30);

  return `${minutes.toString().padStart(2, "0")}:${wholeSeconds.toString().padStart(2, "0")}:${frames.toString().padStart(2, "0")}`;
}

function createMediaAsset(asset: MediaAsset): MediaAsset {
  return asset;
}

function createTrack(id: string, type: TimelineTrackType, label: string): TimelineTrack {
  return {
    id,
    type,
    label,
    muted: false,
    visible: true,
    mediaEnabled: true,
    locked: false,
    clips: [],
  };
}

export function createTimelineClip(options: {
  id: string;
  assetId: string;
  trackId: string;
  name: string;
  type: TimelineTrackType;
  start: number;
  duration: number;
  volume?: number;
  opacity?: number;
  captionText?: string;
  textStyle?: Partial<TextClipStyle>;
}): TimelineClip {
  return createClip({
    id: options.id,
    assetId: options.assetId,
    trackId: options.trackId,
    name: options.name,
    type: options.type,
    start: options.start,
    duration: options.duration,
    trimStart: 0,
    trimEnd: 0,
    volume: options.volume,
    opacity: options.opacity,
    captionText: options.captionText,
    textStyle: options.textStyle,
  });
}

function createClip(options: {
  id: string;
  assetId: string;
  trackId: string;
  name: string;
  type: TimelineTrackType;
  start: number;
  duration: number;
  trimStart: number;
  trimEnd: number;
  volume?: number;
  opacity?: number;
  captionText?: string;
  textStyle?: Partial<TextClipStyle>;
}): TimelineClip {
  const transform = createDefaultTransform();
  transform.opacity = options.opacity ?? transform.opacity;

  return {
    id: options.id,
    assetId: options.assetId,
    trackId: options.trackId,
    name: options.name,
    type: options.type,
    start: options.start,
    duration: options.duration,
    trimStart: options.trimStart,
    trimEnd: options.trimEnd,
    volume: options.volume ?? 1,
    muted: false,
    visible: true,
    speed: 1,
    transform,
    effects: createDefaultEffects(),
    transition: options.type === "video" || options.type === "overlay" ? "soft" : undefined,
    captionText: options.captionText,
    textStyle: options.type === "caption" ? normalizeTextClipStyle(options.textStyle) : undefined,
  };
}
