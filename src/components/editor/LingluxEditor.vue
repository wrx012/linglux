<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from "vue";
import { Channel, convertFileSrc, invoke, isTauri } from "@tauri-apps/api/core";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import {
  ArrowLeft,
  Download,
  Keyboard,
  MoveHorizontal,
  Play,
  Redo2,
  RotateCcw,
  Save,
  Scissors,
  Settings,
  SlidersHorizontal,
  Sparkles,
  Undo2,
  Video,
  X,
} from "@lucide/vue";
import AgentChatPanel from "./AgentChatPanel.vue";
import ExportDialog from "./ExportDialog.vue";
import InspectorPanel from "./InspectorPanel.vue";
import MediaBin from "./MediaBin.vue";
import PreviewMonitor from "./PreviewMonitor.vue";
import StoryboardToVideoDialog, { type StoryboardSource } from "./StoryboardToVideoDialog.vue";
import TimelinePanel from "./TimelinePanel.vue";
import lingluxLogo from "../../assets/linglux-logo-no-text.png";
import type {
  AgentConversation,
  AgentEditPlan,
  AgentTaskEvent,
  AgentTaskSnapshot,
  AgentTurnResult,
} from "../../types/agent";
import type {
  AudioBeatMarker,
  AudioTrackPreset,
  EditSession,
  EditorExportResult,
  EditorProject,
  ExportPreset,
  ImportedMediaFile,
  MediaAsset,
  MediaAssetType,
  MediaDerivatives,
  MediaTaskEvent,
  MediaTaskSnapshot,
  StoryboardToVideoRequest,
  StoryboardToVideoResult,
  SpeechSynthesisRequest,
  TextTemplatePreset,
  TimelineClip,
  TimelineTrack,
  TimelineTrackType,
  TtsEmotion,
  TtsStatus,
  TtsVoice,
} from "../../types/editor";
import {
  calculateProjectDuration,
  closeTimelineTrackGaps,
  cloneProject,
  createTimelineClip,
  exportPresets,
  formatTimecode,
  isPrimaryTimelineTrack,
  TIMELINE_BASE_PIXELS_PER_SECOND,
  TIMELINE_BUTTON_SCALE_STEP,
  TIMELINE_MAX_SCALE,
  TIMELINE_MIN_SCALE,
} from "../../lib/editorProject";
import {
  createAgentProjectSnapshot,
  previewAgentEditPlan,
} from "../../lib/editorAgent";
import { planVideoBeatMarkerAlignment } from "../../lib/beatMarkerAlignment";
import { createSpeechAssetName, validateTtsText } from "../../lib/tts";

type EditorShortcutAction = "togglePlayback" | "splitClip" | "scrollTimelineLeft" | "scrollTimelineRight";
type ShortcutModifier = "Alt" | "Control" | "Meta" | "Shift";
type ShortcutStatusTone = "info" | "success" | "warning";

interface EditorShortcutBinding {
  key: string;
  modifiers: ShortcutModifier[];
  primaryModifier?: boolean;
}

type EditorShortcutMap = Record<EditorShortcutAction, EditorShortcutBinding | null>;

interface ImportedMetadata {
  duration: number;
  width?: number;
  height?: number;
}

interface ImportedMediaElementMetadata extends ImportedMetadata {
  isReliable: boolean;
}

const EDITOR_SHORTCUT_STORAGE_KEY = "linglux-editor-shortcuts";
const SHORTCUT_MODIFIERS: ShortcutModifier[] = ["Control", "Alt", "Shift", "Meta"];
const SHORTCUT_ACTION_LABELS: Record<EditorShortcutAction, string> = {
  togglePlayback: "播放 / 暂停",
  splitClip: "分割片段",
  scrollTimelineLeft: "时间线向左滑动",
  scrollTimelineRight: "时间线向右滑动",
};
const DEFAULT_EDITOR_SHORTCUTS: EditorShortcutMap = {
  togglePlayback: { key: " ", modifiers: [] },
  splitClip: { key: "b", modifiers: [], primaryModifier: true },
  scrollTimelineLeft: { key: "ArrowLeft", modifiers: [], primaryModifier: true },
  scrollTimelineRight: { key: "ArrowRight", modifiers: [], primaryModifier: true },
};
const UNDO_EDITOR_SHORTCUT: EditorShortcutBinding = {
  key: "z",
  modifiers: [],
  primaryModifier: true,
};

const props = defineProps<{
  session: EditSession;
}>();

const emit = defineEmits<{
  returnToWorkflow: [];
  exported: [result: EditorExportResult];
  openSettings: [];
}>();

const project = ref(cloneProject(props.session.project));
const selectedClipId = ref(findFirstClipId());
const selectedAssetId = ref(selectedClipId.value ? "" : project.value.assets[0]?.id ?? "");
const selectedAssetIds = ref<string[]>(selectedAssetId.value ? [selectedAssetId.value] : []);
const libraryPreviewAssetId = ref("");
const playhead = ref(0);
const timelineScale = ref(1);
const snapEnabled = ref(true);
const clipTrimSnapGuideTime = ref<number>();
const isPlaying = ref(false);
const isPreviewVideoClockActive = ref(false);
const isPlayheadScrubbing = ref(false);
const isExportDialogOpen = ref(false);
const isInspectorOpen = ref(false);
const isAgentPanelOpen = ref(false);
const isAgentDrawerViewport = ref(false);
const isAgentRunning = ref(false);
const agentStatus = ref("");
const agentError = ref("");
const activeAgentTaskId = ref("");
const editorVersion = ref(Date.now());
const appliedAgentPlanIds = new Set<string>();
const agentConversation = ref<AgentConversation>(createEmptyAgentConversation(project.value.id));
const isShortcutMenuOpen = ref(false);
const editorShortcuts = ref<EditorShortcutMap>(loadEditorShortcuts());
const editingShortcutAction = ref<EditorShortcutAction | null>(null);
const pendingShortcutBinding = ref<EditorShortcutBinding | null>(null);
const pendingShortcutCode = ref("");
const isShortcutKeyHeld = ref(false);
const shortcutStatusMessage = ref("");
const shortcutStatusTone = ref<ShortcutStatusTone>("info");
const selectedPresetId = ref(exportPresets[0].id);
const isExporting = ref(false);
const exportError = ref("");
const exportProgress = ref(0);
const exportStatus = ref("");
const exportLocationError = ref("");
const completedExportResult = ref<EditorExportResult | null>(null);
const activeExportTaskId = ref("");
const importTaskStatus = ref("");
const activeImportTaskIds = ref<string[]>([]);
const ttsStatus = ref<TtsStatus>({
  supported: false,
  state: "notInstalled",
  runtimeInstalled: false,
  modelInstalled: false,
  requiredBytes: 3_400_000_000,
  voices: [],
  error: "仅桌面版 macOS Apple Silicon 支持。",
});
const isTtsBusy = ref(false);
const ttsProgress = ref(0);
const ttsTaskStatus = ref("");
const ttsError = ref("");
const activeTtsTaskId = ref("");
const isStoryboardDialogOpen = ref(false);
const storyboardSource = ref<StoryboardSource>();
const isStoryboardGenerating = ref(false);
const storyboardProgress = ref(0);
const storyboardStatus = ref("");
const storyboardError = ref("");
const activeStoryboardTaskId = ref("");
const saveState = ref("已保存");
const history = ref<ProjectHistoryEntry[]>([]);
const future = ref<ProjectHistoryEntry[]>([]);
const importedObjectUrls = new Set<string>();
const importedAssetFiles = new Map<string, File>();
const pendingManagedProxyAssetIds = new Set<string>();
const IMPORTED_IMAGE_DURATION_SECONDS = 4;
const IMPORTED_MEDIA_FALLBACK_DURATION_SECONDS = 12;
const IMPORTED_MEDIA_METADATA_TIMEOUT_MS = 2400;
const IMPORTED_BACKGROUND_METADATA_TIMEOUT_MS = 9000;
const WAVEFORM_PEAK_COUNT = 1024;
const AUTO_BEAT_MIN_GAP_SECONDS = 0.32;
const AUTO_BEAT_MAX_MARKERS = 128;
const HISTORY_MAX_ENTRIES = 128;
const HISTORY_MAX_ESTIMATED_BYTES = 8 * 1024 * 1024;
const HISTORY_COALESCE_WINDOW_MS = 650;
const MIN_TRIMMED_CLIP_DURATION_SECONDS = 0.5;
let activeClipTrim: {
  clipId: string;
  edge: "start" | "end";
  start: number;
  duration: number;
  trimStart: number;
  trimEnd: number;
  beatMarkers?: AudioBeatMarker[];
  changed: boolean;
} | undefined;
let playbackFrameId: number | undefined;
let playbackLastTimestamp: number | undefined;
let unlistenNativeDragDrop: (() => void) | undefined;
let pendingHistoryCapture: PendingProjectHistoryCapture | undefined;

interface ImportedAssetContext {
  asset: MediaAsset;
  file?: File;
}

interface HistoryEntityChange<T extends { id: string }> {
  id: string;
  before?: T;
  after?: T;
}

type ProjectHistorySettings = Pick<
  EditorProject,
  "name" | "sourceNodeId" | "mainTrackMagnetEnabled" | "fps" | "resolution"
>;
type TrackHistorySettings = Omit<TimelineTrack, "clips">;

interface TrackHistoryChange {
  id: string;
  beforeTrack?: TimelineTrack;
  afterTrack?: TimelineTrack;
  beforeSettings?: TrackHistorySettings;
  afterSettings?: TrackHistorySettings;
  clipChanges: HistoryEntityChange<TimelineClip>[];
  beforeClipOrder?: string[];
  afterClipOrder?: string[];
}

interface ProjectHistoryEntry {
  beforeSettings?: ProjectHistorySettings;
  afterSettings?: ProjectHistorySettings;
  assetChanges: HistoryEntityChange<MediaAsset>[];
  beforeAssetOrder?: string[];
  afterAssetOrder?: string[];
  trackChanges: TrackHistoryChange[];
  beforeTrackOrder?: string[];
  afterTrackOrder?: string[];
  estimatedBytes: number;
  createdAtMs: number;
  coalesceKey?: string;
}

interface PendingTrackHistoryCapture {
  wholeTrackCaptured: boolean;
  beforeTrack?: TimelineTrack;
  beforeSettings?: TrackHistorySettings;
  beforeClips: Map<string, TimelineClip | undefined>;
}

interface PendingProjectHistoryCapture {
  beforeSettings?: ProjectHistorySettings;
  beforeAssets: Map<string, MediaAsset | undefined>;
  beforeAssetOrder?: string[];
  tracks: Map<string, PendingTrackHistoryCapture>;
}

interface ProjectHistoryCaptureSpec {
  settings?: boolean;
  assetIds?: string[];
  captureAssetOrder?: boolean;
  wholeTrackIds?: string[];
  trackSettingsIds?: string[];
  clips?: Array<{ trackId: string; clipIds: string[] }>;
}

interface AssetPointerDragStateBase {
  pointerId: number;
  startX: number;
  startY: number;
  currentX: number;
  currentY: number;
  isDragging: boolean;
  targetTrackId?: string;
}

interface TimelineDropPreview {
  trackId: string;
  start: number;
  duration: number;
  clipType: TimelineTrackType;
  isCompatible: boolean;
  isSnapped: boolean;
  snapGuideTime?: number;
  label: string;
}

type AssetPointerDragState =
  | (AssetPointerDragStateBase & {
      kind: "asset";
      assetId: string;
    })
  | (AssetPointerDragStateBase & {
      kind: "clip";
      clipId: string;
      grabOffsetSeconds: number;
    })
  | (AssetPointerDragStateBase & {
      kind: "textTemplate";
      preset: TextTemplatePreset;
    });

const assetPointerDragState = ref<AssetPointerDragState | null>(null);
const isNarrowViewport = ref(false);
const selectedClip = computed(() => findClip(selectedClipId.value));
const selectedAudioClip = computed(() => (selectedClip.value?.type === "audio" ? selectedClip.value : undefined));
const libraryPreviewAsset = computed(() => project.value.assets.find((asset) => asset.id === libraryPreviewAssetId.value));
const timelineClips = computed(() => project.value.tracks.flatMap((track) => track.clips));
const isTimelineEmpty = computed(() => timelineClips.value.length === 0);
const timelineAssetIds = computed(() => [...new Set(timelineClips.value.map((clip) => clip.assetId))]);
const selectedClipIsText = computed(() => selectedClip.value?.type === "caption");
const isInspectorDrawerVisible = computed(() => isInspectorOpen.value && (!selectedClipIsText.value || isNarrowViewport.value));
const isAgentPanelDocked = computed(() => isAgentPanelOpen.value && !isAgentDrawerViewport.value);
const selectedPreset = computed(() => exportPresets.find((preset) => preset.id === selectedPresetId.value) ?? exportPresets[0]);
const completedExportOutputPath = computed(() => completedExportResult.value?.outputPath ?? "");
const canUndo = computed(() => history.value.length > 0);
const canRedo = computed(() => future.value.length > 0);
const draggedTimelineClip = computed(() => {
  const drag = assetPointerDragState.value;
  return drag?.kind === "clip" ? findClip(drag.clipId) : undefined;
});
const draggedAsset = computed(() => {
  const drag = assetPointerDragState.value;

  if (!drag) {
    return undefined;
  }

  if (drag.kind === "asset") {
    return project.value.assets.find((asset) => asset.id === drag.assetId);
  }

  if (drag.kind === "clip") {
    const clip = findClip(drag.clipId);
    return clip ? project.value.assets.find((asset) => asset.id === clip.assetId) : undefined;
  }

  return findTextTemplateAsset(drag.preset) ?? createTextTemplateAsset(drag.preset, "");
});
const isAssetDragActive = computed(() => Boolean(assetPointerDragState.value?.isDragging));
const isFirstVisualAssetDrag = computed(() => {
  const drag = assetPointerDragState.value;
  const asset = draggedAsset.value;

  return Boolean(
    isTimelineEmpty.value &&
      drag?.kind === "asset" &&
      asset &&
      (asset.type === "video" || asset.type === "image"),
  );
});
const primaryTimelineTrackId = computed(() => project.value.tracks.find((track) => isPrimaryTimelineTrack(track))?.id);
const audioTimelineTrackId = computed(() => project.value.tracks.find((track) => track.type === "audio")?.id);
const isFirstAudioAssetDrag = computed(() => {
  const drag = assetPointerDragState.value;
  const asset = draggedAsset.value;

  return Boolean(isTimelineEmpty.value && drag?.kind === "asset" && asset?.type === "audio");
});
const assetDragTargetTrackId = computed(() => {
  const targetTrackId = assetPointerDragState.value?.targetTrackId;

  if (isFirstVisualAssetDrag.value && targetTrackId) {
    return primaryTimelineTrackId.value;
  }

  if (isFirstAudioAssetDrag.value && targetTrackId) {
    return audioTimelineTrackId.value;
  }

  return targetTrackId;
});
const assetDragCompatibleTrackIds = computed(() => {
  const asset = draggedAsset.value;

  if (!asset || !isAssetDragActive.value) {
    return [];
  }

  if (isFirstVisualAssetDrag.value) {
    return primaryTimelineTrackId.value ? [primaryTimelineTrackId.value] : [];
  }

  return project.value.tracks
    .filter((track) => !track.locked && isTrackCompatibleWithAsset(asset, track))
    .map((track) => track.id);
});
const timelineDragLayoutRevision = ref(0);
const assetDragPreview = computed<TimelineDropPreview | undefined>(() => {
  timelineDragLayoutRevision.value;

  const drag = assetPointerDragState.value;
  const asset = draggedAsset.value;

  if (!drag?.isDragging || !asset) {
    return undefined;
  }

  const pointerPlacement = getTimelinePlacementFromPoint(drag.currentX, drag.currentY);

  if (!pointerPlacement) {
    return undefined;
  }

  const trackId =
    isFirstVisualAssetDrag.value && primaryTimelineTrackId.value
      ? primaryTimelineTrackId.value
      : isFirstAudioAssetDrag.value && audioTimelineTrackId.value
        ? audioTimelineTrackId.value
        : pointerPlacement.trackId;
  const targetTrack = project.value.tracks.find((track) => track.id === trackId);

  if (!targetTrack) {
    return undefined;
  }

  const isCompatible = isTrackCompatibleWithAsset(asset, targetTrack) && !targetTrack.locked;
  const draggedClip = drag.kind === "clip" ? findClip(drag.clipId) : undefined;
  const rawStart = isFirstVisualAssetDrag.value || isFirstAudioAssetDrag.value
    ? 0
    : pointerPlacement.start - (drag.kind === "clip" ? drag.grabOffsetSeconds : 0);
  const resolvedPlacement = draggedClip
    ? resolveClipPlacementStart(draggedClip.id, trackId, rawStart)
    : resolveAssetPlacementStart(rawStart);

  return {
    trackId,
    start: resolvedPlacement.start,
    duration: draggedClip?.duration ?? Math.max(asset.duration ?? 4, 0.5),
    clipType: draggedClip?.type ?? (isPrimaryTimelineTrack(targetTrack) && (asset.type === "video" || asset.type === "image")
      ? "video"
      : clipTypeForAsset(asset)),
    isCompatible,
    isSnapped: resolvedPlacement.isSnapped,
    snapGuideTime: resolvedPlacement.snapGuideTime,
    label: draggedClip?.name ?? asset.name,
  };
});
const assetDragGhostStyle = computed(() => ({
  transform: `translate3d(${(assetPointerDragState.value?.currentX ?? 0) + 14}px, ${(assetPointerDragState.value?.currentY ?? 0) + 14}px, 0)`,
}));
const clipDragGhostStyle = computed(() => {
  const drag = assetPointerDragState.value;
  const clip = draggedTimelineClip.value;

  if (!drag || drag.kind !== "clip" || !clip) {
    return {};
  }

  const scaledWidth = clip.duration * TIMELINE_BASE_PIXELS_PER_SECOND * timelineScale.value;
  const width = clamp(scaledWidth, 44, 220);
  const grabRatio = clip.duration > 0 ? clamp(drag.grabOffsetSeconds / clip.duration, 0, 1) : 0.5;
  const height = clip.type === "video" ? 58 : 38;

  return {
    width: `${width}px`,
    height: `${height}px`,
    transform: `translate3d(${drag.currentX - width * grabRatio}px, ${drag.currentY - height / 2}px, 0)`,
  };
});
const clipDragGhostThumbnailStyle = computed(() => {
  const asset = draggedAsset.value;
  const thumbnailUrl = asset?.thumbnailUrl ?? (asset?.type === "image" ? asset.url : "");

  return thumbnailUrl
    ? {
        backgroundImage: `url("${thumbnailUrl}")`,
        backgroundPosition: "center",
        backgroundSize: "cover",
      }
    : {};
});
const clipDragStatus = computed(() => {
  if (!assetDragPreview.value) {
    return "拖到兼容轨道";
  }

  if (!assetDragPreview.value.isCompatible) {
    return "此轨道不可放置";
  }

  return `${formatTimecode(assetDragPreview.value.start)} · 松开移动`;
});
const previewWorkspaceLayoutClass = computed(() =>
  isAgentPanelDocked.value
    ? "grid-cols-[minmax(280px,320px)_minmax(0,1fr)_minmax(320px,360px)] max-[1380px]:grid-cols-[280px_minmax(0,1fr)_320px] max-[900px]:grid-cols-1 max-[900px]:grid-rows-[260px_420px]"
    : selectedClipIsText.value
    ? "grid-cols-[minmax(300px,340px)_minmax(0,1fr)_minmax(300px,340px)] max-[1380px]:grid-cols-[minmax(280px,320px)_minmax(0,1fr)_minmax(280px,320px)] max-[900px]:grid-cols-1 max-[900px]:grid-rows-[260px_420px]"
    : "grid-cols-[minmax(300px,340px)_minmax(0,1fr)] max-[1220px]:grid-cols-[minmax(280px,320px)_minmax(0,1fr)] max-[900px]:grid-cols-1 max-[900px]:grid-rows-[260px_420px]",
);

let viewportQuery: MediaQueryList | undefined;
let agentDrawerViewportQuery: MediaQueryList | undefined;
let agentTurnGeneration = 0;

watch(
  () => props.session,
  (session) => {
    agentTurnGeneration += 1;
    const previousAgentTaskId = activeAgentTaskId.value;
    if (previousAgentTaskId && isTauri()) {
      void invoke("cancel_editor_agent_turn", { taskId: previousAgentTaskId }).catch(() => false);
    }
    stopPlaybackLoop();
    isPlaying.value = false;
    isPreviewVideoClockActive.value = false;
    isPlayheadScrubbing.value = false;
    resetExportProgress();
    exportError.value = "";
    isExporting.value = false;
    isExportDialogOpen.value = false;
    cancelAssetPointerDrag();
    cleanupImportedObjectUrls();
    project.value = cloneProject(session.project);
    selectedClipId.value = findFirstClipId();
    if (selectedClipId.value) {
      clearSelectedAssets();
    } else {
      setSingleSelectedAsset(project.value.assets[0]?.id ?? "");
    }
    libraryPreviewAssetId.value = "";
    playhead.value = 0;
    history.value = [];
    future.value = [];
    pendingHistoryCapture = undefined;
    editorVersion.value = Date.now();
    appliedAgentPlanIds.clear();
    agentConversation.value = createEmptyAgentConversation(project.value.id);
    agentError.value = "";
    agentStatus.value = "";
    activeAgentTaskId.value = "";
    isAgentRunning.value = false;
    saveState.value = session.isDirty ? "未保存" : "已保存";
    isInspectorOpen.value = false;
    void loadAgentConversation();
  },
  { deep: true },
);

