import type {
  ClipEffect,
  ClipTransform,
  EditSession,
  EditorProject,
  EditorSessionSeed,
  ExportPreset,
  MediaAsset,
  TimelineClip,
  TimelineTrack,
  TimelineTrackType,
} from "../types/editor";

const DEFAULT_DURATION = 12;

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

export function createSeededEditSession(seed: EditorSessionSeed = {}): EditSession {
  const now = new Date().toISOString();
  const duration = seed.duration ?? DEFAULT_DURATION;
  const sourceNodeId = seed.sourceNodeId;
  const videoAsset = createMediaAsset({
    id: sourceNodeId ? `asset-${sourceNodeId}` : "asset-editor-seed",
    type: "video",
    name: seed.assetName ?? "linglux_generated_take.mp4",
    url: seed.assetUrl ?? `linglux://artifact/${sourceNodeId ?? "seed-video"}`,
    duration,
    sourceNodeId,
    createdAt: now,
  });

  const audioAsset = createMediaAsset({
    id: "asset-room-tone",
    type: "audio",
    name: "ambient_mix.wav",
    url: "linglux://asset/ambient_mix.wav",
    duration,
    createdAt: now,
  });

  const captionAsset = createMediaAsset({
    id: "asset-caption-bed",
    type: "caption",
    name: "字幕轨",
    url: "linglux://caption/default",
    duration: 4,
    createdAt: now,
  });

  const tracks: TimelineTrack[] = [
    createTrack("track-overlay", "overlay", "叠加轨"),
    createTrack("track-video", "video", "视频轨"),
    createTrack("track-caption", "caption", "字幕轨"),
    createTrack("track-audio", "audio", "音频轨"),
  ];

  tracks[0].clips.push(
    createClip({
      id: "clip-overlay-guide",
      assetId: videoAsset.id,
      trackId: "track-overlay",
      name: "片头叠加",
      type: "overlay",
      start: 0.6,
      duration: 2.6,
      trimStart: 0,
      trimEnd: 0,
      opacity: 0.42,
    }),
  );
  tracks[1].clips.push(
    createClip({
      id: "clip-main-video",
      assetId: videoAsset.id,
      trackId: "track-video",
      name: videoAsset.name,
      type: "video",
      start: 0,
      duration,
      trimStart: 0,
      trimEnd: 0,
    }),
  );
  tracks[2].clips.push(
    createClip({
      id: "clip-caption-1",
      assetId: captionAsset.id,
      trackId: "track-caption",
      name: "主字幕",
      type: "caption",
      start: 1,
      duration: 4,
      trimStart: 0,
      trimEnd: 0,
      captionText: "Linglux 自动剪辑草稿",
    }),
  );
  tracks[3].clips.push(
    createClip({
      id: "clip-audio-bed",
      assetId: audioAsset.id,
      trackId: "track-audio",
      name: audioAsset.name,
      type: "audio",
      start: 0,
      duration,
      trimStart: 0,
      trimEnd: 0,
      volume: 0.72,
    }),
  );

  const project: EditorProject = {
    id: `project-${Date.now()}`,
    name: sourceNodeId ? `剪辑会话 · ${sourceNodeId}` : "Linglux 剪辑工程",
    sourceNodeId,
    assets: [videoAsset, audioAsset, captionAsset],
    tracks,
    duration,
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
  return JSON.parse(JSON.stringify(project)) as EditorProject;
}

export function calculateProjectDuration(tracks: TimelineTrack[]) {
  const duration = tracks.flatMap((track) => track.clips).reduce((max, clip) => Math.max(max, clip.start + clip.duration), 0);

  return Math.max(DEFAULT_DURATION, Math.ceil(duration));
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
    locked: false,
    clips: [],
  };
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
    speed: 1,
    transform,
    effects: createDefaultEffects(),
    transition: options.type === "video" || options.type === "overlay" ? "soft" : undefined,
    captionText: options.captionText,
  };
}
