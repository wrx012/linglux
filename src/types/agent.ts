import type { MediaAssetType, TimelineTrackType } from "./editor";

export type AgentPlanState = "pending" | "applied" | "rejected" | "stale";

export interface AgentChatMessage {
  id: string;
  role: "user" | "assistant";
  content: string;
  createdAt: string;
  plan?: AgentEditPlan;
  planState?: AgentPlanState;
}

export interface AgentConversation {
  schemaVersion: number;
  projectId: string;
  messages: AgentChatMessage[];
  updatedAt: string;
}

interface AgentOperationBase {
  id: string;
}

export interface AddAssetRange extends AgentOperationBase {
  type: "addAssetRange";
  assetId: string;
  trackId: string;
  sourceInMs: number;
  sourceOutMs: number;
  timelineStartMs?: number;
}

export interface KeepClipSourceRange extends AgentOperationBase {
  type: "keepClipSourceRange";
  clipId: string;
  sourceInMs: number;
  sourceOutMs: number;
}

export interface RemoveClipSourceRange extends AgentOperationBase {
  type: "removeClipSourceRange";
  clipId: string;
  sourceInMs: number;
  sourceOutMs: number;
}

export interface SplitClipAtTimeline extends AgentOperationBase {
  type: "splitClipAtTimeline";
  clipId: string;
  timelineTimeMs: number;
}

export interface DeleteClip extends AgentOperationBase {
  type: "deleteClip";
  clipId: string;
}

export interface MoveClip extends AgentOperationBase {
  type: "moveClip";
  clipId: string;
  trackId: string;
  timelineStartMs: number;
}

export type AgentEditOperation =
  | AddAssetRange
  | KeepClipSourceRange
  | RemoveClipSourceRange
  | SplitClipAtTimeline
  | DeleteClip
  | MoveClip;

export interface AgentEditPlan {
  id: string;
  projectId: string;
  baseEditorVersion: number;
  summary: string;
  operations: AgentEditOperation[];
  warnings: string[];
}

export interface AgentProjectAsset {
  id: string;
  name: string;
  type: MediaAssetType;
  durationMs: number;
}

export interface AgentProjectClip {
  id: string;
  assetId: string;
  name: string;
  type: TimelineTrackType;
  trackId: string;
  timelineStartMs: number;
  timelineDurationMs: number;
  sourceInMs: number;
  sourceOutMs: number;
  speed: number;
}

export interface AgentProjectTrack {
  id: string;
  label: string;
  type: TimelineTrackType;
  locked: boolean;
  clips: AgentProjectClip[];
}

export interface AgentProjectSnapshot {
  projectId: string;
  projectName: string;
  editorVersion: number;
  durationMs: number;
  playheadMs: number;
  selectedClipId?: string;
  selectedAssetIds: string[];
  mainTrackMagnetEnabled: boolean;
  assets: AgentProjectAsset[];
  tracks: AgentProjectTrack[];
  characterVoiceProfiles: Array<{
    id: string;
    name: string;
    color: string;
    voice: string;
    defaultEmotion: string;
    defaultSpeed: number;
  }>;
  dynamicComicShots: Array<{
    id: string;
    order: number;
    characterId?: string;
    emotion?: string;
    speechSpeed?: number;
  }>;
}

export interface AgentTurnRequest {
  project: AgentProjectSnapshot;
  prompt: string;
}

export interface AgentTurnResult {
  conversation: AgentConversation;
  plan?: AgentEditPlan;
  clarification?: string;
}

export type AgentTaskState = "queued" | "running" | "cancelling" | "cancelled" | "succeeded" | "failed";

export interface AgentTaskSnapshot {
  id: string;
  projectId: string;
  state: AgentTaskState;
  status: string;
  createdAtMs: number;
  updatedAtMs: number;
  result?: AgentTurnResult;
  error?: string;
}

export interface AgentTaskEvent {
  task: AgentTaskSnapshot;
}

export interface AgentProviderSettingsSummary {
  provider: string;
  baseUrl: string;
  model: string;
  hasApiKey: boolean;
  maskedApiKey: string;
}

export interface SaveAgentProviderSettingsRequest {
  provider: string;
  baseUrl: string;
  model: string;
  apiKey?: string;
}

export interface AgentPlanPreview {
  project: import("./editor").EditorProject;
  descriptions: string[];
  affectedTrackIds: string[];
  selectedClipId?: string;
}