onMounted(() => {
  viewportQuery = window.matchMedia("(max-width: 900px)");
  isNarrowViewport.value = viewportQuery.matches;
  viewportQuery.addEventListener("change", handleViewportQueryChange);
  agentDrawerViewportQuery = window.matchMedia("(max-width: 1280px)");
  isAgentDrawerViewport.value = agentDrawerViewportQuery.matches;
  agentDrawerViewportQuery.addEventListener("change", handleAgentDrawerViewportQueryChange);
  window.addEventListener("keydown", handleEditorKeydown);
  window.addEventListener("keyup", handleEditorKeyup);

  if (isTauri()) {
    void loadTtsStatus();
    void getCurrentWebview()
      .onDragDropEvent((event) => {
        if (event.payload.type === "drop" && event.payload.paths.length > 0) {
          void importMediaPaths(event.payload.paths);
        }
      })
      .then((unlisten) => {
        unlistenNativeDragDrop = unlisten;
      });
  }

  if (selectedClip.value) {
    ensureManagedVideoProxy(selectedClip.value.assetId);
  }

  void loadAgentConversation();
});

onUnmounted(() => {
  agentTurnGeneration += 1;
  if (activeAgentTaskId.value && isTauri()) {
    void invoke("cancel_editor_agent_turn", { taskId: activeAgentTaskId.value }).catch(() => false);
  }
  stopPlaybackLoop();
  cancelAssetPointerDrag();
  cleanupImportedObjectUrls();
  unlistenNativeDragDrop?.();
  unlistenNativeDragDrop = undefined;
  viewportQuery?.removeEventListener("change", handleViewportQueryChange);
  agentDrawerViewportQuery?.removeEventListener("change", handleAgentDrawerViewportQueryChange);
  window.removeEventListener("keydown", handleEditorKeydown);
  window.removeEventListener("keyup", handleEditorKeyup);
});

function findFirstClipId() {
  return props.session.project.tracks.flatMap((track) => track.clips)[0]?.id ?? "";
}

function findClip(clipId?: string) {
  if (!clipId) {
    return undefined;
  }

  return project.value.tracks.flatMap((track) => track.clips).find((clip) => clip.id === clipId);
}

function selectClip(clipId: string) {
  const clip = findClip(clipId);

  libraryPreviewAssetId.value = "";
  selectedClipId.value = clipId;
  clearSelectedAssets();

  if (clip) {
    playhead.value = clip.start;
    maybeOpenTextInspector(clip);
    ensureManagedVideoProxy(clip.assetId);
  }
}

function selectPreviewCaptionClip(clipId: string) {
  const clip = findClip(clipId);

  libraryPreviewAssetId.value = "";
  selectedClipId.value = clipId;
  clearSelectedAssets();

  if (clip) {
    maybeOpenTextInspector(clip);
  }
}

function maybeOpenTextInspector(clip: TimelineClip) {
  if (clip.type === "caption" && isNarrowViewport.value) {
    isInspectorOpen.value = true;
  }
}

function handleViewportQueryChange(event: MediaQueryListEvent) {
  isNarrowViewport.value = event.matches;

  if (event.matches && selectedClip.value?.type === "caption") {
    isInspectorOpen.value = true;
  }
}

function handleAgentDrawerViewportQueryChange(event: MediaQueryListEvent) {
  isAgentDrawerViewport.value = event.matches;
}

function setSingleSelectedAsset(assetId: string) {
  setSelectedAssets(assetId ? [assetId] : []);
}

function clearSelectedAssets() {
  setSelectedAssets([]);
}

function setSelectedAssets(assetIds: string[]) {
  const existingAssetIds = new Set(project.value.assets.map((asset) => asset.id));
  const seenAssetIds = new Set<string>();
  const nextAssetIds: string[] = [];

  for (const assetId of assetIds) {
    if (!existingAssetIds.has(assetId) || seenAssetIds.has(assetId)) {
      continue;
    }

    seenAssetIds.add(assetId);
    nextAssetIds.push(assetId);
  }

  selectedAssetIds.value = nextAssetIds;
  selectedAssetId.value = nextAssetIds[0] ?? "";
}

function selectAsset(assetId: string) {
  const asset = project.value.assets.find((item) => item.id === assetId);

  selectedClipId.value = "";
  setSingleSelectedAsset(assetId);

  if (asset?.type !== "video" || isTimelineEmpty.value) {
    if (libraryPreviewAssetId.value) {
      libraryPreviewAssetId.value = "";
    }

    isPlaying.value = false;
    stopPlaybackLoop();

    return;
  }

  ensureManagedVideoProxy(asset.id);
  libraryPreviewAssetId.value = asset.id;
  isPlaying.value = true;
  stopPlaybackLoop();
}

function selectAssets(assetIds: string[]) {
  selectedClipId.value = "";
  setSelectedAssets(assetIds);
  libraryPreviewAssetId.value = "";
  isPlaying.value = false;
  stopPlaybackLoop();
}

function deleteAsset(assetId: string) {
  deleteAssets([assetId]);
}

function deleteAssets(assetIds: string[]) {
  const requestedAssetIds = new Set(assetIds);
  const deletedAssetIds = new Set(
    project.value.assets
      .filter((asset) => requestedAssetIds.has(asset.id) && (asset.type === "video" || asset.type === "audio"))
      .map((asset) => asset.id),
  );

  if (deletedAssetIds.size === 0) {
    return;
  }

  const removedSelectedClip = project.value.tracks.some((track) =>
    track.clips.some((clip) => clip.id === selectedClipId.value && deletedAssetIds.has(clip.assetId)),
  );
  const affectedTrackIds = project.value.tracks
    .filter((track) => track.clips.some((clip) => deletedAssetIds.has(clip.assetId)))
    .map((track) => track.id);

  pushHistory({
    assetIds: [...deletedAssetIds],
    captureAssetOrder: true,
    wholeTrackIds: affectedTrackIds,
  });
  project.value.assets = project.value.assets.filter((item) => !deletedAssetIds.has(item.id));
  for (const assetId of deletedAssetIds) {
    importedAssetFiles.delete(assetId);
  }

  if (libraryPreviewAssetId.value && deletedAssetIds.has(libraryPreviewAssetId.value)) {
    libraryPreviewAssetId.value = "";
    isPlaying.value = false;
    stopPlaybackLoop();
  }

  for (const track of project.value.tracks) {
    track.clips = track.clips.filter((clip) => !deletedAssetIds.has(clip.assetId));

    if (project.value.mainTrackMagnetEnabled && isPrimaryTimelineTrack(track)) {
      closeTimelineTrackGaps(track);
    }
  }

  if (removedSelectedClip) {
    selectedClipId.value = project.value.tracks.flatMap((track) => track.clips)[0]?.id ?? "";
  }

  const clip = findClip(selectedClipId.value);
  const remainingSelectedAssetIds = selectedAssetIds.value.filter((id) =>
    !deletedAssetIds.has(id) && project.value.assets.some((asset) => asset.id === id),
  );

  if (remainingSelectedAssetIds.length > 0) {
    setSelectedAssets(remainingSelectedAssetIds);
  } else if (clip) {
    clearSelectedAssets();
  } else {
    setSingleSelectedAsset(project.value.assets[0]?.id ?? "");
  }

  if (clip) {
    playhead.value = clip.start;
  } else {
    playhead.value = 0;
  }

  markDirty();
  playhead.value = clamp(playhead.value, 0, project.value.duration);
}

function createEmptyAgentConversation(projectId: string): AgentConversation {
  return {
    schemaVersion: 1,
    projectId,
    messages: [],
    updatedAt: new Date().toISOString(),
  };
}

async function loadAgentConversation() {
  if (!isTauri()) {
    agentConversation.value = createEmptyAgentConversation(project.value.id);
    return;
  }

  try {
    agentConversation.value = await invoke<AgentConversation>("load_agent_conversation", {
      projectId: project.value.id,
    });
  } catch (error) {
    agentError.value = normalizeExportError(error);
  }
}

function openAgentPanel() {
  isAgentPanelOpen.value = true;
  agentError.value = "";
}

function closeAgentPanel() {
  isAgentPanelOpen.value = false;
}

function runAgentTurn(prompt: string): Promise<AgentTurnResult> {
  return new Promise((resolve, reject) => {
    let settled = false;
    const onEvent = new Channel<AgentTaskEvent>();

    onEvent.onmessage = ({ task }) => {
      agentStatus.value = task.status;
      activeAgentTaskId.value = task.id;

      if (settled) {
        return;
      }

      if (task.state === "succeeded") {
        settled = true;

        if (!task.result) {
          reject(new Error("Agent 已完成，但没有返回结果。"));
        } else {
          resolve(task.result);
        }
      } else if (task.state === "failed") {
        settled = true;
        reject(new Error(task.error || "Agent 请求失败。"));
      } else if (task.state === "cancelled") {
        settled = true;
        reject(new Error("Agent 请求已取消。"));
      }
    };

    const request = {
      prompt,
      project: createAgentProjectSnapshot({
        project: project.value,
        editorVersion: editorVersion.value,
        playhead: playhead.value,
        selectedClipId: selectedClipId.value,
        selectedAssetIds: selectedAssetIds.value,
      }),
    };

    void invoke<AgentTaskSnapshot>("start_editor_agent_turn", {
      request,
      onEvent,
    })
      .then((task) => {
        activeAgentTaskId.value = task.id;
        agentStatus.value = task.status;
      })
      .catch((error) => {
        if (!settled) {
          settled = true;
          reject(error);
        }
      });
  });
}

async function sendAgentPrompt(prompt: string) {
  if (isAgentRunning.value || !isTauri()) {
    return;
  }

  const turnGeneration = ++agentTurnGeneration;
  const turnProjectId = project.value.id;
  isAgentPanelOpen.value = true;
  isAgentRunning.value = true;
  agentError.value = "";
  agentStatus.value = "正在提交剪辑指令";
  agentConversation.value.messages.push({
    id: `local-user-${Date.now()}`,
    role: "user",
    content: prompt,
    createdAt: new Date().toISOString(),
  });

  try {
    const result = await runAgentTurn(prompt);

    if (turnGeneration !== agentTurnGeneration || project.value.id !== turnProjectId) {
      return;
    }

    agentConversation.value = result.conversation;

    if (result.plan) {
      previewAgentEditPlan(project.value, result.plan, editorVersion.value);
    }
  } catch (error) {
    if (turnGeneration !== agentTurnGeneration || project.value.id !== turnProjectId) {
      return;
    }

    agentError.value = normalizeExportError(error);
    await loadAgentConversation();
  } finally {
    if (turnGeneration === agentTurnGeneration && project.value.id === turnProjectId) {
      isAgentRunning.value = false;
      agentStatus.value = "";
      activeAgentTaskId.value = "";
    }
  }
}

async function cancelAgentTurn() {
  if (!activeAgentTaskId.value) {
    return;
  }

  agentStatus.value = "正在取消 Agent 请求";
  await invoke<boolean>("cancel_editor_agent_turn", {
    taskId: activeAgentTaskId.value,
  }).catch(() => false);
}

async function setAgentPlanState(plan: AgentEditPlan, state: "applied" | "rejected" | "stale") {
  const message = agentConversation.value.messages.find((item) => item.plan?.id === plan.id);
  if (message) {
    message.planState = state;
  }

  if (!isTauri()) {
    return;
  }

  try {
    agentConversation.value = await invoke<AgentConversation>("update_agent_plan_state", {
      projectId: project.value.id,
      planId: plan.id,
      state,
    });
  } catch (error) {
    agentError.value = normalizeExportError(error);
  }
}

function applyAgentPlan(plan: AgentEditPlan) {
  if (appliedAgentPlanIds.has(plan.id)) {
    agentError.value = "该计划已经应用，不能重复执行。";
    return;
  }

  let preview;

  try {
    preview = previewAgentEditPlan(project.value, plan, editorVersion.value);
  } catch (error) {
    agentError.value = normalizeExportError(error);
    void setAgentPlanState(plan, "stale");
    return;
  }

  pushHistory({ wholeTrackIds: preview.affectedTrackIds });
  project.value = preview.project;
  appliedAgentPlanIds.add(plan.id);
  libraryPreviewAssetId.value = "";
  clearSelectedAssets();
  selectedClipId.value = preview.selectedClipId ?? project.value.tracks.flatMap((track) => track.clips)[0]?.id ?? "";
  const clip = findClip(selectedClipId.value);
  playhead.value = clip?.start ?? clamp(playhead.value, 0, project.value.duration);
  markDirty();
  agentError.value = "";
  void setAgentPlanState(plan, "applied");
}

function rejectAgentPlan(plan: AgentEditPlan) {
  void setAgentPlanState(plan, "rejected");
}

async function clearAgentConversation() {
  if (!isTauri()) {
    agentConversation.value = createEmptyAgentConversation(project.value.id);
    return;
  }

  try {
    agentConversation.value = await invoke<AgentConversation>("clear_agent_conversation", {
      projectId: project.value.id,
    });
    agentError.value = "";
  } catch (error) {
    agentError.value = normalizeExportError(error);
  }
}

function runMediaTask<TResult>(
  command: "start_import_media" | "start_storyboard_to_video" | "start_media_derivatives" | "start_export" | "start_tts_setup" | "start_speech_synthesis",
  args: Record<string, unknown>,
  onUpdate?: (task: MediaTaskSnapshot<TResult>) => void,
): Promise<TResult> {
  return new Promise((resolve, reject) => {
    let settled = false;
    let receivedEvent = false;
    const onEvent = new Channel<MediaTaskEvent<TResult>>();

    onEvent.onmessage = ({ task }) => {
      receivedEvent = true;
      onUpdate?.(task);

      if (settled) {
        return;
      }

      if (task.state === "succeeded") {
        settled = true;

        if (task.result === undefined) {
          reject(new Error("媒体任务完成，但没有返回结果。"));
        } else {
          resolve(task.result);
        }
      } else if (task.state === "failed" || task.state === "interrupted") {
        settled = true;
        reject(new Error(task.error || "媒体任务失败。"));
      } else if (task.state === "cancelled") {
        settled = true;
        reject(new Error("任务已取消。"));
      }
    };

    void invoke<MediaTaskSnapshot<TResult>>(command, {
      ...args,
      onEvent,
    })
      .then((task) => {
        if (!receivedEvent) {
          onUpdate?.(task);
        }
      })
      .catch((error) => {
        if (!settled) {
          settled = true;
          reject(error);
        }
      });
  });
}

async function loadTtsStatus() {
  if (!isTauri()) return;
  try {
    ttsStatus.value = await invoke<TtsStatus>("get_tts_status");
  } catch (error) {
    ttsStatus.value = { ...ttsStatus.value, supported: true, state: "error", error: normalizeExportError(error) };
  }
}

async function setupTts() {
  if (!isTauri() || isTtsBusy.value) return;
  isTtsBusy.value = true;
  ttsError.value = "";
  ttsProgress.value = 0;
  ttsTaskStatus.value = "准备安装本地语音模型";
  try {
    ttsStatus.value = await runMediaTask<TtsStatus>("start_tts_setup", {}, updateTtsTask);
    ttsTaskStatus.value = "本地语音模型已就绪";
  } catch (error) {
    ttsError.value = normalizeExportError(error);
    await loadTtsStatus();
  } finally {
    isTtsBusy.value = false;
    activeTtsTaskId.value = "";
  }
}

async function generateSpeech(request: { text: string; voice: TtsVoice; emotion: TtsEmotion; speed: number }) {
  const validationError = validateTtsText(request.text);
  if (!isTauri() || isTtsBusy.value || validationError) {
    ttsError.value = validationError;
    return;
  }
  isTtsBusy.value = true;
  ttsError.value = "";
  ttsProgress.value = 0;
  ttsTaskStatus.value = "准备生成配音";
  try {
    const speechRequest: SpeechSynthesisRequest = { projectId: project.value.id, ...request };
    const imported = await runMediaTask<ImportedMediaFile>("start_speech_synthesis", { request: speechRequest }, updateTtsTask);
    addGeneratedSpeechToProject(imported, request.text);
    ttsTaskStatus.value = "配音已添加到时间线";
  } catch (error) {
    ttsError.value = normalizeExportError(error);
  } finally {
    isTtsBusy.value = false;
    activeTtsTaskId.value = "";
  }
}

function updateTtsTask(task: MediaTaskSnapshot<unknown>) {
  activeTtsTaskId.value = task.state === "queued" || task.state === "running" || task.state === "cancelling" ? task.id : "";
  ttsProgress.value = task.progress;
  ttsTaskStatus.value = `${task.status}${task.progress > 0 ? ` ${Math.round(task.progress)}%` : ""}`;
}

async function cancelTts() {
  if (!activeTtsTaskId.value) return;
  ttsTaskStatus.value = "正在取消";
  try {
    await invoke("cancel_media_task", { taskId: activeTtsTaskId.value });
  } catch (error) {
    ttsError.value = normalizeExportError(error);
  }
}

function addGeneratedSpeechToProject(imported: ImportedMediaFile, text: string) {
  const context = createManagedImportedAsset(imported, 0);
  if (!context) throw new Error("生成的配音格式不受支持。");
  const asset = context.asset;
  asset.name = createSpeechAssetName(text, project.value.assets.map((item) => item.name));
  let track = project.value.tracks.find((item) => item.type === "audio");
  const trackId = track?.id ?? `track-audio-${Date.now()}`;
  pushHistory({ assetIds: [asset.id], captureAssetOrder: true, wholeTrackIds: [trackId] });
  if (!track) {
    track = { id: trackId, type: "audio", label: "音频轨", muted: false, visible: true, mediaEnabled: true, locked: false, clips: [] };
    project.value.tracks.push(track);
  }
  project.value.assets.push(asset);
  const start = clamp(playhead.value, 0, Math.max(project.value.duration, 0));
  const clip = createTimelineClip({
    id: `clip-${asset.id}-${Date.now()}`,
    assetId: asset.id,
    trackId: track.id,
    name: asset.name,
    type: "audio",
    start,
    duration: asset.duration,
    volume: 0.82,
  });
  track.clips.push(clip);
  track.clips.sort((left, right) => left.start - right.start);
  selectedClipId.value = clip.id;
  clearSelectedAssets();
  libraryPreviewAssetId.value = "";
  markDirty();
  void hydrateManagedMediaDerivative(asset, { generateWaveform: true, generateProxy: false }).catch((error) => {
    ttsError.value = `配音已添加，但波形生成失败：${normalizeExportError(error)}`;
  });
}

async function prepareStoryboardConversion(path: string) {
  if (!isTauri() || !path) return;
  storyboardError.value = "";
  storyboardStatus.value = "正在把分镜图导入当前工程";
  try {
    const imported = await runMediaTask<ImportedMediaFile[]>(
      "start_import_media",
      { projectId: project.value.id, paths: [path] },
      (task) => {
        trackActiveImportTask(task);
        storyboardStatus.value = `${task.status} ${Math.round(task.progress)}%`;
      },
    );
    const source = imported[0];
    if (!source) throw new Error("没有找到可转换的分镜图片。");
    storyboardSource.value = { name: source.sourceFileName, managedPath: source.managedPath, url: convertFileSrc(source.managedPath) };
    storyboardProgress.value = 0;
    storyboardStatus.value = "自动识别画格";
    isStoryboardDialogOpen.value = true;
  } catch (error) {
    shortcutStatusTone.value = "warning";
    shortcutStatusMessage.value = normalizeExportError(error);
  }
}

async function generateStoryboardVideo(request: StoryboardToVideoRequest) {
  if (!isTauri() || isStoryboardGenerating.value) return;
  isStoryboardGenerating.value = true;
  storyboardProgress.value = 0;
  storyboardError.value = "";
  storyboardStatus.value = "准备生成分镜视频";
  try {
    const result = await runMediaTask<StoryboardToVideoResult>(
      "start_storyboard_to_video",
      { request },
      (task) => {
        activeStoryboardTaskId.value = ["cancelled", "succeeded", "failed", "interrupted"].includes(task.state) ? "" : task.id;
        storyboardProgress.value = task.progress;
        storyboardStatus.value = task.status;
      },
    );
    const asset: MediaAsset = {
      id: `asset-storyboard-${Date.now()}-${Math.round(Math.random() * 100000)}`,
      type: "video",
      name: result.fileName,
      url: convertFileSrc(result.managedPath),
      filePath: result.managedPath,
      contentFingerprint: result.fingerprint,
      duration: result.duration,
      width: result.width,
      height: result.height,
      createdAt: new Date().toISOString(),
    };
    pushHistory({ assetIds: [asset.id], captureAssetOrder: true });
    project.value.assets.push(asset);
    setSingleSelectedAsset(asset.id);
    markDirty();
    void hydrateManagedMediaDerivatives([{ asset }]);
    shortcutStatusTone.value = "success";
    shortcutStatusMessage.value = `已生成 ${result.frameCount} 帧视频并加入素材库`;
    isStoryboardDialogOpen.value = false;
    storyboardSource.value = undefined;
  } catch (error) {
    storyboardError.value = normalizeExportError(error);
  } finally {
    isStoryboardGenerating.value = false;
    activeStoryboardTaskId.value = "";
  }
}

async function cancelStoryboardGeneration() {
  if (!activeStoryboardTaskId.value) return;
  storyboardStatus.value = "正在取消";
  await invoke("cancel_media_task", { taskId: activeStoryboardTaskId.value }).catch(() => undefined);
}

