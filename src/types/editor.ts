export type ArtifactType = "video" | "image" | "audio" | "project";

export interface Artifact {
  id: string;
  type: ArtifactType;
  name: string;
  sourceNodeId: string;
  url: string;
  duration: number;
  createdAt: string;
}

export type MediaAssetType = "video" | "image" | "audio" | "caption";

export interface MediaAsset {
  id: string;
  type: MediaAssetType;
  name: string;
  sourceNodeId?: string;
  url: string;
  duration: number;
  width?: number;
  height?: number;
  createdAt: string;
}

export type TimelineTrackType = "video" | "audio" | "caption" | "overlay";

export interface ClipTransform {
  x: number;
  y: number;
  scale: number;
  rotation: number;
  opacity: number;
}

export interface ClipEffect {
  id: string;
  type: "color" | "filter" | "transition";
  label: string;
  intensity: number;
}

export interface TimelineClip {
  id: string;
  assetId: string;
  trackId: string;
  name: string;
  type: TimelineTrackType;
  start: number;
  duration: number;
  trimStart: number;
  trimEnd: number;
  volume: number;
  muted: boolean;
  speed: number;
  transform: ClipTransform;
  effects: ClipEffect[];
  transition?: string;
  captionText?: string;
}

export interface CaptionClip extends TimelineClip {
  type: "caption";
  captionText: string;
}

export interface TimelineTrack {
  id: string;
  type: TimelineTrackType;
  label: string;
  muted: boolean;
  locked: boolean;
  clips: TimelineClip[];
}

export interface ExportPreset {
  id: string;
  label: string;
  format: "mp4" | "webm" | "mov";
  resolution: "720p" | "1080p" | "1440p" | "4k";
  fps: 24 | 30 | 60;
  quality: "draft" | "standard" | "high";
}

export interface EditorProject {
  id: string;
  name: string;
  sourceNodeId?: string;
  assets: MediaAsset[];
  tracks: TimelineTrack[];
  duration: number;
  fps: number;
  resolution: {
    width: number;
    height: number;
  };
  createdAt: string;
  updatedAt: string;
}

export interface EditSession {
  id: string;
  sourceNodeId?: string;
  project: EditorProject;
  savedAt?: string;
  isDirty: boolean;
}

export interface EditorExportRequest {
  sessionId: string;
  project: EditorProject;
  preset: ExportPreset;
}

export interface EditorExportResult {
  artifact: Artifact;
  preset: ExportPreset;
}

export interface EditorSessionSeed {
  sourceNodeId?: string;
  assetName?: string;
  assetUrl?: string;
  duration?: number;
}
