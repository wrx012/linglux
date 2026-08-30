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

export type AudioPreviewKind = "lofi" | "chiptune" | "pulse";

export interface AudioTrackPreset {
  id: string;
  title: string;
  duration: number;
  durationLabel: string;
  previewKind: AudioPreviewKind;
}

export interface TextTemplatePreset {
  id: string;
  title: string;
  subtitle: string;
  captionText: string;
  duration: number;
}

export interface MediaAsset {
  id: string;
  type: MediaAssetType;
  name: string;
  sourceNodeId?: string;
  url: string;
  filePath?: string;
  proxyPath?: string;
  contentFingerprint?: string;
  thumbnailUrl?: string;
  waveformPeaks?: number[];
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

export interface AudioBeatMarker {
  time: number;
  intensity: number;
}

export interface TextClipStyle {
  fontFamily: string;
  fontSize: number;
  color: string;
  letterSpacing: number;
  lineHeight: number;
  backgroundEnabled: boolean;
  backgroundColor: string;
  backgroundWidth: number;
  backgroundHeight: number;
  backgroundXOffset: number;
  backgroundYOffset: number;
  backgroundCornerRadius: number;
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
  visible: boolean;
  speed: number;
  transform: ClipTransform;
  effects: ClipEffect[];
  transition?: string;
  captionText?: string;
  textStyle?: TextClipStyle;
  beatMode?: "auto";
  beatMarkers?: AudioBeatMarker[];
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
  visible: boolean;
  mediaEnabled: boolean;
  locked: boolean;
  clips: TimelineClip[];
}

export type EditorProjectMode = "timeline" | "dynamicComic";

export type DynamicComicCameraPreset =
  | "static"
  | "pushIn"
  | "pullOut"
  | "panLeft"
  | "panRight"
  | "panUp"
  | "panDown"
  | "impactPush";

export type DynamicComicCameraEasing = "linear" | "easeIn" | "easeOut" | "easeInOut";

export interface DynamicComicFrame {
  x: number;
  y: number;
  scale: number;
}

export interface DynamicComicCameraMotion {
  preset: DynamicComicCameraPreset;
  start: DynamicComicFrame;
  end: DynamicComicFrame;
  easing: DynamicComicCameraEasing;
}

export interface DynamicComicShot {
  id: string;
  order: number;
  visualAssetId?: string;
  visualClipId?: string;
  duration: number;
  focus: {
    x: number;
    y: number;
  };
  characterId?: string;
  dialogue: string;
  emotion?: string;
  pauseBefore: number;
  pauseAfter: number;
  cameraMotion: DynamicComicCameraMotion;
  transition?: string;
  soundEffectAssetIds: string[];
}

export interface DynamicComicProject {
  shots: DynamicComicShot[];
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
  mode: EditorProjectMode;
  dynamicComic?: DynamicComicProject;
  assets: MediaAsset[];
  tracks: TimelineTrack[];
  mainTrackMagnetEnabled: boolean;
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
  outputPath?: string;
  manifestPath?: string;
  warnings?: string[];
}

export interface EditorSessionSeed {
  sourceNodeId?: string;
  assetName?: string;
  assetUrl?: string;
  duration?: number;
  mode?: EditorProjectMode;
}

export type MediaTaskKind = "import" | "frameSequence" | "export" | "proxy" | "thumbnail" | "waveform" | "projectSave" | "ttsSetup" | "speechSynthesis";

export type MediaTaskState =
  | "queued"
  | "running"
  | "cancelling"
  | "cancelled"
  | "succeeded"
  | "failed"
  | "interrupted";

export interface MediaTaskSnapshot<TResult = unknown> {
  id: string;
  kind: MediaTaskKind;
  state: MediaTaskState;
  projectId?: string;
  label: string;
  progress: number;
  status: string;
  createdAtMs: number;
  updatedAtMs: number;
  result?: TResult;
  error?: string;
}

export interface MediaTaskEvent<TResult = unknown> {
  task: MediaTaskSnapshot<TResult>;
}

export interface ImportedMediaFile {
  sourceFileName: string;
  managedPath: string;
  url: string;
  byteSize: number;
  fingerprint: string;
}

export type TtsVoice = "zhMale" | "zhFemale";
export type TtsEmotion = "natural" | "gentle" | "cheerful" | "serious";

export interface TtsStatus {
  supported: boolean;
  state: "notInstalled" | "installing" | "ready" | "error";
  runtimeInstalled: boolean;
  modelInstalled: boolean;
  requiredBytes: number;
  voices: Array<{ id: TtsVoice; label: string }>;
  error?: string;
}

export interface SpeechSynthesisRequest {
  projectId: string;
  text: string;
  voice: TtsVoice;
  emotion: TtsEmotion;
  speed: number;
}

export interface MediaMetadata {
  duration: number;
  width?: number;
  height?: number;
  hasVideo: boolean;
  hasAudio: boolean;
}

export interface MediaDerivatives {
  metadata: MediaMetadata;
  thumbnailPath?: string;
  proxyPath?: string;
  waveformPeaks?: number[];
  warnings: string[];
}

export interface StoryboardFrameRect {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface StoryboardToVideoRequest {
  projectId: string;
  sourcePath: string;
  sourceWidth: number;
  sourceHeight: number;
  frames: StoryboardFrameRect[];
  fps: number;
  outputName: string;
}

export interface StoryboardToVideoResult {
  managedPath: string;
  url: string;
  fileName: string;
  fingerprint: string;
  duration: number;
  width: number;
  height: number;
  frameCount: number;
  warnings: string[];
}