async function importMediaFiles(files: File[]) {
  const imports = (await Promise.all(files.map((file, index) => createImportedAsset(file, index)))).filter(
    (item): item is ImportedAssetContext & { file: File } => item !== undefined,
  );
  const supportedAssets = imports.map((item) => item.asset);

  if (supportedAssets.length === 0) {
    return;
  }

  pushHistory({
    assetIds: supportedAssets.map((asset) => asset.id),
    captureAssetOrder: true,
  });
  project.value.assets.push(...supportedAssets);
  setSingleSelectedAsset(supportedAssets[0].id);
  markDirty();

  void hydrateImportedMediaMetadata(imports);
  void hydrateImportedVideoThumbnails(imports);
  void hydrateImportedWaveforms(imports);
}

async function importMediaPaths(paths: string[]) {
  if (!isTauri() || paths.length === 0) {
    return;
  }

  importTaskStatus.value = "准备导入";

  try {
    const imported = await runMediaTask<ImportedMediaFile[]>(
      "start_import_media",
      {
        projectId: project.value.id,
        paths,
      },
      (task) => {
        trackActiveImportTask(task);
        importTaskStatus.value = `${task.status} ${Math.round(task.progress)}%`;
      },
    );
    const imports = imported
      .map((item, index) => createManagedImportedAsset(item, index))
      .filter((item): item is ImportedAssetContext => item !== undefined);
    const supportedAssets = imports.map((item) => item.asset);

    if (supportedAssets.length === 0) {
      return;
    }

    pushHistory({
      assetIds: supportedAssets.map((asset) => asset.id),
      captureAssetOrder: true,
    });
    project.value.assets.push(...supportedAssets);
    setSingleSelectedAsset(supportedAssets[0].id);
    markDirty();
    void hydrateManagedMediaDerivatives(imports);
  } catch (error) {
    shortcutStatusTone.value = "warning";
    shortcutStatusMessage.value = normalizeExportError(error);
  } finally {
    if (activeImportTaskIds.value.length === 0) {
      importTaskStatus.value = "";
    }
  }
}

async function cancelImport() {
  const taskIds = [...activeImportTaskIds.value];

  if (taskIds.length === 0) {
    return;
  }

  importTaskStatus.value = "正在取消媒体任务";
  await Promise.all(taskIds.map((taskId) => invoke("cancel_media_task", { taskId }).catch(() => undefined)));
}

function trackActiveImportTask(task: MediaTaskSnapshot<unknown>) {
  const isTerminal = ["cancelled", "succeeded", "failed", "interrupted"].includes(task.state);

  if (isTerminal) {
    activeImportTaskIds.value = activeImportTaskIds.value.filter((taskId) => taskId !== task.id);
    return;
  }

  if (!activeImportTaskIds.value.includes(task.id)) {
    activeImportTaskIds.value = [...activeImportTaskIds.value, task.id];
  }
}

function addAssetToTimeline(assetId: string, placement?: { trackId?: string; start?: number }, options: { recordHistory?: boolean } = {}) {
  const asset = project.value.assets.find((item) => item.id === assetId);

  if (!asset) {
    return;
  }

  const shouldPlaceFirstVisualAssetOnMainTrack =
    isTimelineEmpty.value && (asset.type === "video" || asset.type === "image");
  const shouldPlaceFirstAudioAssetOnAudioTrack = isTimelineEmpty.value && asset.type === "audio";
  const track = shouldPlaceFirstVisualAssetOnMainTrack
    ? project.value.tracks.find((item) => isPrimaryTimelineTrack(item))
    : shouldPlaceFirstAudioAssetOnAudioTrack
      ? project.value.tracks.find((item) => item.type === "audio")
      : resolveTimelineTrackForAsset(asset, placement?.trackId);

  if (!track || !isTrackCompatibleWithAsset(asset, track)) {
    return;
  }

  const shouldAnchorFirstMainClip =
    project.value.mainTrackMagnetEnabled && isPrimaryTimelineTrack(track) && track.clips.length === 0;
  const rawStart = shouldAnchorFirstMainClip || shouldPlaceFirstVisualAssetOnMainTrack || shouldPlaceFirstAudioAssetOnAudioTrack
    ? 0
    : placement?.start ?? getTrackAppendTime(track);
  const start = snapEnabled.value ? snapAssetPlacementTime(rawStart, placement?.start !== undefined) : rawStart;
  const clip = createTimelineClip({
    id: `clip-${asset.id}-${Date.now()}`,
    assetId: asset.id,
    trackId: track.id,
    name: asset.name,
    type: isPrimaryTimelineTrack(track) && (asset.type === "video" || asset.type === "image") ? "video" : clipTypeForAsset(asset),
    start,
    duration: Math.max(asset.duration, asset.type === "image" || asset.type === "caption" ? 4 : 1),
    volume: asset.type === "audio" ? 0.82 : 1,
    opacity: track.type === "overlay" ? 0.72 : 1,
    captionText: asset.type === "caption" ? asset.name.replace(/\.[^.]+$/, "") : undefined,
  });

  if (options.recordHistory !== false) {
    pushHistory({ wholeTrackIds: [track.id] });
  }

  if (project.value.mainTrackMagnetEnabled && isPrimaryTimelineTrack(track)) {
    insertClipIntoMagneticTrack(track, clip, start);
  } else {
    track.clips.push(clip);
    track.clips.sort((left, right) => left.start - right.start);
  }
  libraryPreviewAssetId.value = "";
  selectedClipId.value = clip.id;
  clearSelectedAssets();
  playhead.value = clip.start;
  markDirty();
  ensureManagedVideoProxy(asset.id);
}

function addAudioPresetToTimeline(preset: AudioTrackPreset) {
  const sourceNodeId = `audio-preset:${preset.id}`;
  const url = `linglux://audio-presets/${preset.id}`;
  let asset = project.value.assets.find((item) => item.sourceNodeId === sourceNodeId || item.url === url);
  const candidateAsset =
    asset ??
    ({
      id: `asset-audio-preset-${preset.id}`,
      type: "audio",
      name: preset.title,
      sourceNodeId,
      url,
      waveformPeaks: createPresetWaveformPeaks(preset),
      duration: preset.duration,
      createdAt: "",
    } satisfies MediaAsset);
  const shouldCreateAsset = !asset;

  const targetTrack = resolveTimelineTrackForAsset(candidateAsset);

  if (!targetTrack) {
    return;
  }

  if (!asset) {
    asset = {
      id: `asset-audio-preset-${preset.id}`,
      type: "audio",
      name: preset.title,
      sourceNodeId,
      url,
      waveformPeaks: createPresetWaveformPeaks(preset),
      duration: preset.duration,
      createdAt: new Date().toISOString(),
    };
    pushHistory({
      assetIds: [asset.id],
      captureAssetOrder: true,
      wholeTrackIds: [targetTrack.id],
    });
    project.value.assets.push(asset);
  }

  addAssetToTimeline(asset.id, undefined, { recordHistory: !shouldCreateAsset });
}

function addTextTemplateToTimeline(preset: TextTemplatePreset, placement?: { trackId?: string; start?: number }) {
  let asset = findTextTemplateAsset(preset);
  const candidateAsset = asset ?? createTextTemplateAsset(preset, "");
  const targetTrack = resolveTimelineTrackForAsset(candidateAsset, placement?.trackId);
  const shouldCreateAsset = !asset;

  if (!targetTrack) {
    return;
  }

  if (!asset) {
    asset = createTextTemplateAsset(preset);
    pushHistory({
      assetIds: [asset.id],
      captureAssetOrder: true,
      wholeTrackIds: [targetTrack.id],
    });
    project.value.assets.push(asset);
  }

  addAssetToTimeline(asset.id, placement, { recordHistory: !shouldCreateAsset });
}

function findTextTemplateAsset(preset: TextTemplatePreset) {
  const sourceNodeId = `text-template:${preset.id}`;
  const url = `linglux://text-templates/${preset.id}`;

  return project.value.assets.find((item) => item.sourceNodeId === sourceNodeId || item.url === url);
}

function createTextTemplateAsset(preset: TextTemplatePreset, createdAt = new Date().toISOString()): MediaAsset {
  return {
    id: `asset-text-template-${preset.id}`,
    type: "caption",
    name: preset.captionText,
    sourceNodeId: `text-template:${preset.id}`,
    url: `linglux://text-templates/${preset.id}`,
    duration: preset.duration,
    createdAt,
  };
}

function handleTimelineAssetDrop(assetId: string, trackId: string, seconds: number) {
  addAssetToTimeline(assetId, { trackId, start: seconds });
}

function moveTimelineClip(clipId: string, targetTrackId: string, seconds: number) {
  const sourceTrack = project.value.tracks.find((track) => track.clips.some((clip) => clip.id === clipId));
  const sourceIndex = sourceTrack?.clips.findIndex((clip) => clip.id === clipId) ?? -1;
  const clip = sourceIndex >= 0 ? sourceTrack?.clips[sourceIndex] : undefined;
  const targetTrack = project.value.tracks.find((track) => track.id === targetTrackId);
  const asset = clip ? project.value.assets.find((item) => item.id === clip.assetId) : undefined;

  if (!sourceTrack || !clip || !targetTrack || !asset || sourceTrack.locked || targetTrack.locked || !isTrackCompatibleWithAsset(asset, targetTrack)) {
    return;
  }

  const start = resolveClipPlacementStart(clipId, targetTrackId, seconds).start;

  if (sourceTrack.id === targetTrack.id && Math.abs(clip.start - start) < 0.001) {
    return;
  }

  pushHistory({ wholeTrackIds: [sourceTrack.id, targetTrack.id] });
  sourceTrack.clips.splice(sourceIndex, 1);
  clip.trackId = targetTrack.id;
  clip.start = start;

  if (project.value.mainTrackMagnetEnabled && isPrimaryTimelineTrack(sourceTrack) && sourceTrack.id !== targetTrack.id) {
    closeTimelineTrackGaps(sourceTrack);
  }

  if (project.value.mainTrackMagnetEnabled && isPrimaryTimelineTrack(targetTrack)) {
    insertClipIntoMagneticTrack(targetTrack, clip, start);
  } else {
    targetTrack.clips.push(clip);
    targetTrack.clips.sort((left, right) => left.start - right.start);
  }

  libraryPreviewAssetId.value = "";
  selectedClipId.value = clip.id;
  clearSelectedAssets();
  playhead.value = clip.start;
  markDirty();
}

function insertClipIntoMagneticTrack(track: TimelineTrack, clip: TimelineClip, requestedStart: number) {
  const orderedClips = track.clips
    .filter((item) => item.id !== clip.id)
    .sort((left, right) => left.start - right.start);
  const insertionIndex = orderedClips.findIndex(
    (item) => requestedStart < item.start + item.duration / 2,
  );

  if (insertionIndex === -1) {
    orderedClips.push(clip);
  } else {
    orderedClips.splice(insertionIndex, 0, clip);
  }

  track.clips = orderedClips;
  closeTimelineTrackGaps(track);
}

function toggleMainTrackMagnet() {
  pushHistory({
    settings: true,
    wholeTrackIds: project.value.tracks.filter((track) => isPrimaryTimelineTrack(track)).map((track) => track.id),
  });
  project.value.mainTrackMagnetEnabled = !project.value.mainTrackMagnetEnabled;

  if (project.value.mainTrackMagnetEnabled) {
    for (const track of project.value.tracks) {
      if (isPrimaryTimelineTrack(track)) {
        closeTimelineTrackGaps(track);
      }
    }
  }

  markDirty();
}

function updateTrack(trackId: string, patch: Partial<Pick<TimelineTrack, "muted" | "visible" | "mediaEnabled">>) {
  const track = project.value.tracks.find((item) => item.id === trackId);

  if (!track) {
    return;
  }

  pushHistory({ trackSettingsIds: [trackId] });
  Object.assign(track, patch);
  markDirty();
}

function startAssetPointerDrag(assetId: string, pointerId: number, clientX: number, clientY: number) {
  if (!project.value.assets.some((asset) => asset.id === assetId)) {
    return;
  }

  cancelAssetPointerDrag();
  selectedClipId.value = "";
  setSingleSelectedAsset(assetId);
  assetPointerDragState.value = {
    kind: "asset",
    assetId,
    pointerId,
    startX: clientX,
    startY: clientY,
    currentX: clientX,
    currentY: clientY,
    isDragging: false,
  };

  window.addEventListener("pointermove", moveAssetPointerDrag);
  window.addEventListener("pointerup", finishAssetPointerDrag);
  window.addEventListener("pointercancel", cancelAssetPointerDrag);
  window.addEventListener("keydown", handleAssetPointerDragKeydown);
}

function startClipPointerDrag(clipId: string, pointerId: number, clientX: number, clientY: number, grabOffsetSeconds: number) {
  const clip = findClip(clipId);
  const sourceTrack = clip ? project.value.tracks.find((track) => track.id === clip.trackId) : undefined;

  if (!clip || sourceTrack?.locked) {
    return;
  }

  cancelAssetPointerDrag();
  selectClip(clip.id);
  assetPointerDragState.value = {
    kind: "clip",
    clipId: clip.id,
    grabOffsetSeconds,
    pointerId,
    startX: clientX,
    startY: clientY,
    currentX: clientX,
    currentY: clientY,
    isDragging: false,
  };

  window.addEventListener("pointermove", moveAssetPointerDrag);
  window.addEventListener("pointerup", finishAssetPointerDrag);
  window.addEventListener("pointercancel", cancelAssetPointerDrag);
  window.addEventListener("keydown", handleAssetPointerDragKeydown);
}

function startTextTemplatePointerDrag(preset: TextTemplatePreset, pointerId: number, clientX: number, clientY: number) {
  cancelAssetPointerDrag();

  const existingAsset = findTextTemplateAsset(preset);

  if (existingAsset) {
    setSingleSelectedAsset(existingAsset.id);
  }

  assetPointerDragState.value = {
    kind: "textTemplate",
    preset,
    pointerId,
    startX: clientX,
    startY: clientY,
    currentX: clientX,
    currentY: clientY,
    isDragging: false,
  };

  window.addEventListener("pointermove", moveAssetPointerDrag);
  window.addEventListener("pointerup", finishAssetPointerDrag);
  window.addEventListener("pointercancel", cancelAssetPointerDrag);
  window.addEventListener("keydown", handleAssetPointerDragKeydown);
}

function moveAssetPointerDrag(event: PointerEvent) {
  const drag = assetPointerDragState.value;

  if (!drag || event.pointerId !== drag.pointerId) {
    return;
  }

  const movedDistance = Math.hypot(event.clientX - drag.startX, event.clientY - drag.startY);

  drag.currentX = event.clientX;
  drag.currentY = event.clientY;

  if (!drag.isDragging && movedDistance >= 6) {
    drag.isDragging = true;
  }

  drag.targetTrackId = drag.isDragging ? getTimelineTrackElementFromPoint(event.clientX, event.clientY)?.dataset.timelineTrackId : undefined;

  if (drag.isDragging) {
    event.preventDefault();
    updateTimelineDragAutoScroll(event.clientX, event.clientY);
  }
}

function finishAssetPointerDrag(event: PointerEvent) {
  const drag = assetPointerDragState.value;

  if (!drag || event.pointerId !== drag.pointerId) {
    return;
  }

  if (drag.isDragging) {
    const placement = getTimelinePlacementFromPoint(event.clientX, event.clientY);

    if (placement) {
      const dropPlacement =
        isFirstVisualAssetDrag.value && primaryTimelineTrackId.value
          ? { trackId: primaryTimelineTrackId.value, start: 0 }
          : isFirstAudioAssetDrag.value && audioTimelineTrackId.value
            ? { trackId: audioTimelineTrackId.value, start: 0 }
            : { trackId: placement.trackId, start: placement.start };

      if (drag.kind === "textTemplate") {
        addTextTemplateToTimeline(drag.preset, dropPlacement);
      } else if (drag.kind === "clip") {
        moveTimelineClip(drag.clipId, dropPlacement.trackId, dropPlacement.start - drag.grabOffsetSeconds);
      } else {
        addAssetToTimeline(drag.assetId, dropPlacement);
      }
    }
  }

  cancelAssetPointerDrag();
}

function cancelAssetPointerDrag(event?: PointerEvent) {
  const drag = assetPointerDragState.value;

  if (event && drag && event.pointerId !== drag.pointerId) {
    return;
  }

  assetPointerDragState.value = null;
  cleanupAssetPointerDragListeners();
}

function handleAssetPointerDragKeydown(event: KeyboardEvent) {
  if (event.key !== "Escape") {
    return;
  }

  event.preventDefault();
  cancelAssetPointerDrag();
}

function cleanupAssetPointerDragListeners() {
  window.removeEventListener("pointermove", moveAssetPointerDrag);
  window.removeEventListener("pointerup", finishAssetPointerDrag);
  window.removeEventListener("pointercancel", cancelAssetPointerDrag);
  window.removeEventListener("keydown", handleAssetPointerDragKeydown);
  stopTimelineDragAutoScroll();
}

function getTimelinePlacementFromPoint(clientX: number, clientY: number) {
  const trackElement = getTimelineTrackElementFromPoint(clientX, clientY);
  const trackId = trackElement?.dataset.timelineTrackId;

  if (!trackElement || !trackId) {
    return undefined;
  }

  const bounds = trackElement.getBoundingClientRect();

  return {
    trackId,
    start: Math.max(0, (clientX - bounds.left) / (TIMELINE_BASE_PIXELS_PER_SECOND * timelineScale.value)),
  };
}

function getTimelineTrackElementFromPoint(clientX: number, clientY: number) {
  return document
    .elementsFromPoint(clientX, clientY)
    .map((element) => element.closest<HTMLElement>("[data-timeline-track-id]"))
    .find((element): element is HTMLElement => element !== null);
}

let timelineDragAutoScrollFrameId: number | undefined;
let timelineDragAutoScrollVelocity = 0;
let timelineDragScroller: HTMLElement | null = null;

function updateTimelineDragAutoScroll(clientX: number, clientY: number) {
  const scroller = document.querySelector<HTMLElement>("[data-timeline-scroller]");

  if (!scroller) {
    stopTimelineDragAutoScroll();
    return;
  }

  const bounds = scroller.getBoundingClientRect();
  const edgeSize = Math.min(64, bounds.width * 0.16);
  let velocity = 0;

  if (clientY >= bounds.top && clientY <= bounds.bottom) {
    if (clientX < bounds.left + edgeSize) {
      velocity = -18 * clamp((bounds.left + edgeSize - clientX) / edgeSize, 0, 1);
    } else if (clientX > bounds.right - edgeSize) {
      velocity = 18 * clamp((clientX - (bounds.right - edgeSize)) / edgeSize, 0, 1);
    }
  }

  timelineDragScroller = scroller;
  timelineDragAutoScrollVelocity = velocity;

  if (velocity !== 0 && timelineDragAutoScrollFrameId === undefined) {
    timelineDragAutoScrollFrameId = window.requestAnimationFrame(runTimelineDragAutoScroll);
  } else if (velocity === 0) {
    stopTimelineDragAutoScroll();
  }
}

function runTimelineDragAutoScroll() {
  const scroller = timelineDragScroller;
  const drag = assetPointerDragState.value;

  if (!scroller || !drag?.isDragging || timelineDragAutoScrollVelocity === 0) {
    stopTimelineDragAutoScroll();
    return;
  }

  const previousScrollLeft = scroller.scrollLeft;
  scroller.scrollLeft += timelineDragAutoScrollVelocity;

  if (scroller.scrollLeft !== previousScrollLeft) {
    timelineDragLayoutRevision.value += 1;
    drag.targetTrackId = getTimelineTrackElementFromPoint(drag.currentX, drag.currentY)?.dataset.timelineTrackId;
  } else {
    stopTimelineDragAutoScroll();
    return;
  }

  timelineDragAutoScrollFrameId = window.requestAnimationFrame(runTimelineDragAutoScroll);
}

function stopTimelineDragAutoScroll() {
  if (timelineDragAutoScrollFrameId !== undefined) {
    window.cancelAnimationFrame(timelineDragAutoScrollFrameId);
  }

  timelineDragAutoScrollFrameId = undefined;
  timelineDragAutoScrollVelocity = 0;
  timelineDragScroller = null;
}

function pushHistory(spec: ProjectHistoryCaptureSpec) {
  const capture = pendingHistoryCapture ?? {
    beforeAssets: new Map<string, MediaAsset | undefined>(),
    tracks: new Map<string, PendingTrackHistoryCapture>(),
  };
  pendingHistoryCapture = capture;

  if (spec.settings && !capture.beforeSettings) {
    capture.beforeSettings = projectHistorySettings(project.value);
  }

  for (const assetId of spec.assetIds ?? []) {
    if (!capture.beforeAssets.has(assetId)) {
      const asset = project.value.assets.find((item) => item.id === assetId);
      capture.beforeAssets.set(assetId, asset ? cloneHistoryValue(asset) : undefined);
    }
  }

  if (spec.captureAssetOrder && !capture.beforeAssetOrder) {
    capture.beforeAssetOrder = project.value.assets.map((asset) => asset.id);
  }

  for (const trackId of spec.wholeTrackIds ?? []) {
    const trackCapture = getOrCreatePendingTrackCapture(capture, trackId);

    if (!trackCapture.wholeTrackCaptured) {
      const track = project.value.tracks.find((item) => item.id === trackId);
      trackCapture.wholeTrackCaptured = true;
      trackCapture.beforeTrack = track ? cloneHistoryValue(track) : undefined;
      trackCapture.beforeSettings = undefined;
      trackCapture.beforeClips.clear();
    }
  }

  for (const trackId of spec.trackSettingsIds ?? []) {
    const trackCapture = getOrCreatePendingTrackCapture(capture, trackId);

    if (!trackCapture.wholeTrackCaptured && !trackCapture.beforeSettings) {
      const track = project.value.tracks.find((item) => item.id === trackId);

      if (track) {
        trackCapture.beforeSettings = trackHistorySettings(track);
      }
    }
  }

  for (const clipCapture of spec.clips ?? []) {
    const trackCapture = getOrCreatePendingTrackCapture(capture, clipCapture.trackId);

    if (trackCapture.wholeTrackCaptured) {
      continue;
    }

    const track = project.value.tracks.find((item) => item.id === clipCapture.trackId);

    for (const clipId of clipCapture.clipIds) {
      if (!trackCapture.beforeClips.has(clipId)) {
        const clip = track?.clips.find((item) => item.id === clipId);
        trackCapture.beforeClips.set(clipId, clip ? cloneHistoryValue(clip) : undefined);
      }
    }
  }
}

function getOrCreatePendingTrackCapture(capture: PendingProjectHistoryCapture, trackId: string) {
  const existing = capture.tracks.get(trackId);

  if (existing) {
    return existing;
  }

  const created: PendingTrackHistoryCapture = {
    wholeTrackCaptured: false,
    beforeClips: new Map<string, TimelineClip | undefined>(),
  };
  capture.tracks.set(trackId, created);
  return created;
}

function markDirty() {
  project.value.duration = calculateProjectDuration(project.value.tracks);

  if (isTimelineEmpty.value) {
    libraryPreviewAssetId.value = "";
    isPlaying.value = false;
    playhead.value = 0;
    stopPlaybackLoop();
  }

  project.value.updatedAt = new Date().toISOString();
  editorVersion.value += 1;
  saveState.value = "未保存";
  commitPendingHistory();
}

function commitPendingHistory() {
  const capture = pendingHistoryCapture;
  pendingHistoryCapture = undefined;

  if (!capture) {
    return;
  }

  const entry = createProjectHistoryEntry(capture);

  if (!entry) {
    return;
  }

  const previous = history.value[history.value.length - 1];

  if (previous && canCoalesceHistoryEntries(previous, entry)) {
    mergeCoalescedHistoryEntry(previous, entry);
  } else {
    history.value.push(entry);
  }
  future.value = [];
  trimHistoryToBudget(history.value);
}

function createProjectHistoryEntry(capture: PendingProjectHistoryCapture): ProjectHistoryEntry | undefined {
  const afterSettings = capture.beforeSettings ? projectHistorySettings(project.value) : undefined;
  const settingsChanged = Boolean(
    capture.beforeSettings && afterSettings && !historyValuesEqual(capture.beforeSettings, afterSettings),
  );
  const assetChanges = [...capture.beforeAssets.entries()].flatMap(([id, before]) => {
    const after = project.value.assets.find((asset) => asset.id === id);

    if (historyValuesEqual(before, after)) {
      return [];
    }

    return [{
      id,
      before: before ? cloneHistoryValue(before) : undefined,
      after: after ? cloneHistoryValue(after) : undefined,
    } satisfies HistoryEntityChange<MediaAsset>];
  });
  const afterAssetOrder = capture.beforeAssetOrder
    ? project.value.assets.map((asset) => asset.id)
    : undefined;
  const assetOrderChanged = Boolean(
    capture.beforeAssetOrder && afterAssetOrder && !historyValuesEqual(capture.beforeAssetOrder, afterAssetOrder),
  );
  const trackChanges = createCapturedTrackHistoryChanges(capture);

  if (!settingsChanged && assetChanges.length === 0 && trackChanges.length === 0 && !assetOrderChanged) {
    return undefined;
  }

  const entry: ProjectHistoryEntry = {
    beforeSettings: settingsChanged ? cloneHistoryValue(capture.beforeSettings) : undefined,
    afterSettings: settingsChanged ? cloneHistoryValue(afterSettings) : undefined,
    assetChanges,
    beforeAssetOrder: assetOrderChanged ? capture.beforeAssetOrder : undefined,
    afterAssetOrder: assetOrderChanged ? afterAssetOrder : undefined,
    trackChanges,
    estimatedBytes: 0,
    createdAtMs: Date.now(),
  };
  entry.coalesceKey = historyEntryCoalesceKey(entry);
  entry.estimatedBytes = JSON.stringify(entry).length * 2;
  return entry;
}

function createCapturedTrackHistoryChanges(capture: PendingProjectHistoryCapture) {
  const changes: TrackHistoryChange[] = [];

  for (const [trackId, trackCapture] of capture.tracks) {
    const afterTrack = project.value.tracks.find((track) => track.id === trackId);

    if (trackCapture.wholeTrackCaptured) {
      changes.push(...diffHistoryTracks(
        trackCapture.beforeTrack ? [trackCapture.beforeTrack] : [],
        afterTrack ? [afterTrack] : [],
      ));
      continue;
    }

    const afterSettings = trackCapture.beforeSettings && afterTrack
      ? trackHistorySettings(afterTrack)
      : undefined;
    const settingsChanged = Boolean(
      trackCapture.beforeSettings && afterSettings && !historyValuesEqual(trackCapture.beforeSettings, afterSettings),
    );
    const clipChanges = [...trackCapture.beforeClips.entries()].flatMap(([clipId, before]) => {
      const after = afterTrack?.clips.find((clip) => clip.id === clipId);

      if (historyValuesEqual(before, after)) {
        return [];
      }

      return [{
        id: clipId,
        before: before ? cloneHistoryValue(before) : undefined,
        after: after ? cloneHistoryValue(after) : undefined,
      } satisfies HistoryEntityChange<TimelineClip>];
    });

    if (settingsChanged || clipChanges.length > 0) {
      changes.push({
        id: trackId,
        beforeSettings: settingsChanged ? trackCapture.beforeSettings : undefined,
        afterSettings: settingsChanged ? afterSettings : undefined,
        clipChanges,
      });
    }
  }

  return changes;
}

function historyEntryCoalesceKey(entry: ProjectHistoryEntry) {
  if (
    entry.beforeSettings ||
    entry.afterSettings ||
    entry.assetChanges.length > 0 ||
    entry.beforeAssetOrder ||
    entry.afterAssetOrder ||
    entry.beforeTrackOrder ||
    entry.afterTrackOrder ||
    entry.trackChanges.length !== 1
  ) {
    return undefined;
  }

  const trackChange = entry.trackChanges[0];

  if (
    trackChange.beforeTrack ||
    trackChange.afterTrack ||
    trackChange.beforeSettings ||
    trackChange.afterSettings ||
    trackChange.beforeClipOrder ||
    trackChange.afterClipOrder ||
    trackChange.clipChanges.length !== 1
  ) {
    return undefined;
  }

  const clipChange = trackChange.clipChanges[0];
  return clipChange.before && clipChange.after ? `clip:${trackChange.id}:${clipChange.id}` : undefined;
}

function canCoalesceHistoryEntries(previous: ProjectHistoryEntry, next: ProjectHistoryEntry) {
  return Boolean(
    previous.coalesceKey &&
      previous.coalesceKey === next.coalesceKey &&
      next.createdAtMs - previous.createdAtMs <= HISTORY_COALESCE_WINDOW_MS,
  );
}

function mergeCoalescedHistoryEntry(previous: ProjectHistoryEntry, next: ProjectHistoryEntry) {
  const previousClipChange = previous.trackChanges[0]?.clipChanges[0];
  const nextClipChange = next.trackChanges[0]?.clipChanges[0];

  if (!previousClipChange || !nextClipChange) {
    return;
  }

  previousClipChange.after = nextClipChange.after ? cloneHistoryValue(nextClipChange.after) : undefined;
  previous.createdAtMs = next.createdAtMs;
  previous.estimatedBytes = JSON.stringify(previous).length * 2;
}

function projectHistorySettings(projectValue: EditorProject): ProjectHistorySettings {
  return {
    name: projectValue.name,
    sourceNodeId: projectValue.sourceNodeId,
    mainTrackMagnetEnabled: projectValue.mainTrackMagnetEnabled,
    fps: projectValue.fps,
    resolution: cloneHistoryValue(projectValue.resolution),
  };
}

function trackHistorySettings(track: TimelineTrack): TrackHistorySettings {
  const { clips: _clips, ...settings } = track;
  return cloneHistoryValue(settings);
}

function diffHistoryTracks(beforeTracks: TimelineTrack[], afterTracks: TimelineTrack[]): TrackHistoryChange[] {
  const beforeById = new Map(beforeTracks.map((track) => [track.id, track]));
  const afterById = new Map(afterTracks.map((track) => [track.id, track]));
  const ids = new Set([...beforeById.keys(), ...afterById.keys()]);
  const changes: TrackHistoryChange[] = [];

  for (const id of ids) {
    const before = beforeById.get(id);
    const after = afterById.get(id);

    if (!before || !after) {
      changes.push({
        id,
        beforeTrack: before ? cloneHistoryValue(before) : undefined,
        afterTrack: after ? cloneHistoryValue(after) : undefined,
        clipChanges: [],
      });
      continue;
    }

    const beforeSettings = trackHistorySettings(before);
    const afterSettings = trackHistorySettings(after);
    const settingsChanged = !historyValuesEqual(beforeSettings, afterSettings);
    const clipChanges = diffHistoryEntities(before.clips, after.clips);
    const beforeClipOrder = before.clips.map((clip) => clip.id);
    const afterClipOrder = after.clips.map((clip) => clip.id);
    const clipOrderChanged = !historyValuesEqual(beforeClipOrder, afterClipOrder);

    if (!settingsChanged && clipChanges.length === 0 && !clipOrderChanged) {
      continue;
    }

    changes.push({
      id,
      beforeSettings: settingsChanged ? beforeSettings : undefined,
      afterSettings: settingsChanged ? afterSettings : undefined,
      clipChanges,
      beforeClipOrder: clipOrderChanged ? beforeClipOrder : undefined,
      afterClipOrder: clipOrderChanged ? afterClipOrder : undefined,
    });
  }

  return changes;
}

function diffHistoryEntities<T extends { id: string }>(beforeItems: T[], afterItems: T[]): HistoryEntityChange<T>[] {
  const beforeById = new Map(beforeItems.map((item) => [item.id, item]));
  const afterById = new Map(afterItems.map((item) => [item.id, item]));
  const ids = new Set([...beforeById.keys(), ...afterById.keys()]);
  const changes: HistoryEntityChange<T>[] = [];

  for (const id of ids) {
    const before = beforeById.get(id);
    const after = afterById.get(id);

    if (historyValuesEqual(before, after)) {
      continue;
    }

    changes.push({
      id,
      before: before ? cloneHistoryValue(before) : undefined,
      after: after ? cloneHistoryValue(after) : undefined,
    });
  }

  return changes;
}

function historyValuesEqual(left: unknown, right: unknown) {
  return JSON.stringify(left) === JSON.stringify(right);
}

function cloneHistoryValue<T>(value: T): T {
  return JSON.parse(JSON.stringify(value)) as T;
}

function trimHistoryToBudget(entries: ProjectHistoryEntry[]) {
  let estimatedBytes = entries.reduce((total, entry) => total + entry.estimatedBytes, 0);

  while (entries.length > HISTORY_MAX_ENTRIES || estimatedBytes > HISTORY_MAX_ESTIMATED_BYTES) {
    const removed = entries.shift();

    if (!removed) {
      break;
    }

    estimatedBytes -= removed.estimatedBytes;
  }
}

function updatePlayhead(seconds: number) {
  libraryPreviewAssetId.value = "";
  const nextSeconds = snapEnabled.value && !isPlayheadScrubbing.value ? snapTime(seconds) : seconds;
  playhead.value = clamp(nextSeconds, 0, project.value.duration);
  playbackLastTimestamp = undefined;
}

function beginPlayheadScrub() {
  isPlayheadScrubbing.value = true;
  playbackLastTimestamp = undefined;

  if (isPlaying.value) {
    stopPlaybackLoop();
  }
}

function endPlayheadScrub(seconds: number) {
  if (!isPlayheadScrubbing.value) {
    return;
  }

  libraryPreviewAssetId.value = "";
  isPlayheadScrubbing.value = false;
  playhead.value = clamp(seconds, 0, project.value.duration);
  playbackLastTimestamp = undefined;

  if (!isPlaying.value) {
    return;
  }

  if (playhead.value >= project.value.duration) {
    isPlaying.value = false;
    stopPlaybackLoop();
    return;
  }

  if (!isPreviewVideoClockActive.value) {
    startPlaybackLoop();
  }
}

function snapTime(seconds: number) {
  return Math.round(seconds * 2) / 2;
}

function snapAssetPlacementTime(seconds: number, hasExplicitDropPoint: boolean) {
  if (hasExplicitDropPoint && seconds < 0.5) {
    return 0;
  }

  return snapTime(seconds);
}

function resolveAssetPlacementStart(seconds: number) {
  const rawStart = Math.max(0, seconds);
  const start = snapEnabled.value ? snapAssetPlacementTime(rawStart, true) : rawStart;

  return {
    start,
    isSnapped: snapEnabled.value && Math.abs(start - rawStart) > 0.001,
    snapGuideTime: undefined,
  };
}

function resolveClipPlacementStart(clipId: string, targetTrackId: string, seconds: number) {
  const rawStart = Math.max(0, seconds);
  const targetTrack = project.value.tracks.find((track) => track.id === targetTrackId);
  const draggedClip = findClip(clipId);

  if (!targetTrack || !draggedClip) {
    return { start: rawStart, isSnapped: false };
  }

  const otherClips = targetTrack.clips
    .filter((clip) => clip.id !== clipId)
    .sort((left, right) => left.start - right.start);

  if (project.value.mainTrackMagnetEnabled && isPrimaryTimelineTrack(targetTrack)) {
    const insertionIndex = otherClips.findIndex(
      (clip) => rawStart < clip.start + clip.duration / 2,
    );
    const clipsBeforeInsertion = insertionIndex === -1 ? otherClips : otherClips.slice(0, insertionIndex);
    const start = clipsBeforeInsertion.reduce((total, clip) => total + clip.duration, 0);

    return { start, isSnapped: true };
  }

  if (!snapEnabled.value) {
    return { start: rawStart, isSnapped: false };
  }

  const snapThreshold = 10 / (TIMELINE_BASE_PIXELS_PER_SECOND * timelineScale.value);
  const clipBoundaryCandidates = project.value.tracks.flatMap((track) =>
    track.clips
      .filter((clip) => clip.id !== clipId)
      .flatMap((clip) => [clip.start, clip.start + clip.duration]),
  );
  const draggedEdges = [
    { time: rawStart, offset: 0 },
    { time: rawStart + draggedClip.duration, offset: draggedClip.duration },
  ];
  const nearestBoundaryAlignment = clipBoundaryCandidates
    .flatMap((candidate) => draggedEdges.map((edge) => ({
      candidate,
      distance: Math.abs(candidate - edge.time),
      start: candidate - edge.offset,
    })))
    .filter((alignment) => alignment.start >= 0)
    .reduce<{ candidate: number; distance: number; start: number } | undefined>(
      (nearest, alignment) => !nearest || alignment.distance < nearest.distance ? alignment : nearest,
      undefined,
    );

  if (nearestBoundaryAlignment && nearestBoundaryAlignment.distance <= snapThreshold) {
    return {
      start: nearestBoundaryAlignment.start,
      isSnapped: true,
      snapGuideTime: nearestBoundaryAlignment.candidate,
    };
  }

  const basicSnapCandidates = [0, playhead.value];
  const nearestBasicCandidate = basicSnapCandidates.reduce((nearest, candidate) =>
    Math.abs(candidate - rawStart) < Math.abs(nearest - rawStart) ? candidate : nearest,
  basicSnapCandidates[0]);

  if (Math.abs(nearestBasicCandidate - rawStart) <= snapThreshold) {
    return { start: Math.max(0, nearestBasicCandidate), isSnapped: true };
  }

  return {
    start: snapAssetPlacementTime(rawStart, true),
    isSnapped: false,
  };
}

function togglePlayback() {
  if (isTimelineEmpty.value || project.value.duration <= 0) {
    isPlaying.value = false;
    playhead.value = 0;
    stopPlaybackLoop();
    return;
  }

  if (isPlaying.value) {
    isPlaying.value = false;
    stopPlaybackLoop();
    return;
  }

  if (playhead.value >= project.value.duration) {
    playhead.value = 0;
  }

  isPlaying.value = true;

  if (isPreviewVideoClockActive.value) {
    stopPlaybackLoop();
  } else {
    startPlaybackLoop();
  }
}

function handlePreviewClockState(isActive: boolean) {
  isPreviewVideoClockActive.value = isActive;

  if (!isPlaying.value) {
    return;
  }

  if (isActive) {
    stopPlaybackLoop();
  } else {
    startPlaybackLoop();
  }
}

function handlePreviewPlayhead(seconds: number) {
  if (isPlayheadScrubbing.value) {
    return;
  }

  const nextPlayhead = clamp(seconds, 0, project.value.duration);
  playbackLastTimestamp = undefined;

  if (nextPlayhead >= project.value.duration - 0.02) {
    playhead.value = project.value.duration;
    isPlaying.value = false;
    stopPlaybackLoop();
    return;
  }

  playhead.value = nextPlayhead;
}

function handlePreviewEnded() {
  isPlaying.value = false;
  stopPlaybackLoop();
}

function startPlaybackLoop() {
  stopPlaybackLoop();
  playbackFrameId = window.requestAnimationFrame(stepPlayback);
}

function stepPlayback(timestamp: number) {
  if (!isPlaying.value) {
    stopPlaybackLoop();
    return;
  }

  if (playbackLastTimestamp === undefined) {
    playbackLastTimestamp = timestamp;
  }

  const elapsedSeconds = Math.max(0, (timestamp - playbackLastTimestamp) / 1000);
  playbackLastTimestamp = timestamp;
  playhead.value = clamp(playhead.value + elapsedSeconds, 0, project.value.duration);

  if (playhead.value >= project.value.duration) {
    isPlaying.value = false;
    stopPlaybackLoop();
    return;
  }

  playbackFrameId = window.requestAnimationFrame(stepPlayback);
}

function stopPlaybackLoop() {
  if (playbackFrameId !== undefined) {
    window.cancelAnimationFrame(playbackFrameId);
  }

  playbackFrameId = undefined;
  playbackLastTimestamp = undefined;
}

function updateClip(clipId: string, patch: Partial<TimelineClip>) {
  const track = project.value.tracks.find((item) => item.clips.some((clip) => clip.id === clipId));

  if (!track) {
    return;
  }

  const index = track.clips.findIndex((clip) => clip.id === clipId);
  pushHistory({ clips: [{ trackId: track.id, clipIds: [clipId] }] });

  track.clips[index] = {
    ...track.clips[index],
    ...patch,
  };
  selectedClipId.value = clipId;
  markDirty();
}

function toggleSelectedAudioBeatMarkers() {
  const clip = selectedAudioClip.value;

  if (!clip) {
    return;
  }

  pushHistory({ clips: [{ trackId: clip.trackId, clipIds: [clip.id] }] });

  if (clip.beatMode === "auto" && clip.beatMarkers && clip.beatMarkers.length > 0) {
    delete clip.beatMode;
    delete clip.beatMarkers;
  } else {
    const asset = project.value.assets.find((item) => item.id === clip.assetId);
    setAudioBeatMarkers(clip, createAudioBeatMarkers(clip, asset?.waveformPeaks, asset?.duration));
  }

  selectedClipId.value = clip.id;
  markDirty();
}

function alignSelectedVideoToBeatMarkers() {
  const alignment = planVideoBeatMarkerAlignment(
    project.value,
    selectedClipId.value,
    MIN_TRIMMED_CLIP_DURATION_SECONDS,
  );

  if (!alignment) {
    return;
  }

  const clipCaptures = new Map<string, string[]>();

  for (const patch of alignment.patches) {
    const clipIds = clipCaptures.get(patch.trackId) ?? [];
    clipIds.push(patch.clipId);
    clipCaptures.set(patch.trackId, clipIds);
  }

  pushHistory({
    clips: [...clipCaptures.entries()].map(([trackId, clipIds]) => ({ trackId, clipIds })),
  });

  for (const patch of alignment.patches) {
    const track = project.value.tracks.find((item) => item.id === patch.trackId);
    const clip = track?.clips.find((item) => item.id === patch.clipId);

    if (!clip) {
      continue;
    }

    clip.start = patch.start;
    clip.duration = patch.duration;
    clip.trimStart = patch.trimStart;
    clip.trimEnd = patch.trimEnd;

    if (patch.beatMarkers) {
      clip.beatMode = "auto";
      clip.beatMarkers = patch.beatMarkers;
    }
  }

  markDirty();
  playhead.value = clamp(playhead.value, 0, project.value.duration);
}

function createAudioBeatMarkers(clip: TimelineClip, peaks?: number[], sourceDuration = 0): AudioBeatMarker[] {
  const clipDuration = Math.max(clip.duration, 0);

  if (clipDuration <= 0) {
    return [];
  }

  if (!peaks || peaks.length < 4 || sourceDuration <= 0) {
    return createFallbackAudioBeatMarkers(clipDuration);
  }

  const speed = Math.max(clip.speed, 0.01);
  const sourceStart = clamp(clip.trimStart, 0, sourceDuration);
  const availableSourceEnd = clamp(sourceDuration - clip.trimEnd, sourceStart, sourceDuration);
  const requestedSourceEnd = sourceStart + clipDuration * speed;
  const sourceEnd = clamp(Math.min(availableSourceEnd, requestedSourceEnd), sourceStart, sourceDuration);

  if (sourceEnd <= sourceStart) {
    return createFallbackAudioBeatMarkers(clipDuration);
  }

  const peakStart = clamp(Math.floor((sourceStart / sourceDuration) * peaks.length), 0, peaks.length - 1);
  const peakEnd = clamp(Math.ceil((sourceEnd / sourceDuration) * peaks.length), peakStart + 1, peaks.length);
  const visiblePeaks = peaks.slice(peakStart, peakEnd).map((peak) => clamp(Math.abs(peak), 0, 1));
  const maximumPeak = Math.max(...visiblePeaks);

  if (maximumPeak <= 0.04) {
    return createFallbackAudioBeatMarkers(clipDuration);
  }

  const averagePeak = visiblePeaks.reduce((total, peak) => total + peak, 0) / visiblePeaks.length;
  const threshold = clamp(averagePeak + (maximumPeak - averagePeak) * 0.34, 0.16, 0.72);
  const candidates: AudioBeatMarker[] = [];

  for (let index = 1; index < visiblePeaks.length - 1; index += 1) {
    const peak = visiblePeaks[index];

    if (peak < threshold || peak < visiblePeaks[index - 1] || peak <= visiblePeaks[index + 1]) {
      continue;
    }

    const sourceTime = ((peakStart + index + 0.5) / peaks.length) * sourceDuration;
    const time = clamp((sourceTime - sourceStart) / speed, 0, clipDuration);
    const intensity = clamp((peak - threshold) / Math.max(maximumPeak - threshold, 0.01), 0.3, 1);

    candidates.push({
      time,
      intensity,
    });
  }

  if (candidates.length === 0) {
    return createFallbackAudioBeatMarkers(clipDuration);
  }

  const maxMarkers = Math.min(AUTO_BEAT_MAX_MARKERS, Math.max(4, Math.ceil(clipDuration / AUTO_BEAT_MIN_GAP_SECONDS)));
  const selectedMarkers: AudioBeatMarker[] = [];

  for (const candidate of candidates.sort((left, right) => right.intensity - left.intensity)) {
    if (selectedMarkers.some((marker) => Math.abs(marker.time - candidate.time) < AUTO_BEAT_MIN_GAP_SECONDS)) {
      continue;
    }

    selectedMarkers.push(candidate);

    if (selectedMarkers.length >= maxMarkers) {
      break;
    }
  }

  return normalizeAudioBeatMarkersForClip(selectedMarkers, clipDuration);
}

function createFallbackAudioBeatMarkers(duration: number): AudioBeatMarker[] {
  if (duration <= 0.2) {
    return [];
  }

  const interval = duration >= 18 ? 0.75 : duration >= 8 ? 0.5 : 0.4;
  const markers: AudioBeatMarker[] = [];
  let markerIndex = 0;

  for (let time = Math.min(interval, duration * 0.35); time < duration - 0.08; time += interval) {
    markers.push({
      time,
      intensity: markerIndex % 4 === 0 ? 0.95 : 0.58,
    });
    markerIndex += 1;
  }

  return normalizeAudioBeatMarkersForClip(markers, duration);
}

function setAudioBeatMarkers(clip: TimelineClip, markers: AudioBeatMarker[]) {
  if (clip.type !== "audio") {
    return;
  }

  const normalizedMarkers = normalizeAudioBeatMarkersForClip(markers, clip.duration);

  if (normalizedMarkers.length === 0) {
    delete clip.beatMode;
    delete clip.beatMarkers;
    return;
  }

  clip.beatMode = "auto";
  clip.beatMarkers = normalizedMarkers;
}

function normalizeAudioBeatMarkersForClip(markers: AudioBeatMarker[], duration: number) {
  return markers
    .filter((marker) => Number.isFinite(marker.time) && marker.time >= 0 && marker.time <= duration)
    .map((marker) => ({
      time: Number(marker.time.toFixed(3)),
      intensity: Number(clamp(Number.isFinite(marker.intensity) ? marker.intensity : 0.65, 0.25, 1).toFixed(2)),
    }))
    .sort((left, right) => left.time - right.time);
}

function toggleClipVisibility(clipId: string) {
  const clip = findClip(clipId);

  if (!clip) {
    return;
  }

  updateClip(clipId, { visible: clip.visible === false });
}

function splitSelectedClip() {
  const clip = selectedClip.value;

  if (!clip) {
    return;
  }

  const splitAt = clamp(snapEnabled.value ? snapTime(playhead.value) : playhead.value, clip.start + 0.5, clip.start + clip.duration - 0.5);

  if (splitAt <= clip.start || splitAt >= clip.start + clip.duration) {
    return;
  }

  const track = project.value.tracks.find((item) => item.id === clip.trackId);

  if (!track) {
    return;
  }

  pushHistory({ wholeTrackIds: [track.id] });

  const firstDuration = splitAt - clip.start;
  const secondDuration = clip.duration - firstDuration;
  const originalBeatMarkers = clip.beatMarkers ? [...clip.beatMarkers] : [];
  const secondClip: TimelineClip = {
    ...cloneClip(clip),
    id: `${clip.id}-split-${Date.now()}`,
    name: `${clip.name} · B`,
    start: splitAt,
    duration: secondDuration,
    trimStart: clip.trimStart + firstDuration,
  };

  clip.duration = firstDuration;

  if (clip.type === "audio" && originalBeatMarkers.length > 0) {
    setAudioBeatMarkers(
      clip,
      originalBeatMarkers.filter((marker) => marker.time < firstDuration - 0.02),
    );
    setAudioBeatMarkers(
      secondClip,
      originalBeatMarkers
        .filter((marker) => marker.time > firstDuration + 0.02)
        .map((marker) => ({
          ...marker,
          time: marker.time - firstDuration,
        })),
    );
  }

  track.clips.push(secondClip);
  track.clips.sort((left, right) => left.start - right.start);
  selectedClipId.value = secondClip.id;
  markDirty();
}

function handleEditorKeydown(event: KeyboardEvent) {
  if (event.key === "Escape") {
    if (editingShortcutAction.value) {
      event.preventDefault();
      cancelShortcutEditing("已取消快捷键修改");
    } else {
      closeShortcutMenu();
    }

    return;
  }

  if (editingShortcutAction.value) {
    handleShortcutEditKeydown(event);
    return;
  }

  if (event.repeat || event.defaultPrevented || event.isComposing || isShortcutBlockedTarget(event.target)) {
    return;
  }

  if ((event.key === "Delete" || event.key === "Backspace") && selectedClip.value) {
    event.preventDefault();
    deleteSelectedClip();
    closeShortcutMenu();
    return;
  }

  if (matchesShortcut(event, UNDO_EDITOR_SHORTCUT)) {
    event.preventDefault();
    undo();
    closeShortcutMenu();
    return;
  }

  if (matchesShortcut(event, editorShortcuts.value.togglePlayback)) {
    event.preventDefault();
    togglePlayback();
    closeShortcutMenu();
    return;
  }

  if (!matchesShortcut(event, editorShortcuts.value.splitClip)) {
    return;
  }

  event.preventDefault();
  splitSelectedClip();
  closeShortcutMenu();
}

function handleEditorKeyup(event: KeyboardEvent) {
  if (!editingShortcutAction.value || event.isComposing) {
    return;
  }

  event.preventDefault();
  event.stopPropagation();

  if (!pendingShortcutBinding.value) {
    if (isModifierOnlyKey(event.key)) {
      isShortcutKeyHeld.value = false;
      shortcutStatusTone.value = "info";
      shortcutStatusMessage.value = "请再按一个非修饰键";
    }

    return;
  }

  const releasedCode = event.code || normalizeShortcutKey(event.key);

  if (releasedCode !== pendingShortcutCode.value) {
    return;
  }

  commitShortcutBinding(pendingShortcutBinding.value);
}

function isShortcutBlockedTarget(target: EventTarget | null) {
  if (!(target instanceof HTMLElement)) {
    return false;
  }

  if (target.closest("input, textarea, select, [contenteditable]:not([contenteditable='false'])")) {
    return true;
  }

  return isShortcutMenuOpen.value && Boolean(target.closest("[data-linglux-shortcut-menu]"));
}

function updateShortcutMenuOpen(open: boolean) {
  if (!open) {
    closeShortcutMenu();
    return;
  }

  isShortcutMenuOpen.value = true;
  shortcutStatusMessage.value = "";
}

function updateInspectorDrawerOpen(open: boolean) {
  if (!open) {
    isInspectorOpen.value = false;
  }
}

function closeShortcutMenu() {
  isShortcutMenuOpen.value = false;
  editingShortcutAction.value = null;
  pendingShortcutBinding.value = null;
  pendingShortcutCode.value = "";
  isShortcutKeyHeld.value = false;
  shortcutStatusMessage.value = "";
}

function beginShortcutEditing(action: EditorShortcutAction) {
  editingShortcutAction.value = action;
  pendingShortcutBinding.value = null;
  pendingShortcutCode.value = "";
  isShortcutKeyHeld.value = false;
  shortcutStatusTone.value = "info";
  shortcutStatusMessage.value = "按住新的快捷键，松开后完成修改";
}

function cancelShortcutEditing(message = "") {
  editingShortcutAction.value = null;
  pendingShortcutBinding.value = null;
  pendingShortcutCode.value = "";
  isShortcutKeyHeld.value = false;
  shortcutStatusTone.value = "info";
  shortcutStatusMessage.value = message;
}

function handleShortcutEditKeydown(event: KeyboardEvent) {
  if (event.isComposing) {
    return;
  }

  event.preventDefault();
  event.stopPropagation();

  if (event.repeat) {
    return;
  }

  isShortcutKeyHeld.value = true;

  if (isModifierOnlyKey(event.key)) {
    shortcutStatusTone.value = "info";
    shortcutStatusMessage.value = "正在修改，请继续按下组合键";
    return;
  }

  const binding = shortcutBindingFromEvent(event);
  const conflictAction = findShortcutConflict(editingShortcutAction.value!, binding);
  pendingShortcutBinding.value = binding;
  pendingShortcutCode.value = event.code || normalizeShortcutKey(event.key);
  shortcutStatusTone.value = conflictAction ? "warning" : "info";
  shortcutStatusMessage.value = conflictAction
    ? `与“${SHORTCUT_ACTION_LABELS[conflictAction]}”冲突，松开后将替换`
    : "正在修改，松开按键完成";
}

function commitShortcutBinding(binding: EditorShortcutBinding) {
  const action = editingShortcutAction.value;

  if (!action) {
    return;
  }

  const conflictAction = findShortcutConflict(action, binding);
  const nextShortcuts: EditorShortcutMap = {
    ...editorShortcuts.value,
    [action]: cloneShortcutBinding(binding),
  };

  if (conflictAction) {
    nextShortcuts[conflictAction] = null;
  }

  editorShortcuts.value = nextShortcuts;
  persistEditorShortcuts(nextShortcuts);
  editingShortcutAction.value = null;
  pendingShortcutBinding.value = null;
  pendingShortcutCode.value = "";
  isShortcutKeyHeld.value = false;
  shortcutStatusTone.value = conflictAction ? "warning" : "success";
  shortcutStatusMessage.value = conflictAction
    ? `快捷键冲突：已将“${SHORTCUT_ACTION_LABELS[conflictAction]}”设为未设置`
    : `“${SHORTCUT_ACTION_LABELS[action]}”已修改为 ${formatShortcutBinding(binding)}`;
}

function initializeEditorShortcuts() {
  const defaults = createDefaultEditorShortcuts();
  editorShortcuts.value = defaults;
  persistEditorShortcuts(defaults);
  editingShortcutAction.value = null;
  pendingShortcutBinding.value = null;
  pendingShortcutCode.value = "";
  isShortcutKeyHeld.value = false;
  shortcutStatusTone.value = "success";
  shortcutStatusMessage.value = "快捷键已恢复初始设置";
}

function shortcutBindingLabels(action: EditorShortcutAction) {
  const binding = editorShortcuts.value[action];

  if (!binding) {
    return ["未设置"];
  }

  if (binding.primaryModifier) {
    const keyLabel = formatShortcutKey(binding.key);
    return [`⌘ ${keyLabel}`, `Ctrl ${keyLabel}`];
  }

  return [formatShortcutBinding(binding)];
}

function shortcutBindingFromEvent(event: KeyboardEvent): EditorShortcutBinding {
  return {
    key: normalizeShortcutKey(event.key),
    modifiers: SHORTCUT_MODIFIERS.filter((modifier) => eventHasModifier(event, modifier)),
  };
}

function matchesShortcut(event: KeyboardEvent, binding: EditorShortcutBinding | null) {
  if (!binding || normalizeShortcutKey(event.key) !== normalizeShortcutKey(binding.key)) {
    return false;
  }

  return bindingModifierSignatures(binding).includes(eventModifierSignature(event));
}

function findShortcutConflict(action: EditorShortcutAction, binding: EditorShortcutBinding) {
  return (Object.keys(editorShortcuts.value) as EditorShortcutAction[]).find((candidateAction) => {
    const candidateBinding = editorShortcuts.value[candidateAction];
    return candidateAction !== action && candidateBinding !== null && shortcutBindingsConflict(binding, candidateBinding);
  });
}

function shortcutBindingsConflict(left: EditorShortcutBinding, right: EditorShortcutBinding) {
  if (normalizeShortcutKey(left.key) !== normalizeShortcutKey(right.key)) {
    return false;
  }

  const rightSignatures = new Set(bindingModifierSignatures(right));
  return bindingModifierSignatures(left).some((signature) => rightSignatures.has(signature));
}

function bindingModifierSignatures(binding: EditorShortcutBinding) {
  if (!binding.primaryModifier) {
    return [modifierSignature(binding.modifiers)];
  }

  return ["Meta", "Control"].map((primaryModifier) =>
    modifierSignature([...binding.modifiers, primaryModifier as ShortcutModifier]),
  );
}

function eventModifierSignature(event: KeyboardEvent) {
  return modifierSignature(SHORTCUT_MODIFIERS.filter((modifier) => eventHasModifier(event, modifier)));
}

function modifierSignature(modifiers: ShortcutModifier[]) {
  const modifierSet = new Set(modifiers);
  return SHORTCUT_MODIFIERS.filter((modifier) => modifierSet.has(modifier)).join("+");
}

function eventHasModifier(event: KeyboardEvent, modifier: ShortcutModifier) {
  if (modifier === "Meta") {
    return event.metaKey;
  }

  if (modifier === "Control") {
    return event.ctrlKey;
  }

  if (modifier === "Alt") {
    return event.altKey;
  }

  return event.shiftKey;
}

function formatShortcutBinding(binding: EditorShortcutBinding) {
  const modifierLabels: Record<ShortcutModifier, string> = {
    Alt: "Alt",
    Control: "Ctrl",
    Meta: "⌘",
    Shift: "Shift",
  };
  const modifiers = binding.primaryModifier
    ? ["⌘ / Ctrl"]
    : binding.modifiers.map((modifier) => modifierLabels[modifier]);

  return [...modifiers, formatShortcutKey(binding.key)].join(" ");
}

function formatShortcutKey(key: string) {
  if (key === " ") {
    return "Space";
  }

  if (key.length === 1) {
    return key.toUpperCase();
  }

  const keyLabels: Record<string, string> = {
    ArrowDown: "↓",
    ArrowLeft: "←",
    ArrowRight: "→",
    ArrowUp: "↑",
    Backspace: "Backspace",
    Delete: "Delete",
    Enter: "Enter",
    Escape: "Esc",
    Tab: "Tab",
  };

  return keyLabels[key] ?? key;
}

function normalizeShortcutKey(key: string) {
  return key.length === 1 && key !== " " ? key.toLowerCase() : key;
}

function isModifierOnlyKey(key: string) {
  return ["Alt", "AltGraph", "Control", "Meta", "OS", "Shift"].includes(key);
}

function createDefaultEditorShortcuts(): EditorShortcutMap {
  return {
    togglePlayback: cloneShortcutBinding(DEFAULT_EDITOR_SHORTCUTS.togglePlayback!),
    splitClip: cloneShortcutBinding(DEFAULT_EDITOR_SHORTCUTS.splitClip!),
    scrollTimelineLeft: cloneShortcutBinding(DEFAULT_EDITOR_SHORTCUTS.scrollTimelineLeft!),
    scrollTimelineRight: cloneShortcutBinding(DEFAULT_EDITOR_SHORTCUTS.scrollTimelineRight!),
  };
}

function cloneShortcutBinding(binding: EditorShortcutBinding): EditorShortcutBinding {
  return {
    key: binding.key,
    modifiers: [...binding.modifiers],
    primaryModifier: binding.primaryModifier,
  };
}

function loadEditorShortcuts(): EditorShortcutMap {
  if (typeof window === "undefined") {
    return createDefaultEditorShortcuts();
  }

  try {
    const saved = window.localStorage.getItem(EDITOR_SHORTCUT_STORAGE_KEY);

    if (!saved) {
      return createDefaultEditorShortcuts();
    }

    const parsed = JSON.parse(saved) as Record<string, unknown>;
    const togglePlayback = parseShortcutBinding(parsed.togglePlayback);
    const splitClip = parseShortcutBinding(parsed.splitClip);
    const scrollTimelineLeft = parseShortcutBinding(parsed.scrollTimelineLeft);
    const scrollTimelineRight = parseShortcutBinding(parsed.scrollTimelineRight);

    return {
      togglePlayback:
        togglePlayback === undefined ? cloneShortcutBinding(DEFAULT_EDITOR_SHORTCUTS.togglePlayback!) : togglePlayback,
      splitClip: splitClip === undefined ? cloneShortcutBinding(DEFAULT_EDITOR_SHORTCUTS.splitClip!) : splitClip,
      scrollTimelineLeft:
        scrollTimelineLeft === undefined ? cloneShortcutBinding(DEFAULT_EDITOR_SHORTCUTS.scrollTimelineLeft!) : scrollTimelineLeft,
      scrollTimelineRight:
        scrollTimelineRight === undefined ? cloneShortcutBinding(DEFAULT_EDITOR_SHORTCUTS.scrollTimelineRight!) : scrollTimelineRight,
    };
  } catch {
    return createDefaultEditorShortcuts();
  }
}

function parseShortcutBinding(value: unknown): EditorShortcutBinding | null | undefined {
  if (value === null) {
    return null;
  }

  if (!value || typeof value !== "object") {
    return undefined;
  }

  const candidate = value as Partial<EditorShortcutBinding>;

  if (typeof candidate.key !== "string" || !Array.isArray(candidate.modifiers)) {
    return undefined;
  }

  const modifiers = candidate.modifiers.filter(
    (modifier): modifier is ShortcutModifier =>
      typeof modifier === "string" && SHORTCUT_MODIFIERS.includes(modifier as ShortcutModifier),
  );

  if (modifiers.length !== candidate.modifiers.length) {
    return undefined;
  }

  return {
    key: normalizeShortcutKey(candidate.key),
    modifiers,
    primaryModifier: candidate.primaryModifier === true,
  };
}

function persistEditorShortcuts(shortcuts: EditorShortcutMap) {
  try {
    window.localStorage.setItem(EDITOR_SHORTCUT_STORAGE_KEY, JSON.stringify(shortcuts));
  } catch {
    // The shortcuts still work for the current session when local storage is unavailable.
  }
}

function deleteSelectedClip() {
  const clip = selectedClip.value;

  if (!clip) {
    return;
  }

  deleteClip(clip.id);
}

function deleteClip(clipId: string) {
  const clip = findClip(clipId);

  if (!clip) {
    return;
  }

  pushHistory({ wholeTrackIds: [clip.trackId] });

  for (const track of project.value.tracks) {
    const index = track.clips.findIndex((item) => item.id === clip.id);

    if (index !== -1) {
      track.clips.splice(index, 1);

      if (project.value.mainTrackMagnetEnabled && isPrimaryTimelineTrack(track)) {
        closeTimelineTrackGaps(track);
      }

      selectedClipId.value = project.value.tracks.flatMap((item) => item.clips)[0]?.id ?? "";
      markDirty();
      return;
    }
  }
}

function trimClip(clipId: string, edge: "start" | "end") {
  const clip = findClip(clipId);

  if (!clip || clip.duration <= 0.75) {
    return;
  }

  const track = project.value.tracks.find((item) => item.id === clip.trackId);

  if (!track) {
    return;
  }

  pushHistory(
    project.value.mainTrackMagnetEnabled && isPrimaryTimelineTrack(track)
      ? { wholeTrackIds: [track.id] }
      : { clips: [{ trackId: track.id, clipIds: [clip.id] }] },
  );
  const originalBeatMarkers = clip.beatMarkers ? [...clip.beatMarkers] : [];

  if (edge === "start") {
    const trimAmount = Math.min(0.5, clip.duration - 0.5);
    clip.start = snapEnabled.value ? snapTime(clip.start + trimAmount) : clip.start + trimAmount;
    clip.duration -= trimAmount;
    clip.trimStart += trimAmount;

    if (clip.type === "audio" && originalBeatMarkers.length > 0) {
      setAudioBeatMarkers(
        clip,
        originalBeatMarkers
          .filter((marker) => marker.time > trimAmount + 0.02)
          .map((marker) => ({
            ...marker,
            time: marker.time - trimAmount,
          })),
      );
    }
  } else {
    clip.duration = Math.max(0.5, clip.duration - 0.5);
    clip.trimEnd += 0.5;

    if (clip.type === "audio" && originalBeatMarkers.length > 0) {
      setAudioBeatMarkers(clip, originalBeatMarkers);
    }
  }

  if (project.value.mainTrackMagnetEnabled && isPrimaryTimelineTrack(track)) {
    closeTimelineTrackGaps(track);
  }

  selectedClipId.value = clip.id;
  markDirty();
}

function beginClipTrim(clipId: string, edge: "start" | "end") {
  const clip = findClip(clipId);
  const track = clip ? project.value.tracks.find((item) => item.id === clip.trackId) : undefined;

  if (!clip || !track || track.locked) {
    return;
  }

  pushHistory(
    project.value.mainTrackMagnetEnabled && isPrimaryTimelineTrack(track)
      ? { wholeTrackIds: [track.id] }
      : { clips: [{ trackId: track.id, clipIds: [clip.id] }] },
  );
  activeClipTrim = {
    clipId,
    edge,
    start: clip.start,
    duration: clip.duration,
    trimStart: clip.trimStart,
    trimEnd: clip.trimEnd,
    beatMarkers: clip.beatMarkers ? cloneHistoryValue(clip.beatMarkers) : undefined,
    changed: false,
  };
  clipTrimSnapGuideTime.value = undefined;
}

function updateClipTrim(clipId: string, edge: "start" | "end", rawDeltaSeconds: number) {
  const drag = activeClipTrim;
  const clip = findClip(clipId);
  const track = clip ? project.value.tracks.find((item) => item.id === clip.trackId) : undefined;

  if (!drag || drag.clipId !== clipId || drag.edge !== edge || !clip || !track) {
    return;
  }

  const speed = Math.max(clip.speed, 0.01);
  const originalEdgeTime = edge === "start" ? drag.start : drag.start + drag.duration;
  const rawEdgeTime = originalEdgeTime + rawDeltaSeconds;
  const snapThreshold = 10 / (TIMELINE_BASE_PIXELS_PER_SECOND * timelineScale.value);
  const boundaryCandidates = project.value.tracks.flatMap((candidateTrack) =>
    candidateTrack.clips
      .filter((candidate) => candidate.id !== clipId)
      .flatMap((candidate) => [candidate.start, candidate.start + candidate.duration]),
  );
  const nearestBoundary = boundaryCandidates.reduce<number | undefined>((nearest, candidate) => {
    if (nearest === undefined) {
      return candidate;
    }

    return Math.abs(candidate - rawEdgeTime) < Math.abs(nearest - rawEdgeTime) ? candidate : nearest;
  }, undefined);
  const shouldAlignToBoundary = snapEnabled.value
    && nearestBoundary !== undefined
    && Math.abs(nearestBoundary - rawEdgeTime) <= snapThreshold;
  const snappedDelta = shouldAlignToBoundary
    ? nearestBoundary - originalEdgeTime
    : snapEnabled.value
      ? snapTime(rawDeltaSeconds)
      : rawDeltaSeconds;

  clipTrimSnapGuideTime.value = shouldAlignToBoundary ? nearestBoundary : undefined;

  if (edge === "start") {
    const delta = clamp(
      snappedDelta,
      -drag.trimStart / speed,
      drag.duration - MIN_TRIMMED_CLIP_DURATION_SECONDS,
    );
    clip.start = drag.start + delta;
    clip.duration = drag.duration - delta;
    clip.trimStart = Math.max(0, drag.trimStart + delta * speed);
    clip.trimEnd = drag.trimEnd;

    if (clipTrimSnapGuideTime.value !== undefined && Math.abs(clip.start - clipTrimSnapGuideTime.value) > 0.001) {
      clipTrimSnapGuideTime.value = undefined;
    }

    if (clip.type === "audio" && drag.beatMarkers) {
      setAudioBeatMarkers(
        clip,
        drag.beatMarkers
          .filter((marker) => marker.time > delta + 0.02)
          .map((marker) => ({ ...marker, time: marker.time - delta })),
      );
    }
  } else {
    const delta = clamp(
      snappedDelta,
      -(drag.duration - MIN_TRIMMED_CLIP_DURATION_SECONDS),
      drag.trimEnd / speed,
    );
    clip.start = drag.start;
    clip.duration = drag.duration + delta;
    clip.trimStart = drag.trimStart;
    clip.trimEnd = Math.max(0, drag.trimEnd - delta * speed);

    if (
      clipTrimSnapGuideTime.value !== undefined
      && Math.abs(clip.start + clip.duration - clipTrimSnapGuideTime.value) > 0.001
    ) {
      clipTrimSnapGuideTime.value = undefined;
    }

    if (clip.type === "audio" && drag.beatMarkers) {
      setAudioBeatMarkers(clip, drag.beatMarkers);
    }
  }

  if (project.value.mainTrackMagnetEnabled && isPrimaryTimelineTrack(track)) {
    closeTimelineTrackGaps(track);
  }

  drag.changed = drag.changed || Math.abs(snappedDelta) >= 0.001;
  selectedClipId.value = clip.id;
}

function endClipTrim(clipId: string) {
  const drag = activeClipTrim;

  if (!drag || drag.clipId !== clipId) {
    return;
  }

  activeClipTrim = undefined;
  clipTrimSnapGuideTime.value = undefined;

  if (drag.changed) {
    markDirty();
  } else {
    pendingHistoryCapture = undefined;
  }
}

function undo() {
  const entry = history.value.pop();

  if (!entry) {
    return;
  }

  pendingHistoryCapture = undefined;
  applyProjectHistoryEntry(entry, "before");
  future.value.push(entry);
  trimHistoryToBudget(future.value);
  normalizeEditorAfterHistoryChange();
}

function redo() {
  const entry = future.value.pop();

  if (!entry) {
    return;
  }

  pendingHistoryCapture = undefined;
  applyProjectHistoryEntry(entry, "after");
  history.value.push(entry);
  trimHistoryToBudget(history.value);
  normalizeEditorAfterHistoryChange();
}

function applyProjectHistoryEntry(entry: ProjectHistoryEntry, direction: "before" | "after") {
  const settings = direction === "before" ? entry.beforeSettings : entry.afterSettings;

  if (settings) {
    project.value.name = settings.name;
    project.value.sourceNodeId = settings.sourceNodeId;
    project.value.mainTrackMagnetEnabled = settings.mainTrackMagnetEnabled;
    project.value.fps = settings.fps;
    project.value.resolution = cloneHistoryValue(settings.resolution);
  }

  applyHistoryEntityChanges(
    project.value.assets,
    entry.assetChanges,
    direction,
    direction === "before" ? entry.beforeAssetOrder : entry.afterAssetOrder,
  );

  for (const change of entry.trackChanges) {
    const wholeTrack = direction === "before" ? change.beforeTrack : change.afterTrack;
    const oppositeWholeTrack = direction === "before" ? change.afterTrack : change.beforeTrack;
    const trackIndex = project.value.tracks.findIndex((track) => track.id === change.id);

    if (wholeTrack || oppositeWholeTrack) {
      if (!wholeTrack && trackIndex !== -1) {
        project.value.tracks.splice(trackIndex, 1);
      } else if (wholeTrack && trackIndex === -1) {
        project.value.tracks.push(cloneHistoryValue(wholeTrack));
      } else if (wholeTrack && trackIndex !== -1) {
        project.value.tracks[trackIndex] = cloneHistoryValue(wholeTrack);
      }

      continue;
    }

    const track = project.value.tracks[trackIndex];

    if (!track) {
      continue;
    }

    const trackSettings = direction === "before" ? change.beforeSettings : change.afterSettings;

    if (trackSettings) {
      Object.assign(track, cloneHistoryValue(trackSettings));
    }

    applyHistoryEntityChanges(
      track.clips,
      change.clipChanges,
      direction,
      direction === "before" ? change.beforeClipOrder : change.afterClipOrder,
    );
  }

  const trackOrder = direction === "before" ? entry.beforeTrackOrder : entry.afterTrackOrder;

  if (trackOrder) {
    project.value.tracks = reorderHistoryEntities(project.value.tracks, trackOrder);
  }
}

function applyHistoryEntityChanges<T extends { id: string }>(
  items: T[],
  changes: HistoryEntityChange<T>[],
  direction: "before" | "after",
  order?: string[],
) {
  for (const change of changes) {
    const target = direction === "before" ? change.before : change.after;
    const index = items.findIndex((item) => item.id === change.id);

    if (!target && index !== -1) {
      items.splice(index, 1);
    } else if (target && index === -1) {
      items.push(cloneHistoryValue(target));
    } else if (target && index !== -1) {
      items[index] = cloneHistoryValue(target);
    }
  }

  if (order) {
    const ordered = reorderHistoryEntities(items, order);
    items.splice(0, items.length, ...ordered);
  }
}

function reorderHistoryEntities<T extends { id: string }>(items: T[], order: string[]) {
  const byId = new Map(items.map((item) => [item.id, item]));
  const ordered = order.flatMap((id) => {
    const item = byId.get(id);

    if (!item) {
      return [];
    }

    byId.delete(id);
    return [item];
  });

  return [...ordered, ...byId.values()];
}

function normalizeEditorAfterHistoryChange() {
  project.value.duration = calculateProjectDuration(project.value.tracks);
  project.value.updatedAt = new Date().toISOString();
  editorVersion.value += 1;
  libraryPreviewAssetId.value = "";
  selectedClipId.value = project.value.tracks.flatMap((track) => track.clips)[0]?.id ?? "";
  if (selectedClipId.value) {
    clearSelectedAssets();
  } else {
    setSingleSelectedAsset(project.value.assets[0]?.id ?? "");
  }
  playhead.value = clamp(playhead.value, 0, project.value.duration);
  saveState.value = "未保存";
}

function setTimelineScale(scale: number) {
  timelineScale.value = clamp(Number(scale.toFixed(3)), TIMELINE_MIN_SCALE, TIMELINE_MAX_SCALE);
}

function zoomIn() {
  setTimelineScale(timelineScale.value + TIMELINE_BUTTON_SCALE_STEP);
}

function zoomOut() {
  setTimelineScale(timelineScale.value - TIMELINE_BUTTON_SCALE_STEP);
}

async function saveProject() {
  saveState.value = "保存中";

  try {
    project.value = await invoke<typeof project.value>("save_edit_project", {
      sessionId: props.session.id,
      project: project.value,
    });
    saveState.value = "已保存";
  } catch (error) {
    if (!isTauri()) {
      project.value.updatedAt = new Date().toISOString();
      saveState.value = "已保存";
    } else {
      saveState.value = "保存失败";
      shortcutStatusTone.value = "warning";
      shortcutStatusMessage.value = normalizeExportError(error);
    }
  }
}

async function exportProject() {
  isExporting.value = true;
  exportError.value = "";
  exportLocationError.value = "";
  completedExportResult.value = null;
  startExportProgress();

  try {
    const result = await runMediaTask<EditorExportResult>(
      "start_export",
      {
        request: {
          sessionId: props.session.id,
          project: project.value,
          preset: selectedPreset.value,
        },
      },
      (task) => {
        activeExportTaskId.value = task.id;
        exportProgress.value = task.progress;
        exportStatus.value = task.status;
      },
    );
    finishExportProgress(result);
  } catch (error) {
    if (!isTauri()) {
      finishExportProgress(createFallbackExport(selectedPreset.value));
    } else {
      exportError.value = normalizeExportError(error);
      failExportProgress();
      saveState.value = "导出失败";
    }
  } finally {
    isExporting.value = false;
    activeExportTaskId.value = "";
  }
}

async function cancelExport() {
  const taskId = activeExportTaskId.value;

  if (!taskId || !isExporting.value) {
    return;
  }

  exportStatus.value = "正在取消";

  try {
    await invoke("cancel_media_task", { taskId });
  } catch (error) {
    exportError.value = normalizeExportError(error);
  }
}

function openExportDialog() {
  if (!isExporting.value) {
    resetExportProgress();
  }

  exportError.value = "";
  exportLocationError.value = "";
  isExportDialogOpen.value = true;
}

function completeExport() {
  const result = completedExportResult.value;

  if (!result) {
    return;
  }

  isExportDialogOpen.value = false;
  completedExportResult.value = null;
  emit("exported", result);
}

async function openCompletedExportLocation() {
  const outputPath = completedExportOutputPath.value;
  exportLocationError.value = "";

  if (!outputPath) {
    exportLocationError.value = "没有可打开的导出路径。";
    return;
  }

  if (!isTauri()) {
    exportLocationError.value = "Web 预览无法打开本地目录。";
    return;
  }

  try {
    await invoke("reveal_export_file", { outputPath });
  } catch (error) {
    exportLocationError.value = normalizeExportError(error);
  }
}

function startExportProgress() {
  exportProgress.value = 0;
  exportStatus.value = "准备素材";
  saveState.value = "导出中";
}

function finishExportProgress(result: EditorExportResult) {
  exportProgress.value = 100;
  exportStatus.value = "导出完成";
  completedExportResult.value = result;
  saveState.value = "已导出";
}

function failExportProgress() {
  exportProgress.value = Math.max(exportProgress.value, 12);
  exportStatus.value = "导出失败";
}

function resetExportProgress() {
  exportProgress.value = 0;
  exportStatus.value = "";
  exportLocationError.value = "";
  completedExportResult.value = null;
}

function createFallbackExport(preset: ExportPreset): EditorExportResult {
  const createdAt = new Date().toISOString();

  return {
    preset,
    artifact: {
      id: `artifact-edit-${Date.now()}`,
      type: "video",
      name: `${project.value.name} · edited.${preset.format}`,
      sourceNodeId: props.session.sourceNodeId ?? "editor",
      url: `linglux://exports/${project.value.id}.${preset.format}`,
      duration: project.value.duration,
      createdAt,
    },
  };
}

function normalizeExportError(error: unknown) {
  if (typeof error === "string") {
    return error;
  }

  if (error instanceof Error) {
    return error.message;
  }

  return "导出失败，请检查素材文件和 FFmpeg 环境。";
}

function cloneClip(clip: TimelineClip): TimelineClip {
  return JSON.parse(JSON.stringify(clip)) as TimelineClip;
}

function clamp(value: number, min: number, max: number) {
  return Math.min(Math.max(value, min), max);
}

async function createImportedAsset(file: File, index: number): Promise<ImportedAssetContext & { file: File } | undefined> {
  const type = detectAssetType(file);

  if (!type) {
    return undefined;
  }

  const url = URL.createObjectURL(file);
  const id = `asset-import-${Date.now()}-${index}-${Math.round(Math.random() * 100000)}`;
  importedObjectUrls.add(url);
  importedAssetFiles.set(id, file);

  const metadata = await readImportedMetadata(url, type, file);

  return {
    asset: {
      id,
      type,
      name: file.name,
      url,
      duration: metadata.duration,
      width: metadata.width,
      height: metadata.height,
      createdAt: new Date().toISOString(),
    },
    file,
  };
}

function createManagedImportedAsset(
  imported: ImportedMediaFile,
  index: number,
): ImportedAssetContext | undefined {
  const type = detectAssetTypeFromName(imported.sourceFileName);

  if (!type) {
    return undefined;
  }

  const url = convertFileSrc(imported.managedPath);
  const metadata = createInitialManagedMetadata(type);

  return {
    asset: {
      id: `asset-import-${Date.now()}-${index}-${Math.round(Math.random() * 100000)}`,
      type,
      name: imported.sourceFileName,
      url,
      filePath: imported.managedPath,
      contentFingerprint: imported.fingerprint,
      duration: metadata.duration,
      width: metadata.width,
      height: metadata.height,
      createdAt: new Date().toISOString(),
    },
  };
}

function createInitialManagedMetadata(type: MediaAssetType): ImportedMetadata {
  return {
    duration:
      type === "image" || type === "caption"
        ? IMPORTED_IMAGE_DURATION_SECONDS
        : IMPORTED_MEDIA_FALLBACK_DURATION_SECONDS,
  };
}

async function hydrateManagedMediaDerivatives(imports: ImportedAssetContext[]) {
  await Promise.allSettled(
    imports.map(({ asset }) =>
      hydrateManagedMediaDerivative(asset, {
        generateWaveform: asset.type === "audio",
        generateProxy: false,
      }),
    ),
  );

  if (activeImportTaskIds.value.length === 0) {
    importTaskStatus.value = "";
  }
}

interface ManagedMediaDerivativeOptions {
  generateWaveform: boolean;
  generateProxy: boolean;
}

async function hydrateManagedMediaDerivative(
  asset: MediaAsset,
  options: ManagedMediaDerivativeOptions,
) {
  if (!asset.filePath || !asset.contentFingerprint || asset.type === "caption") {
    return;
  }

  const derivatives = await runMediaTask<MediaDerivatives>(
    "start_media_derivatives",
    {
      projectId: project.value.id,
      assetId: asset.id,
      sourcePath: asset.filePath,
      mediaKind: asset.type,
      fingerprint: asset.contentFingerprint,
      fallbackDuration: asset.duration,
      generateWaveform: options.generateWaveform,
      generateProxy: options.generateProxy,
    },
    (task) => {
      trackActiveImportTask(task);
      importTaskStatus.value = `${task.status} ${Math.round(task.progress)}%`;
    },
  );
  const currentAsset = project.value.assets.find((item) => item.id === asset.id);

  if (!currentAsset) {
    return;
  }

  applyImportedAssetMetadata(currentAsset.id, {
    duration: derivatives.metadata.duration || currentAsset.duration,
    width: derivatives.metadata.width,
    height: derivatives.metadata.height,
  });
  currentAsset.waveformPeaks = derivatives.waveformPeaks ?? currentAsset.waveformPeaks;

  if (derivatives.thumbnailPath) {
    currentAsset.thumbnailUrl = convertFileSrc(derivatives.thumbnailPath);
  }

  if (derivatives.proxyPath) {
    currentAsset.proxyPath = derivatives.proxyPath;
    currentAsset.url = convertFileSrc(derivatives.proxyPath);
  }

  markDirty();
}

function ensureManagedVideoProxy(assetId: string) {
  const asset = project.value.assets.find((item) => item.id === assetId);

  if (
    asset?.type !== "video" ||
    !asset.filePath ||
    !asset.contentFingerprint ||
    asset.proxyPath ||
    pendingManagedProxyAssetIds.has(asset.id)
  ) {
    return;
  }

  pendingManagedProxyAssetIds.add(asset.id);
  void hydrateManagedMediaDerivative(asset, {
    generateWaveform: true,
    generateProxy: true,
  })
    .catch((error) => {
      shortcutStatusTone.value = "warning";
      shortcutStatusMessage.value = normalizeExportError(error);
    })
    .finally(() => {
      pendingManagedProxyAssetIds.delete(asset.id);

      if (activeImportTaskIds.value.length === 0) {
        importTaskStatus.value = "";
      }
    });
}

async function hydrateImportedMediaMetadata(imports: ImportedAssetContext[]) {
  for (const imported of imports) {
    if (imported.asset.type !== "video") {
      continue;
    }

    const metadata = await readImportedMediaElementMetadata(
      imported.asset.url,
      "video",
      IMPORTED_BACKGROUND_METADATA_TIMEOUT_MS,
    );

    if (!metadata.isReliable) {
      continue;
    }

    applyImportedAssetMetadata(imported.asset.id, toImportedMetadata(metadata));
  }
}

async function hydrateImportedVideoThumbnails(imports: ImportedAssetContext[]) {
  for (const imported of imports) {
    if (imported.asset.type !== "video") {
      continue;
    }

    const asset = project.value.assets.find((item) => item.id === imported.asset.id);

    if (!asset || asset.thumbnailUrl) {
      continue;
    }

    const thumbnailUrl = await createVideoThumbnail(asset.url);
    const currentAsset = project.value.assets.find((item) => item.id === imported.asset.id);

    if (!currentAsset || !thumbnailUrl) {
      continue;
    }

    importedObjectUrls.add(thumbnailUrl);
    currentAsset.thumbnailUrl = thumbnailUrl;
    markDirty();
  }
}

async function hydrateImportedWaveforms(imports: ImportedAssetContext[]) {
  for (const imported of imports) {
    if (imported.asset.type !== "audio" || !imported.file) {
      continue;
    }

    const currentAsset = project.value.assets.find((item) => item.id === imported.asset.id);

    if (!currentAsset) {
      continue;
    }

    const waveformPeaks = await createWaveformPeaks(imported.file);

    if (!waveformPeaks) {
      continue;
    }

    currentAsset.waveformPeaks = waveformPeaks;
    markDirty();
  }
}

function applyImportedAssetMetadata(assetId: string, metadata: ImportedMetadata) {
  const asset = project.value.assets.find((item) => item.id === assetId);

  if (!asset) {
    return;
  }

  const previousDuration = asset.duration;
  let changed = false;

  if (isReliableMediaDuration(metadata.duration) && Math.abs(metadata.duration - asset.duration) > 0.01) {
    asset.duration = metadata.duration;
    changed = true;
  }

  if (metadata.width && metadata.width !== asset.width) {
    asset.width = metadata.width;
    changed = true;
  }

  if (metadata.height && metadata.height !== asset.height) {
    asset.height = metadata.height;
    changed = true;
  }

  if (!changed) {
    return;
  }

  syncFallbackTimelineClipDurations(asset.id, previousDuration, asset.duration);
  markDirty();
  playhead.value = clamp(playhead.value, 0, project.value.duration);
}

function syncFallbackTimelineClipDurations(assetId: string, previousDuration: number, nextDuration: number) {
  if (!isReliableMediaDuration(nextDuration)) {
    return;
  }

  const changedTracks = new Set<TimelineTrack>();

  for (const track of project.value.tracks) {
    for (const clip of track.clips) {
      const stillUsesImportedFallback =
        clip.assetId === assetId &&
        (Math.abs(clip.duration - previousDuration) < 0.05 ||
          Math.abs(clip.duration - IMPORTED_MEDIA_FALLBACK_DURATION_SECONDS) < 0.05);

      if (stillUsesImportedFallback) {
        clip.duration = Math.max(nextDuration, 0.5);
        changedTracks.add(track);
      }
    }
  }

  for (const track of changedTracks) {
    if (project.value.mainTrackMagnetEnabled && isPrimaryTimelineTrack(track)) {
      closeTimelineTrackGaps(track);
    }
  }
}

async function createWaveformPeaks(file: File): Promise<number[] | undefined> {
  const audioBuffer = await decodeAudioFile(file);

  if (!audioBuffer) {
    return undefined;
  }

  return downsampleWaveformPeaks(audioBuffer, WAVEFORM_PEAK_COUNT);
}

function downsampleWaveformPeaks(audioBuffer: AudioBuffer, targetCount: number) {
  if (audioBuffer.length === 0 || audioBuffer.numberOfChannels === 0) {
    return undefined;
  }

  const peakCount = Math.min(targetCount, audioBuffer.length);
  const peaks = new Array<number>(peakCount).fill(0);

  for (let peakIndex = 0; peakIndex < peakCount; peakIndex += 1) {
    const start = Math.floor((peakIndex / peakCount) * audioBuffer.length);
    const end = Math.max(start + 1, Math.floor(((peakIndex + 1) / peakCount) * audioBuffer.length));
    const sampleStep = Math.max(1, Math.floor((end - start) / 256));
    let peak = 0;

    for (let channelIndex = 0; channelIndex < audioBuffer.numberOfChannels; channelIndex += 1) {
      const channel = audioBuffer.getChannelData(channelIndex);

      for (let sampleIndex = start; sampleIndex < end; sampleIndex += sampleStep) {
        peak = Math.max(peak, Math.abs(channel[sampleIndex] ?? 0));
      }
    }

    peaks[peakIndex] = peak;
  }

  const maximumPeak = Math.max(...peaks);

  if (maximumPeak <= 0) {
    return peaks;
  }

  return peaks.map((peak) => Number((peak / maximumPeak).toFixed(4)));
}

function createPresetWaveformPeaks(preset: AudioTrackPreset) {
  const peakCount = 512;

  return Array.from({ length: peakCount }, (_, index) => {
    const progress = index / Math.max(peakCount - 1, 1);
    let amplitude = 0;

    if (preset.previewKind === "lofi") {
      amplitude =
        0.28 +
        Math.abs(Math.sin(progress * Math.PI * 12)) * 0.24 +
        Math.abs(Math.sin(progress * Math.PI * 41)) * 0.12;
    } else if (preset.previewKind === "chiptune") {
      const beatProgress = (progress * 24) % 1;
      amplitude = 0.2 + (beatProgress < 0.62 ? 0.52 : 0.12) + Math.abs(Math.sin(progress * Math.PI * 48)) * 0.1;
    } else {
      const pulseProgress = (progress * 10) % 1;
      amplitude = 0.12 + Math.exp(-pulseProgress * 5.5) * 0.78;
    }

    const fade = Math.min(1, progress * 24, (1 - progress) * 24);
    return Number(clamp(amplitude * Math.max(fade, 0), 0, 1).toFixed(4));
  });
}

function detectAssetType(file: File): MediaAssetType | undefined {
  if (file.type.startsWith("video/")) {
    return "video";
  }

  if (file.type.startsWith("image/")) {
    return "image";
  }

  if (file.type.startsWith("audio/")) {
    return "audio";
  }

  return detectAssetTypeFromName(file.name);
}

function detectAssetTypeFromName(fileName: string): MediaAssetType | undefined {
  const extension = fileName.split(".").pop()?.toLowerCase() ?? "";

  if (["mp4", "mov", "mkv", "webm", "avi", "m4v"].includes(extension)) {
    return "video";
  }

  if (["jpg", "jpeg", "png", "webp", "gif"].includes(extension)) {
    return "image";
  }

  if (["wav", "mp3", "m4a", "aac", "flac", "ogg"].includes(extension)) {
    return "audio";
  }

  if (["srt", "vtt", "txt"].includes(extension)) {
    return "caption";
  }

  return undefined;
}

async function readImportedMetadata(url: string, type: MediaAssetType, file?: File): Promise<ImportedMetadata> {
  if (type === "image") {
    return new Promise((resolve) => {
      const image = new Image();
      const timeout = window.setTimeout(() => resolve({ duration: IMPORTED_IMAGE_DURATION_SECONDS }), 1200);

      image.onload = () => {
        window.clearTimeout(timeout);
        resolve({ duration: IMPORTED_IMAGE_DURATION_SECONDS, width: image.naturalWidth, height: image.naturalHeight });
      };
      image.onerror = () => {
        window.clearTimeout(timeout);
        resolve({ duration: IMPORTED_IMAGE_DURATION_SECONDS });
      };
      image.src = url;
    });
  }

  if (type === "audio") {
    return readImportedAudioMetadata(url, file);
  }

  if (type === "video") {
    return toImportedMetadata(await readImportedMediaElementMetadata(url, "video"));
  }

  return { duration: IMPORTED_IMAGE_DURATION_SECONDS };
}

async function readImportedAudioMetadata(url: string, file?: File): Promise<ImportedMetadata> {
  const elementMetadata = await readImportedMediaElementMetadata(url, "audio");
  const shouldVerifyFallbackDuration =
    Math.abs(elementMetadata.duration - IMPORTED_MEDIA_FALLBACK_DURATION_SECONDS) < 0.001;

  if (elementMetadata.isReliable && !shouldVerifyFallbackDuration) {
    return toImportedMetadata(elementMetadata);
  }

  const decodedDuration = file ? await readDecodedAudioDuration(file) : undefined;

  if (isReliableMediaDuration(decodedDuration)) {
    return { duration: decodedDuration };
  }

  return toImportedMetadata(elementMetadata);
}

function readImportedMediaElementMetadata(
  url: string,
  type: "video" | "audio",
  timeoutMs = IMPORTED_MEDIA_METADATA_TIMEOUT_MS,
): Promise<ImportedMediaElementMetadata> {
  return new Promise((resolve) => {
    const element = type === "video" ? document.createElement("video") : document.createElement("audio");
    let settled = false;
    const timeout = window.setTimeout(() => finish(createFallbackMediaElementMetadata(element)), timeoutMs);

    const finish = (metadata: ImportedMediaElementMetadata) => {
      if (settled) {
        return;
      }

      settled = true;
      window.clearTimeout(timeout);
      element.onloadedmetadata = null;
      element.onerror = null;
      element.removeAttribute("src");
      element.load();
      resolve(metadata);
    };

    element.preload = "metadata";
    element.onloadedmetadata = () => {
      void finishLoadedMediaElementMetadata(element, finish);
    };
    element.onerror = () => finish(createFallbackMediaElementMetadata(element));
    element.src = url;
    element.load();
  });
}

async function finishLoadedMediaElementMetadata(
  element: HTMLMediaElement,
  finish: (metadata: ImportedMediaElementMetadata) => void,
) {
  const metadata = readMediaElementMetadata(element);

  if (metadata.isReliable) {
    finish(metadata);
    return;
  }

  await seekMediaElementForDuration(element);
  finish(readMediaElementMetadata(element));
}

function seekMediaElementForDuration(element: HTMLMediaElement): Promise<boolean> {
  return new Promise((resolve) => {
    let settled = false;
    const timeout = window.setTimeout(() => finish(false), 1200);

    const finish = (didResolveDuration: boolean) => {
      if (settled) {
        return;
      }

      settled = true;
      window.clearTimeout(timeout);
      element.removeEventListener("durationchange", handleDurationChange);
      element.removeEventListener("seeked", handleSeeked);
      element.removeEventListener("error", handleError);
      resolve(didResolveDuration);
    };

    const handleDurationChange = () => {
      if (isReliableMediaDuration(element.duration)) {
        finish(true);
      }
    };
    const handleSeeked = () => handleDurationChange();
    const handleError = () => finish(false);

    element.addEventListener("durationchange", handleDurationChange);
    element.addEventListener("seeked", handleSeeked);
    element.addEventListener("error", handleError, { once: true });

    try {
      element.currentTime = Number.MAX_SAFE_INTEGER;
      handleDurationChange();
    } catch {
      finish(false);
    }
  });
}

function readMediaElementMetadata(element: HTMLMediaElement): ImportedMediaElementMetadata {
  const isReliable = isReliableMediaDuration(element.duration);
  const metadata: ImportedMediaElementMetadata = {
    duration: isReliable ? element.duration : IMPORTED_MEDIA_FALLBACK_DURATION_SECONDS,
    isReliable,
  };

  if (element instanceof HTMLVideoElement) {
    metadata.width = element.videoWidth || undefined;
    metadata.height = element.videoHeight || undefined;
  }

  return metadata;
}

function createFallbackMediaElementMetadata(element: HTMLMediaElement): ImportedMediaElementMetadata {
  return {
    ...readMediaElementMetadata(element),
    duration: IMPORTED_MEDIA_FALLBACK_DURATION_SECONDS,
    isReliable: false,
  };
}

async function readDecodedAudioDuration(file: File): Promise<number | undefined> {
  const audioBuffer = await decodeAudioFile(file);

  if (!audioBuffer || !isReliableMediaDuration(audioBuffer.duration)) {
    return undefined;
  }

  return audioBuffer.duration;
}

async function decodeAudioFile(file: File): Promise<AudioBuffer | undefined> {
  const AudioContextConstructor = window.AudioContext;

  if (!AudioContextConstructor) {
    return undefined;
  }

  let audioContext: AudioContext | undefined;

  try {
    audioContext = new AudioContextConstructor();

    return await audioContext.decodeAudioData(await file.arrayBuffer());
  } catch {
    return undefined;
  } finally {
    if (audioContext) {
      await audioContext.close().catch(() => undefined);
    }
  }
}

function isReliableMediaDuration(duration: number | undefined): duration is number {
  return typeof duration === "number" && Number.isFinite(duration) && duration > 0;
}

function toImportedMetadata(metadata: ImportedMediaElementMetadata): ImportedMetadata {
  const { isReliable: _isReliable, ...importedMetadata } = metadata;

  return importedMetadata;
}

interface ThumbnailFrameCandidate {
  url: string;
  score: number;
  isUsable: boolean;
}

function createVideoThumbnail(url: string): Promise<string | undefined> {
  return new Promise((resolve) => {
    const video = document.createElement("video");
    let settled = false;
    const timeout = window.setTimeout(() => finish(), 6500);

    const finish = (thumbnailUrl?: string) => {
      if (settled) {
        return;
      }

      settled = true;
      window.clearTimeout(timeout);
      video.onloadedmetadata = null;
      video.onerror = null;
      video.removeAttribute("src");
      video.load();
      resolve(thumbnailUrl);
    };

    const scanFrames = async () => {
      const width = video.videoWidth;
      const height = video.videoHeight;

      if (width <= 0 || height <= 0) {
        finish();
        return;
      }

      const canvas = document.createElement("canvas");
      const scale = Math.min(1, 480 / Math.max(width, height));
      canvas.width = Math.max(1, Math.round(width * scale));
      canvas.height = Math.max(1, Math.round(height * scale));

      const context = canvas.getContext("2d");

      if (!context) {
        finish();
        return;
      }

      let bestFrame: ThumbnailFrameCandidate | undefined;

      for (const time of createVideoThumbnailTimes(video.duration)) {
        const frame = await captureVideoThumbnailFrame(video, canvas, context, time);

        if (settled) {
          if (frame) {
            URL.revokeObjectURL(frame.url);
          }
          return;
        }

        if (!frame) {
          continue;
        }

        if (frame.isUsable) {
          if (bestFrame) {
            URL.revokeObjectURL(bestFrame.url);
          }
          finish(frame.url);
          return;
        }

        if (!bestFrame || frame.score > bestFrame.score) {
          if (bestFrame) {
            URL.revokeObjectURL(bestFrame.url);
          }
          bestFrame = frame;
        } else {
          URL.revokeObjectURL(frame.url);
        }
      }

      finish(bestFrame?.url);
    };

    video.preload = "auto";
    video.muted = true;
    video.playsInline = true;
    video.onloadedmetadata = () => {
      void scanFrames().catch(() => finish());
    };
    video.onerror = () => finish();
    video.src = url;
    video.load();
  });
}

function createVideoThumbnailTimes(duration: number) {
  const safeDuration = Number.isFinite(duration) && duration > 0 ? duration : 0;

  if (safeDuration <= 0) {
    return [0];
  }

  const maxSeekTime = Math.max(0, safeDuration - 0.08);
  const rawTimes = [
    0.5,
    1,
    2,
    safeDuration * 0.2,
    safeDuration * 0.4,
    safeDuration * 0.6,
    safeDuration * 0.8,
  ];

  if (safeDuration < 2) {
    rawTimes.unshift(0.05, safeDuration * 0.5);
  }

  const uniqueTimes = new Set<number>();

  for (const time of rawTimes) {
    const clampedTime = clamp(time, 0, maxSeekTime);
    uniqueTimes.add(Math.round(clampedTime * 100) / 100);
  }

  return Array.from(uniqueTimes).sort((left, right) => left - right);
}

async function captureVideoThumbnailFrame(
  video: HTMLVideoElement,
  canvas: HTMLCanvasElement,
  context: CanvasRenderingContext2D,
  time: number,
): Promise<ThumbnailFrameCandidate | undefined> {
  const didSeek = await seekVideoToTime(video, time);

  if (!didSeek) {
    return undefined;
  }

  try {
    context.drawImage(video, 0, 0, canvas.width, canvas.height);
  } catch {
    return undefined;
  }

  const metrics = analyzeThumbnailFrame(context, canvas.width, canvas.height);
  const frameUrl = await canvasToObjectUrl(canvas);

  if (!frameUrl) {
    return undefined;
  }

  return {
    url: frameUrl,
    score: metrics.score,
    isUsable: metrics.isUsable,
  };
}

function seekVideoToTime(video: HTMLVideoElement, time: number): Promise<boolean> {
  return new Promise((resolve) => {
    let resolved = false;
    const timeout = window.setTimeout(() => finish(false), 900);

    const finish = (didSeek: boolean) => {
      if (resolved) {
        return;
      }

      resolved = true;
      window.clearTimeout(timeout);
      video.removeEventListener("seeked", handleSeeked);
      video.removeEventListener("loadeddata", handleLoadedData);
      video.removeEventListener("error", handleError);
      resolve(didSeek);
    };

    const handleSeeked = () => finish(true);
    const handleLoadedData = () => finish(true);
    const handleError = () => finish(false);

    video.addEventListener("seeked", handleSeeked, { once: true });
    video.addEventListener("loadeddata", handleLoadedData, { once: true });
    video.addEventListener("error", handleError, { once: true });

    try {
      if (Math.abs(video.currentTime - time) < 0.05 && video.readyState >= 2) {
        finish(true);
        return;
      }

      video.currentTime = time;
    } catch {
      finish(video.readyState >= 2);
    }
  });
}

function analyzeThumbnailFrame(context: CanvasRenderingContext2D, width: number, height: number) {
  const data = context.getImageData(0, 0, width, height).data;
  let darkPixels = 0;
  let lumaSum = 0;
  let lumaSquaredSum = 0;
  let pixelCount = 0;
  const stride = Math.max(4, Math.floor(data.length / 36000 / 4) * 4);

  for (let index = 0; index < data.length; index += stride) {
    const luma = data[index] * 0.2126 + data[index + 1] * 0.7152 + data[index + 2] * 0.0722;

    if (luma < 18) {
      darkPixels += 1;
    }

    lumaSum += luma;
    lumaSquaredSum += luma * luma;
    pixelCount += 1;
  }

  const brightness = pixelCount > 0 ? lumaSum / pixelCount : 0;
  const variance = pixelCount > 0 ? Math.max(0, lumaSquaredSum / pixelCount - brightness * brightness) : 0;
  const contrast = Math.sqrt(variance);
  const darkPixelRatio = pixelCount > 0 ? darkPixels / pixelCount : 1;
  const score = brightness + contrast * 2 - darkPixelRatio * 50;
  const isUsable =
    (brightness >= 28 && darkPixelRatio < 0.92) ||
    (brightness >= 45 && darkPixelRatio < 0.97) ||
    (contrast >= 14 && darkPixelRatio < 0.985);

  return {
    score,
    isUsable,
  };
}

function canvasToObjectUrl(canvas: HTMLCanvasElement): Promise<string | undefined> {
  return new Promise((resolve) => {
    try {
      canvas.toBlob(
        (blob) => {
          resolve(blob ? URL.createObjectURL(blob) : undefined);
        },
        "image/jpeg",
        0.82,
      );
    } catch {
      resolve(undefined);
    }
  });
}

function preferredTrackTypeForAsset(asset: MediaAsset): TimelineTrackType {
  if (asset.type === "audio") {
    return "audio";
  }

  if (asset.type === "caption") {
    return "caption";
  }

  return "video";
}

function clipTypeForAsset(asset: MediaAsset): TimelineTrackType {
  return preferredTrackTypeForAsset(asset);
}

function compatibleTrackTypesForAsset(asset: MediaAsset): TimelineTrackType[] {
  if (asset.type === "video" || asset.type === "image") {
    return ["video"];
  }

  if (asset.type === "audio") {
    return ["audio"];
  }

  return ["caption"];
}

function isTrackCompatibleWithAsset(asset: MediaAsset, track: TimelineTrack) {
  return compatibleTrackTypesForAsset(asset).includes(track.type);
}

function resolveTimelineTrackForAsset(asset: MediaAsset, requestedTrackId?: string) {
  const preferredType = preferredTrackTypeForAsset(asset);
  const requestedTrack = requestedTrackId ? project.value.tracks.find((track) => track.id === requestedTrackId) : undefined;

  if (requestedTrackId) {
    return requestedTrack && isTrackCompatibleWithAsset(asset, requestedTrack) ? requestedTrack : undefined;
  }

  return project.value.tracks.find((track) => track.type === preferredType);
}

function getTrackAppendTime(track: TimelineTrack) {
  return track.clips.reduce((max, clip) => Math.max(max, clip.start + clip.duration), 0);
}

function cleanupImportedObjectUrls() {
  for (const url of importedObjectUrls) {
    URL.revokeObjectURL(url);
  }

  importedObjectUrls.clear();
  importedAssetFiles.clear();
}
</script>

<template>
  <UDashboardGroup as="section" :persistent="false" class="h-dvh min-h-[720px] overflow-hidden bg-default text-default max-[900px]:h-auto max-[900px]:min-h-dvh" aria-label="Linglux 内置剪辑器">
    <UDashboardPanel
      id="editor-workspace"
      class="relative grid h-full min-w-0 grid-rows-[auto_minmax(0,1fr)] overflow-hidden"
      :ui="{ root: 'border-0 bg-default', body: 'p-0' }"
    >
    <UDashboardNavbar
      as="header"
      :toggle="false"
      class="min-h-14 border-b border-default bg-default/95 px-3 shadow-sm backdrop-blur-xl"
      :ui="{ root: 'gap-3 py-2', left: 'min-w-[220px] gap-2', center: 'min-w-[220px] flex-1 justify-center max-[900px]:justify-start', right: 'ml-auto shrink-0 gap-1.5' }"
    >
      <template #left>
        <UButton color="neutral" variant="soft" square size="sm" title="返回工作流" aria-label="返回工作流" @click="emit('returnToWorkflow')">
          <ArrowLeft :size="15" />
        </UButton>
        <img :src="lingluxLogo" alt="" class="size-8 rounded-xl object-contain shadow-[0_8px_20px_rgb(37_99_235/0.18)]" />
        <div class="min-w-0">
          <div class="flex items-center gap-1.5">
            <h1 class="truncate text-[14px] font-black leading-4 text-highlighted">Linglux Studio</h1>
            <UBadge color="secondary" variant="subtle" size="sm" class="px-1.5 py-0 text-[9px]">v0.1</UBadge>
          </div>
          <p class="truncate text-[9px] font-semibold leading-3 text-muted">AI video editor workspace</p>
        </div>
      </template>

      <template #default>
        <div class="flex min-w-0 flex-wrap items-center justify-center gap-2 text-[11px] font-semibold text-toned max-[900px]:justify-start">
          <span class="text-muted max-[760px]:hidden">当前工程</span>
          <UBadge color="neutral" variant="subtle" size="sm" class="max-w-[240px] truncate">
            {{ project.name }}
          </UBadge>
          <UBadge color="success" variant="subtle" size="sm" class="gap-1 max-[760px]:hidden">
            <span class="size-1.5 rounded-full bg-success" aria-hidden="true"></span>
            云端同步
          </UBadge>
          <UButton v-if="activeImportTaskIds.length > 0" color="neutral" variant="subtle" size="xs" class="max-w-[240px] truncate" type="button" :title="`${importTaskStatus} · 后台处理，可点击取消`" @click="cancelImport">
            后台：{{ importTaskStatus }}
          </UButton>
          <UBadge v-else color="neutral" variant="outline" size="sm" :title="saveState">{{ saveState }}</UBadge>
        </div>
      </template>

      <template #right>
        <UTooltip text="撤销" :kbds="['meta', 'Z']">
          <UButton color="neutral" variant="ghost" square size="sm" type="button" aria-label="撤销（⌘ Z / Ctrl Z）" :disabled="!canUndo" @click="undo">
            <Undo2 :size="15" />
          </UButton>
        </UTooltip>
        <UTooltip text="重做">
          <UButton color="neutral" variant="ghost" square size="sm" type="button" aria-label="重做" :disabled="!canRedo" @click="redo">
            <Redo2 :size="15" />
          </UButton>
        </UTooltip>
        <UTooltip text="保存工程">
          <UButton color="neutral" variant="soft" square size="sm" type="button" aria-label="保存剪辑工程" @click="saveProject">
            <Save :size="15" />
          </UButton>
        </UTooltip>
        <UTooltip text="AI 剪辑助手">
          <UButton
            color="primary"
            :variant="isAgentPanelOpen ? 'soft' : 'ghost'"
            square
            size="sm"
            type="button"
            aria-label="打开 AI 剪辑助手"
            :aria-pressed="isAgentPanelOpen"
            @click="isAgentPanelOpen ? closeAgentPanel() : openAgentPanel()"
          >
            <Sparkles :size="14" />
          </UButton>
        </UTooltip>
        <UPopover
          :open="isShortcutMenuOpen"
          :content="{ side: 'bottom', align: 'end', sideOffset: 8, collisionPadding: 12 }"
          :ui="{ content: 'z-50 w-[360px] overflow-hidden rounded-lg border border-default bg-elevated/98 p-2 shadow-2xl backdrop-blur-xl' }"
          @update:open="updateShortcutMenuOpen"
        >
          <UTooltip text="快捷键">
            <UButton
              color="neutral"
              variant="soft"
              square
              size="sm"
              type="button"
              aria-label="快捷键"
              aria-haspopup="menu"
              :aria-expanded="isShortcutMenuOpen"
            >
              <Keyboard :size="14" />
            </UButton>
          </UTooltip>
          <template #content>
          <div
            data-linglux-shortcut-menu
            role="menu"
            aria-label="编辑器快捷键"
          >
            <div class="flex items-center justify-between gap-3 border-b border-[#252c39] px-2 pb-2 pt-1">
              <span class="min-w-0">
                <strong class="block text-[11px] font-black text-[#e5e7eb]">快捷键设置</strong>
                <span class="block truncate text-[9px] font-semibold text-[#778398]">选择一项，按住新按键并松开</span>
              </span>
              <UButton
                color="neutral"
                variant="outline"
                size="xs"
                class="shrink-0 text-[9px]"
                type="button"
                role="menuitem"
                aria-label="初始化快捷键"
                @click="initializeEditorShortcuts"
              >
                <RotateCcw :size="12" />
                初始化
              </UButton>
            </div>
            <p class="px-2 pb-1.5 pt-1 text-[9px] font-black uppercase tracking-[0.16em] text-[#687386]">预览</p>
            <button
              class="grid w-full grid-cols-[32px_minmax(0,1fr)_auto] items-center gap-2 rounded-lg border px-2 py-2 text-left transition"
              :class="editingShortcutAction === 'togglePlayback' ? 'border-[#3b82f6] bg-[#172747] shadow-[0_0_0_1px_rgb(59_130_246/0.2)]' : 'border-transparent hover:bg-[#1b2230]'"
              type="button"
              role="menuitemradio"
              :aria-checked="editingShortcutAction === 'togglePlayback'"
              @click="beginShortcutEditing('togglePlayback')"
            >
              <span class="grid size-8 place-items-center rounded-lg bg-[#14244a] text-[#60a5fa]">
                <Play :size="15" />
              </span>
              <span class="min-w-0">
                <strong class="block text-[11px] font-black text-[#e5e7eb]">播放 / 暂停</strong>
                <span class="block truncate text-[9px] font-semibold text-[#778398]">切换预览播放状态</span>
              </span>
              <kbd
                v-if="editingShortcutAction === 'togglePlayback' && isShortcutKeyHeld"
                class="rounded border border-[#3b82f6]/70 bg-[#10234a] px-2 py-1 font-mono text-[9px] font-bold text-[#93c5fd]"
              >正在修改</kbd>
              <span v-else class="flex items-center gap-1 text-[9px] font-bold text-[#687386]">
                <template v-for="(label, index) in shortcutBindingLabels('togglePlayback')" :key="label">
                  <span v-if="index > 0">/</span>
                  <kbd class="rounded border border-[#343c4b] bg-[#0c1018] px-1.5 py-1 font-mono text-[#cbd5e1]">{{ label }}</kbd>
                </template>
              </span>
            </button>
            <p class="px-2 pb-1.5 pt-1 text-[9px] font-black uppercase tracking-[0.16em] text-[#687386]">时间线</p>
            <button
              class="grid w-full grid-cols-[32px_minmax(0,1fr)_auto] items-center gap-2 rounded-lg border px-2 py-2 text-left transition"
              :class="editingShortcutAction === 'splitClip' ? 'border-[#3b82f6] bg-[#172747] shadow-[0_0_0_1px_rgb(59_130_246/0.2)]' : 'border-transparent hover:bg-[#1b2230]'"
              type="button"
              role="menuitemradio"
              :aria-checked="editingShortcutAction === 'splitClip'"
              @click="beginShortcutEditing('splitClip')"
            >
              <span class="grid size-8 place-items-center rounded-lg bg-[#102b36] text-[#5eead4]">
                <Scissors :size="15" />
              </span>
              <span class="min-w-0">
                <strong class="block text-[11px] font-black text-[#e5e7eb]">分割片段</strong>
                <span class="block truncate text-[9px] font-semibold text-[#778398]">在播放头处分割选中片段</span>
              </span>
              <kbd
                v-if="editingShortcutAction === 'splitClip' && isShortcutKeyHeld"
                class="rounded border border-[#3b82f6]/70 bg-[#10234a] px-2 py-1 font-mono text-[9px] font-bold text-[#93c5fd]"
              >正在修改</kbd>
              <span v-else class="flex items-center gap-1 text-[9px] font-bold text-[#687386]">
                <template v-for="(label, index) in shortcutBindingLabels('splitClip')" :key="label">
                  <span v-if="index > 0">/</span>
                  <kbd class="rounded border border-[#343c4b] bg-[#0c1018] px-1.5 py-1 font-mono text-[#cbd5e1]">{{ label }}</kbd>
                </template>
              </span>
            </button>
            <button
              class="grid w-full grid-cols-[32px_minmax(0,1fr)_auto] items-center gap-2 rounded-lg border px-2 py-2 text-left transition"
              :class="editingShortcutAction === 'scrollTimelineLeft' ? 'border-[#3b82f6] bg-[#172747] shadow-[0_0_0_1px_rgb(59_130_246/0.2)]' : 'border-transparent hover:bg-[#1b2230]'"
              type="button"
              role="menuitemradio"
              :aria-checked="editingShortcutAction === 'scrollTimelineLeft'"
              @click="beginShortcutEditing('scrollTimelineLeft')"
            >
              <span class="grid size-8 place-items-center rounded-lg bg-[#18263d] text-[#93c5fd]">
                <MoveHorizontal :size="15" />
              </span>
              <span class="min-w-0">
                <strong class="block text-[11px] font-black text-[#e5e7eb]">时间线向左滑动</strong>
                <span class="block truncate text-[9px] font-semibold text-[#778398]">在时间戳区域水平移动视图</span>
              </span>
              <kbd
                v-if="editingShortcutAction === 'scrollTimelineLeft' && isShortcutKeyHeld"
                class="rounded border border-[#3b82f6]/70 bg-[#10234a] px-2 py-1 font-mono text-[9px] font-bold text-[#93c5fd]"
              >正在修改</kbd>
              <span v-else class="flex items-center gap-1 text-[9px] font-bold text-[#687386]">
                <template v-for="(label, index) in shortcutBindingLabels('scrollTimelineLeft')" :key="label">
                  <span v-if="index > 0">/</span>
                  <kbd class="rounded border border-[#343c4b] bg-[#0c1018] px-1.5 py-1 font-mono text-[#cbd5e1]">{{ label }}</kbd>
                </template>
              </span>
            </button>
            <button
              class="grid w-full grid-cols-[32px_minmax(0,1fr)_auto] items-center gap-2 rounded-lg border px-2 py-2 text-left transition"
              :class="editingShortcutAction === 'scrollTimelineRight' ? 'border-[#3b82f6] bg-[#172747] shadow-[0_0_0_1px_rgb(59_130_246/0.2)]' : 'border-transparent hover:bg-[#1b2230]'"
              type="button"
              role="menuitemradio"
              :aria-checked="editingShortcutAction === 'scrollTimelineRight'"
              @click="beginShortcutEditing('scrollTimelineRight')"
            >
              <span class="grid size-8 place-items-center rounded-lg bg-[#18263d] text-[#93c5fd]">
                <MoveHorizontal :size="15" />
              </span>
              <span class="min-w-0">
                <strong class="block text-[11px] font-black text-[#e5e7eb]">时间线向右滑动</strong>
                <span class="block truncate text-[9px] font-semibold text-[#778398]">在时间戳区域水平移动视图</span>
              </span>
              <kbd
                v-if="editingShortcutAction === 'scrollTimelineRight' && isShortcutKeyHeld"
                class="rounded border border-[#3b82f6]/70 bg-[#10234a] px-2 py-1 font-mono text-[9px] font-bold text-[#93c5fd]"
              >正在修改</kbd>
              <span v-else class="flex items-center gap-1 text-[9px] font-bold text-[#687386]">
                <template v-for="(label, index) in shortcutBindingLabels('scrollTimelineRight')" :key="label">
                  <span v-if="index > 0">/</span>
                  <kbd class="rounded border border-[#343c4b] bg-[#0c1018] px-1.5 py-1 font-mono text-[#cbd5e1]">{{ label }}</kbd>
                </template>
              </span>
            </button>
            <p
              v-if="shortcutStatusMessage"
              class="mx-1 mt-2 rounded-md border px-2.5 py-2 text-[9px] font-bold leading-4"
              :class="shortcutStatusTone === 'warning' ? 'border-[#f59e0b]/35 bg-[#3a250d]/65 text-[#fbbf24]' : shortcutStatusTone === 'success' ? 'border-[#10b981]/30 bg-[#0d3028]/65 text-[#6ee7b7]' : 'border-[#3b82f6]/30 bg-[#10264a]/65 text-[#93c5fd]'"
              role="status"
              aria-live="polite"
            >
              {{ shortcutStatusMessage }}
            </p>
          </div>
          </template>
        </UPopover>
        <UTooltip text="属性">
          <UButton color="secondary" variant="soft" square size="sm" type="button" aria-label="打开片段属性" @click="isInspectorOpen = true">
            <SlidersHorizontal :size="14" />
          </UButton>
        </UTooltip>
        <UTooltip text="设置">
          <UButton color="neutral" variant="soft" square size="sm" type="button" aria-label="打开设置" @click="emit('openSettings')">
            <Settings :size="14" />
          </UButton>
        </UTooltip>
        <UTooltip text="导出工程">
          <UButton color="primary" variant="solid" square size="sm" class="shadow-lg shadow-primary/15" type="button" aria-label="打开导出设置" @click="openExportDialog">
            <Download :size="14" />
          </UButton>
        </UTooltip>
      </template>
    </UDashboardNavbar>

    <div class="grid min-h-0 grid-rows-[minmax(280px,1fr)_minmax(340px,44vh)] overflow-hidden max-[900px]:min-h-[980px] max-[900px]:grid-rows-[minmax(680px,auto)_360px]">
      <div class="grid min-h-0 overflow-hidden" :class="previewWorkspaceLayoutClass">
        <MediaBin
          compact
          class="h-full self-stretch overflow-hidden"
          :assets="project.assets"
          :active-asset-id="selectedAssetId"
          :selected-asset-ids="selectedAssetIds"
          :timeline-asset-ids="timelineAssetIds"
          :tts-status="ttsStatus"
          :tts-busy="isTtsBusy"
          :tts-progress="ttsProgress"
          :tts-task-status="ttsTaskStatus"
          :tts-error="ttsError"
          @select-asset="selectAsset"
          @select-assets="selectAssets"
          @import-files="importMediaFiles"
          @import-paths="importMediaPaths"
          @convert-storyboard="prepareStoryboardConversion"
          @add-asset-to-timeline="addAssetToTimeline"
          @add-audio-preset-to-timeline="addAudioPresetToTimeline"
          @setup-tts="setupTts"
          @generate-speech="generateSpeech"
          @cancel-tts="cancelTts"
          @add-text-template-to-timeline="addTextTemplateToTimeline"
          @delete-asset="deleteAsset"
          @delete-assets="deleteAssets"
          @begin-asset-drag="startAssetPointerDrag"
          @begin-text-template-drag="startTextTemplatePointerDrag"
        />
        <PreviewMonitor
          :project="project"
          :selected-clip="selectedClip"
          :preview-asset="libraryPreviewAsset"
          :playhead="playhead"
          :is-playing="isPlaying"
          :is-playhead-scrubbing="isPlayheadScrubbing"
          @toggle-playback="togglePlayback"
          @preview-clock-state="handlePreviewClockState"
          @preview-playhead="handlePreviewPlayhead"
          @preview-ended="handlePreviewEnded"
          @select-caption-clip="selectPreviewCaptionClip"
        />
        <AgentChatPanel
          v-if="isAgentPanelDocked"
          :project="project"
          :editor-version="editorVersion"
          :conversation="agentConversation"
          :is-running="isAgentRunning"
          :status="agentStatus"
          :error="agentError"
          :is-desktop="isTauri()"
          @close="closeAgentPanel"
          @send="sendAgentPrompt"
          @cancel="cancelAgentTurn"
          @apply-plan="applyAgentPlan"
          @reject-plan="rejectAgentPlan"
          @clear="clearAgentConversation"
          @open-settings="emit('openSettings')"
        />
        <InspectorPanel
          v-if="selectedClipIsText && !isAgentPanelDocked"
          class="min-h-0 border-l border-[#20242f] max-[900px]:hidden"
          :selected-clip="selectedClip"
          @update-clip="updateClip"
        />
      </div>

      <TimelinePanel
        :project="project"
        :selected-clip-id="selectedClipId"
        :playhead="playhead"
        :timeline-scale="timelineScale"
        :main-track-magnet-enabled="project.mainTrackMagnetEnabled"
        :is-asset-drag-active="isAssetDragActive"
        :asset-drag-target-track-id="assetDragTargetTrackId"
        :asset-drag-compatible-track-ids="assetDragCompatibleTrackIds"
        :dragging-clip-id="draggedTimelineClip?.id"
        :drag-preview="assetDragPreview"
        :trim-snap-guide-time="clipTrimSnapGuideTime"
        :scroll-left-shortcut="editorShortcuts.scrollTimelineLeft"
        :scroll-right-shortcut="editorShortcuts.scrollTimelineRight"
        @select-clip="selectClip"
        @begin-clip-drag="startClipPointerDrag"
        @begin-playhead-scrub="beginPlayheadScrub"
        @end-playhead-scrub="endPlayheadScrub"
        @update-playhead="updatePlayhead"
        @trim-clip="trimClip"
        @begin-clip-trim="beginClipTrim"
        @update-clip-trim="updateClipTrim"
        @end-clip-trim="endClipTrim"
        @split-selected="splitSelectedClip"
        @delete-selected="deleteSelectedClip"
        @delete-clip="deleteClip"
        @toggle-clip-visibility="toggleClipVisibility"
        @set-timeline-scale="setTimelineScale"
        @zoom-in="zoomIn"
        @zoom-out="zoomOut"
        @toggle-audio-beat-markers="toggleSelectedAudioBeatMarkers"
        @align-selected-video-to-beat-markers="alignSelectedVideoToBeatMarkers"
        @toggle-main-track-magnet="toggleMainTrackMagnet"
        @drop-asset="handleTimelineAssetDrop"
        @update-track="updateTrack"
      />
    </div>

    <div
      v-if="assetPointerDragState?.isDragging && draggedTimelineClip && draggedAsset"
      class="pointer-events-none fixed left-0 top-0 z-[80] overflow-hidden border-2 bg-[#07191c]/96 text-[#d9ffff] shadow-[0_16px_36px_rgb(0_0_0/0.46),0_0_0_1px_rgb(255_255_255/0.05)] backdrop-blur-sm"
      :class="[
        draggedTimelineClip.type === 'video' ? 'rounded-md' : 'rounded-lg',
        assetDragPreview && !assetDragPreview.isCompatible
          ? 'border-[#fb7185]/90 shadow-[0_16px_36px_rgb(0_0_0/0.46),0_0_18px_rgb(251_113_133/0.2)]'
          : 'border-[#67e8f9]/90 shadow-[0_16px_36px_rgb(0_0_0/0.46),0_0_18px_rgb(34_211_238/0.22)]',
      ]"
      :style="clipDragGhostStyle"
      aria-hidden="true"
    >
      <div class="grid size-full grid-rows-[16px_minmax(0,1fr)_14px]">
        <div class="flex min-w-0 items-center justify-between gap-1 bg-[#0f6b73] px-1.5 font-mono text-[8px] font-black leading-[16px]">
          <span class="truncate">{{ draggedTimelineClip.name }}</span>
          <span class="shrink-0">{{ formatTimecode(draggedTimelineClip.duration) }}</span>
        </div>
        <div class="relative overflow-hidden bg-[#10252a]" :style="clipDragGhostThumbnailStyle">
          <span v-if="!draggedAsset.thumbnailUrl && draggedAsset.type !== 'image'" class="absolute inset-0 bg-[linear-gradient(135deg,#143e44_25%,#0e2a2f_25%,#0e2a2f_50%,#143e44_50%,#143e44_75%,#0e2a2f_75%)] bg-[length:12px_12px]"></span>
        </div>
        <div
          class="truncate px-1.5 font-mono text-[8px] font-black leading-[14px]"
          :class="assetDragPreview && !assetDragPreview.isCompatible ? 'bg-[#881337] text-[#fecdd3]' : 'bg-[#08727a] text-[#cffafe]'"
        >
          {{ clipDragStatus }}
        </div>
      </div>
    </div>

    <div
      v-else-if="assetPointerDragState?.isDragging && draggedAsset"
      class="pointer-events-none fixed left-0 top-0 z-[80] grid max-w-[280px] grid-cols-[32px_minmax(0,1fr)] items-center gap-2 rounded-lg border border-[#2f6df6]/65 bg-[#111827]/95 px-3 py-2 text-[#e5e7eb] shadow-[0_18px_42px_rgb(0_0_0/0.36)] backdrop-blur"
      :style="assetDragGhostStyle"
      aria-hidden="true"
    >
      <span class="grid size-8 place-items-center rounded-md bg-[#14305f] text-[#60a5fa]">
        <Video :size="16" />
      </span>
      <span class="min-w-0">
        <strong class="block truncate text-[12px] font-black">{{ draggedTimelineClip?.name ?? draggedAsset.name }}</strong>
        <span class="block truncate text-[10px] font-bold text-[#7f8da3]">{{ draggedTimelineClip ? "松开后移动片段" : "松开后加入时间线" }}</span>
      </span>
    </div>

    <USlideover
      :open="isInspectorDrawerVisible"
      :close="false"
      title="片段属性"
      class="w-full max-w-[420px] bg-transparent p-0 ring-0 shadow-none"
      :ui="{ overlay: 'z-[60] bg-black/55 backdrop-blur-sm', content: 'z-[60]' }"
      @update:open="updateInspectorDrawerOpen"
    >
      <template #content>
        <aside class="absolute right-0 top-0 grid h-full w-full max-w-[420px] grid-rows-[72px_minmax(0,1fr)] overflow-hidden border-l border-[#262c38] bg-[#111620] shadow-[0_0_70px_rgb(0_0_0/0.42)]">
          <header class="flex items-center justify-between gap-3 border-b border-[#262c38] px-5">
            <div class="min-w-0">
              <p class="text-[10px] font-black uppercase tracking-[0.16em] text-[#5f6b7d]">Inspector</p>
              <h2 aria-hidden="true" class="truncate text-[16px] font-black text-highlighted">片段属性</h2>
            </div>
            <button class="grid size-9 place-items-center rounded-lg border border-[#2d3445] bg-[#151a24] text-[#aeb7c7] hover:text-white" type="button" aria-label="关闭片段属性" @click="isInspectorOpen = false">
              <X :size="16" />
            </button>
          </header>
          <InspectorPanel :selected-clip="selectedClip" @update-clip="updateClip" />
        </aside>
      </template>
    </USlideover>

    <USlideover
      :open="isAgentPanelOpen && isAgentDrawerViewport"
      :close="false"
      title="AI 剪辑助手"
      class="w-full max-w-[420px] bg-transparent p-0 ring-0 shadow-none"
      :ui="{ overlay: 'z-[60] bg-black/55 backdrop-blur-sm', content: 'z-[60]' }"
      @update:open="($event) => { if (!$event) closeAgentPanel() }"
    >
      <template #content>
        <AgentChatPanel
          :project="project"
          :editor-version="editorVersion"
          :conversation="agentConversation"
          :is-running="isAgentRunning"
          :status="agentStatus"
          :error="agentError"
          :is-desktop="isTauri()"
          @close="closeAgentPanel"
          @send="sendAgentPrompt"
          @cancel="cancelAgentTurn"
          @apply-plan="applyAgentPlan"
          @reject-plan="rejectAgentPlan"
          @clear="clearAgentConversation"
          @open-settings="emit('openSettings')"
        />
      </template>
    </USlideover>

    <ExportDialog
      :open="isExportDialogOpen"
      :presets="exportPresets"
      :selected-preset="selectedPreset"
      :is-exporting="isExporting"
      :export-progress="exportProgress"
      :export-status="exportStatus"
      :export-error="exportError"
      :export-output-path="completedExportOutputPath"
      :reveal-error="exportLocationError"
      :has-export-result="completedExportResult !== null"
      @close="isExportDialogOpen = false"
      @select-preset="selectedPresetId = $event"
      @export="exportProject"
      @cancel="cancelExport"
      @open-location="openCompletedExportLocation"
      @finish="completeExport"
    />
    <StoryboardToVideoDialog
      :open="isStoryboardDialogOpen"
      :source="storyboardSource"
      :project-id="project.id"
      :project-fps="project.fps"
      :is-generating="isStoryboardGenerating"
      :progress="storyboardProgress"
      :status="storyboardStatus"
      :error="storyboardError"
      @close="isStoryboardDialogOpen = false"
      @generate="generateStoryboardVideo"
      @cancel="cancelStoryboardGeneration"
    />
    </UDashboardPanel>
  </UDashboardGroup>
</template>
