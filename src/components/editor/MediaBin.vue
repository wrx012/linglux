<script setup lang="ts">
import { computed, onUnmounted, ref, watch } from "vue";
import { isTauri } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import {
  Captions,
  Clapperboard,
  CirclePlay,
  Grid2x2,
  Folder,
  Headphones,
  Image as ImageIcon,
  List,
  Music,
  Mic2,
  Plus,
  Search,
  SlidersHorizontal,
  Smile,
  Trash2,
  Type,
  Upload,
  WandSparkles,
  Video,
} from "@lucide/vue";
import type { Component } from "vue";
import type { AudioTrackPreset, MediaAsset, MediaAssetType, TextTemplatePreset, TtsEmotion, TtsStatus, TtsVoice } from "../../types/editor";
import {
  DEFAULT_TTS_EMOTION,
  DEFAULT_TTS_VOICE,
  TTS_EMOTION_OPTIONS,
  TTS_MAX_TEXT_LENGTH,
  TTS_VOICE_OPTIONS,
  validateTtsText,
} from "../../lib/tts";

type AssetFilter = "all" | "audio" | "caption" | "image" | "visual";
type MediaToolId = "assets" | "audio" | "text" | "stickers" | "effects" | "captions" | "color";
type MediaViewMode = "grid" | "list";
type AudioWindow = Window & { webkitAudioContext?: typeof AudioContext };

interface MiddleEllipsisName {
  head: string;
  tail: string;
}

const LINGLUX_ASSET_DRAG_TYPE = "application/x-linglux-asset";
const LINGLUX_ASSET_KIND_DRAG_TYPE = "application/x-linglux-asset-type";
const ASSET_NAME_MIDDLE_ELLIPSIS_THRESHOLD = 16;
const ASSET_NAME_SUFFIX_LENGTH = 7;

const props = defineProps<{
  assets: MediaAsset[];
  activeAssetId?: string;
  selectedAssetIds?: string[];
  timelineAssetIds?: string[];
  compact?: boolean;
  ttsStatus?: TtsStatus;
  ttsBusy?: boolean;
  ttsProgress?: number;
  ttsTaskStatus?: string;
  ttsError?: string;
}>();

const emit = defineEmits<{
  selectAsset: [assetId: string];
  selectAssets: [assetIds: string[]];
  importFiles: [files: File[]];
  importPaths: [paths: string[]];
  importImageSequenceFiles: [files: File[]];
  importImageSequencePaths: [paths: string[]];
  convertStoryboard: [path: string];
  addAssetToTimeline: [assetId: string];
  addAudioPresetToTimeline: [preset: AudioTrackPreset];
  addTextTemplateToTimeline: [preset: TextTemplatePreset];
  deleteAsset: [assetId: string];
  deleteAssets: [assetIds: string[]];
  beginAssetDrag: [assetId: string, pointerId: number, clientX: number, clientY: number];
  beginTextTemplateDrag: [preset: TextTemplatePreset, pointerId: number, clientX: number, clientY: number];
  setupTts: [];
  generateSpeech: [request: { text: string; voice: TtsVoice; emotion: TtsEmotion; speed: number }];
  cancelTts: [];
}>();

const searchQuery = ref("");
const activeTool = ref<MediaToolId>("assets");
const viewMode = ref<MediaViewMode>("grid");
const fileInput = ref<HTMLInputElement | null>(null);
const imageSequenceInput = ref<HTMLInputElement | null>(null);
const assetScroller = ref<HTMLElement | null>(null);
const isImportDragActive = ref(false);
const previewingAudioPresetId = ref<string | null>(null);
const speechText = ref("");
const speechVoice = ref<TtsVoice>(DEFAULT_TTS_VOICE);
const speechEmotion = ref<TtsEmotion>(DEFAULT_TTS_EMOTION);
const speechSpeed = ref(1);
const speechVoiceOptions = TTS_VOICE_OPTIONS;
const speechEmotionOptions = TTS_EMOTION_OPTIONS;
const localSpeechError = computed(() => validateTtsText(speechText.value));
const speechCharacterCount = computed(() => [...speechText.value].length);
const canGenerateSpeech = computed(() => props.ttsStatus?.state === "ready" && !props.ttsBusy && !localSpeechError.value);

function submitSpeech() {
  if (!canGenerateSpeech.value) return;
  emit("generateSpeech", {
    text: speechText.value.trim(),
    voice: speechVoice.value,
    emotion: speechEmotion.value,
    speed: speechSpeed.value,
  });
}
const marqueeSelectionState = ref<{
  pointerId: number;
  startClientX: number;
  startClientY: number;
  currentClientX: number;
  currentClientY: number;
  startContentX: number;
  startContentY: number;
  currentContentX: number;
  currentContentY: number;
  isActive: boolean;
  additive: boolean;
  baseAssetIds: string[];
} | null>(null);
const textTemplatePointerState = ref<{
  presetId: string;
  pointerId: number;
  startX: number;
  startY: number;
  didDrag: boolean;
} | null>(null);
const suppressedTextTemplateClickId = ref<string | null>(null);
let audioPreviewContext: AudioContext | undefined;
let audioPreviewCleanup: (() => void) | undefined;
let audioPreviewTimer: number | undefined;

const assetIcons: Record<MediaAssetType, Component> = {
  video: Video,
  image: ImageIcon,
  audio: Music,
  caption: Type,
};

const toolTabs: Array<{ id: MediaToolId; label: string; icon: Component; filter?: AssetFilter }> = [
  { id: "assets", label: "资源库", icon: Folder, filter: "all" },
  { id: "audio", label: "音频轨", icon: Headphones, filter: "audio" },
  { id: "text", label: "文本字", icon: Captions },
  { id: "stickers", label: "加贴纸", icon: Smile, filter: "image" },
  { id: "effects", label: "滤镜特效", icon: WandSparkles, filter: "visual" },
  { id: "captions", label: "字幕轨", icon: Type, filter: "caption" },
  { id: "color", label: "色彩调节", icon: SlidersHorizontal, filter: "visual" },
];

const audioTrackPresets: AudioTrackPreset[] = [
  { id: "lofi-noon", title: "Lo-Fi 慵懒午后（复古）", duration: 15, durationLabel: "00:15", previewKind: "lofi" },
  { id: "8bit-white-machine", title: "8Bit 经典复古红白机", duration: 8, durationLabel: "00:08", previewKind: "chiptune" },
  { id: "electro-pulse", title: "电子迷幻脉冲音效", duration: 4, durationLabel: "00:04", previewKind: "pulse" },
];

const textTemplatePresets: TextTemplatePreset[] = [
  { id: "minimal-white", title: "HELLO WORLD", subtitle: "极简白色花字", captionText: "HELLO WORLD", duration: 4 },
  { id: "cyberpunk-neon", title: "CYBERPUNK", subtitle: "赛博红蓝花字", captionText: "CYBERPUNK", duration: 4 },
];

watch(activeTool, () => {
  stopAudioPreview();
  cleanupMarqueeSelectionListeners();
});

onUnmounted(() => {
  stopAudioPreview();
  cleanupMarqueeSelectionListeners();
  cleanupTextTemplatePointerListeners();
  void audioPreviewContext?.close();
});

const filteredAssets = computed(() => {
  const query = searchQuery.value.trim().toLowerCase();

  return props.assets.filter((asset) => {
    const matchesFilter = assetMatchesFilter(asset, activeAssetFilter.value);
    const matchesQuery = !query || asset.name.toLowerCase().includes(query) || asset.type.toLowerCase().includes(query);

    return matchesFilter && matchesQuery;
  });
});
const isAssetLibraryEmpty = computed(() => props.assets.length === 0);
const timelineAssetIdSet = computed(() => new Set(props.timelineAssetIds ?? []));
const selectedAssetIdSet = computed(() => new Set(currentSelectedAssetIds()));
const selectedDeletableAssetIds = computed(() =>
  props.assets.filter((asset) => selectedAssetIdSet.value.has(asset.id) && isAssetDeletable(asset)).map((asset) => asset.id),
);
const selectedDeletableAssetCount = computed(() => selectedDeletableAssetIds.value.length);
const marqueeSelectionStyle = computed(() => {
  const state = marqueeSelectionState.value;

  if (!state?.isActive) {
    return {};
  }

  const left = Math.min(state.startContentX, state.currentContentX);
  const top = Math.min(state.startContentY, state.currentContentY);
  const width = Math.abs(state.currentContentX - state.startContentX);
  const height = Math.abs(state.currentContentY - state.startContentY);

  return {
    left: `${left}px`,
    top: `${top}px`,
    width: `${width}px`,
    height: `${height}px`,
  };
});

const assetCountLabel = computed(() => {
  if (filteredAssets.value.length === props.assets.length) {
    return `${props.assets.length} 个文件`;
  }

  return `${filteredAssets.value.length}/${props.assets.length} 个文件`;
});

const activeToolTab = computed(() => toolTabs.find((tab) => tab.id === activeTool.value) ?? toolTabs[0]);
const activeAssetFilter = computed<AssetFilter>(() => activeToolTab.value.filter ?? "all");
const activeTabLabel = computed(() => activeToolTab.value.label);
const assetPanelDescription = computed(() => {
  if (activeTool.value === "captions") {
    return "字幕文件和文本资产";
  }

  if (activeTool.value === "stickers") {
    return "贴纸、图片和叠加素材";
  }

  if (activeTool.value === "effects" || activeTool.value === "color") {
    return "图片、视频和视觉参考";
  }

  return "项目素材、字幕和音频资产";
});
const navigationIconSize = computed(() => (props.compact ? 16 : 18));
const viewIconSize = computed(() => (props.compact ? 14 : 16));
const importIconSize = computed(() => (props.compact ? 13 : 15));
const uploadIconSize = computed(() => (props.compact ? 18 : 22));

function formatDuration(duration: number) {
  if (duration <= 0) {
    return "00:00";
  }

  const minutes = Math.floor(duration / 60);
  const seconds = Math.floor(duration % 60);

  return `${minutes.toString().padStart(2, "0")}:${seconds.toString().padStart(2, "0")}`;
}

function assetDisplayName(asset: MediaAsset) {
  return asset.name.replace(/\.[^.]+$/, "").replace(/[_-]+/g, " ");
}

function middleEllipsisNameParts(name: string): MiddleEllipsisName {
  const normalizedName = name.trim() || name;

  if (normalizedName.length <= ASSET_NAME_MIDDLE_ELLIPSIS_THRESHOLD) {
    return { head: normalizedName, tail: "" };
  }

  const extension = normalizedName.match(/\.[^.]{1,8}$/)?.[0] ?? "";
  const suffixLength = Math.min(
    normalizedName.length - 1,
    Math.max(ASSET_NAME_SUFFIX_LENGTH, extension ? extension.length + 4 : ASSET_NAME_SUFFIX_LENGTH),
  );

  return {
    head: normalizedName.slice(0, normalizedName.length - suffixLength),
    tail: normalizedName.slice(-suffixLength),
  };
}

function assetDisplayNameParts(asset: MediaAsset) {
  return middleEllipsisNameParts(assetDisplayName(asset));
}

function assetKindLabel(type: MediaAssetType) {
  const labels: Record<MediaAssetType, string> = {
    video: "VIDEO",
    image: "IMAGE",
    audio: "AUDIO",
    caption: "TEXT",
  };

  return labels[type];
}

function assetMeta(asset: MediaAsset) {
  if (asset.type === "image") {
    return asset.width && asset.height ? `${asset.width}x${asset.height}` : "静态图片";
  }

  return `${formatDuration(asset.duration)} | ${asset.type.toUpperCase()}`;
}

function assetPreviewUrl(asset: MediaAsset) {
  if (asset.thumbnailUrl) {
    return asset.thumbnailUrl;
  }

  if (asset.type === "image") {
    return asset.url;
  }

  return "";
}

function isVisualAddedToTimeline(asset: MediaAsset) {
  return (asset.type === "video" || asset.type === "image") && timelineAssetIdSet.value.has(asset.id);
}

function addAssetTypeLabel(asset: MediaAsset) {
  if (asset.type === "audio") return "音频";
  if (asset.type === "image") return "图片";
  return "视频";
}

function currentSelectedAssetIds() {
  if (props.selectedAssetIds !== undefined) {
    return props.selectedAssetIds;
  }

  return props.activeAssetId ? [props.activeAssetId] : [];
}

function isAssetSelected(assetId: string) {
  return selectedAssetIdSet.value.has(assetId);
}

function gridAssetSelectionClass(assetId: string) {
  if (isAssetSelected(assetId)) {
    return "border-[#67e8f9] shadow-[0_0_0_1px_rgb(103_232_249/0.8),0_10px_24px_rgb(8_145_178/0.2)]";
  }

  if (props.activeAssetId === assetId) {
    return "border-[#2f6df6] shadow-[0_0_0_1px_rgb(47_109_246/0.35),0_12px_28px_rgb(47_109_246/0.14)]";
  }

  return "border-[#242b3a]";
}

function listAssetSelectionClass(assetId: string) {
  if (isAssetSelected(assetId)) {
    return "border-[#67e8f9] bg-[#14212b] shadow-[0_0_0_1px_rgb(103_232_249/0.48)]";
  }

  if (props.activeAssetId === assetId) {
    return "border-[#2f6df6]";
  }

  return "border-[#242b3a]";
}

function isAssetDeletable(asset: MediaAsset) {
  return asset.type === "video" || asset.type === "audio" || asset.type === "image";
}

function deleteAssetLabel(asset: MediaAsset) {
  if (asset.type === "audio") {
    return `删除音频 ${asset.name}`;
  }

  if (asset.type === "video") {
    return `删除视频 ${asset.name}`;
  }

  if (asset.type === "image") {
    return `删除图片 ${asset.name}`;
  }

  return `删除素材 ${asset.name}`;
}

function cardTone(asset: MediaAsset, index: number) {
  if (asset.type === "audio") {
    return "from-[#0f3d2c] to-[#0b1d18]";
  }

  if (asset.type === "caption") {
    return "from-[#3a1b54] to-[#181123]";
  }

  const tones = [
    "from-[#183044] to-[#0f172a]",
    "from-[#4b1d1d] to-[#1f1113]",
    "from-[#164427] to-[#0d1f16]",
    "from-[#34194c] to-[#151020]",
    "from-[#111827] to-[#090d18]",
  ];

  return tones[index % tones.length];
}

function textTemplatePreviewClass(preset: TextTemplatePreset) {
  if (preset.id === "cyberpunk-neon") {
    return "text-[#ff3158] [text-shadow:-1px_0_8px_rgb(6_182_212/0.75),1px_0_10px_rgb(244_63_94/0.8)]";
  }

  return "text-white [text-shadow:0_1px_10px_rgb(255_255_255/0.18)]";
}

function assetMatchesFilter(asset: MediaAsset, filter: AssetFilter) {
  if (filter === "all") {
    return true;
  }

  if (filter === "visual") {
    return asset.type === "video" || asset.type === "image";
  }

  return asset.type === filter;
}

async function openFilePicker() {
  if (isTauri()) {
    const selected = await open({
      title: "导入媒体素材",
      multiple: true,
      directory: false,
      filters: [
        {
          name: "视频、图片、音频和字幕",
          extensions: [
            "mp4",
            "mov",
            "mkv",
            "webm",
            "avi",
            "m4v",
            "jpg",
            "jpeg",
            "png",
            "webp",
            "gif",
            "wav",
            "mp3",
            "m4a",
            "aac",
            "flac",
            "ogg",
            "srt",
            "vtt",
            "txt",
          ],
        },
      ],
    });
    const paths = Array.isArray(selected) ? selected : selected ? [selected] : [];

    if (paths.length > 0) {
      emit("importPaths", paths);
    }

    return;
  }

  fileInput.value?.click();
}

async function openStoryboardPicker() {
  if (!isTauri()) {
    return;
  }
  const selected = await open({
    title: "选择分镜宫格图",
    multiple: false,
    directory: false,
    filters: [{ name: "分镜图片", extensions: ["jpg", "jpeg", "png", "webp"] }],
  });
  if (typeof selected === "string") {
    emit("convertStoryboard", selected);
  }
}

async function openImageSequencePicker() {
  if (isTauri()) {
    const selected = await open({
      title: "导入动态漫图片序列",
      multiple: true,
      directory: false,
      filters: [{ name: "图片", extensions: ["jpg", "jpeg", "png", "webp", "gif"] }],
    });
    const paths = Array.isArray(selected) ? selected : selected ? [selected] : [];
    if (paths.length) emit("importImageSequencePaths", paths);
    return;
  }
  imageSequenceInput.value?.click();
}

function handleImageSequenceInput(event: Event) {
  const input = event.target as HTMLInputElement;
  const files = Array.from(input.files ?? []).filter((file) => file.type.startsWith("image/") || /\.(jpe?g|png|webp|gif)$/i.test(file.name));
  if (files.length) emit("importImageSequenceFiles", files);
  input.value = "";
}

function handleFileInput(event: Event) {
  const input = event.target as HTMLInputElement;
  emitImportedFiles(Array.from(input.files ?? []));
  input.value = "";
}

function handleImportDrop(event: DragEvent) {
  isImportDragActive.value = false;

  if (isTauri()) {
    return;
  }

  emitImportedFiles(Array.from(event.dataTransfer?.files ?? []));
}

function emitImportedFiles(files: File[]) {
  if (files.length === 0) {
    return;
  }

  emit("importFiles", files);
}

function handleAssetDragStart(asset: MediaAsset, event: DragEvent) {
  if (!event.dataTransfer) {
    return;
  }

  event.dataTransfer.effectAllowed = "copy";
  event.dataTransfer.setData(LINGLUX_ASSET_DRAG_TYPE, asset.id);
  event.dataTransfer.setData(LINGLUX_ASSET_KIND_DRAG_TYPE, asset.type);
  event.dataTransfer.setData("text/plain", asset.id);
}

function handleAssetPointerDown(asset: MediaAsset, event: PointerEvent) {
  if (!event.isPrimary || event.button !== 0 || isAssetDragBlockedTarget(event.target)) {
    return;
  }

  emit("beginAssetDrag", asset.id, event.pointerId, event.clientX, event.clientY);
}

function isAssetDragBlockedTarget(target: EventTarget | null) {
  return target instanceof Element && target.closest("[data-asset-drag-block]") !== null;
}

function handleAssetClick(assetId: string, event: MouseEvent) {
  if (event.shiftKey) {
    emit("selectAssets", getRangedAssetIds(assetId));
    return;
  }

  if (event.metaKey || event.ctrlKey) {
    emit("selectAssets", getToggledAssetIds(assetId));
    return;
  }

  emit("selectAsset", assetId);
}

function getRangedAssetIds(assetId: string) {
  const visibleAssetIds = filteredAssets.value.map((asset) => asset.id);
  const anchorId = props.activeAssetId || currentSelectedAssetIds()[0] || assetId;
  const anchorIndex = visibleAssetIds.indexOf(anchorId);
  const targetIndex = visibleAssetIds.indexOf(assetId);

  if (anchorIndex === -1 || targetIndex === -1) {
    return [assetId];
  }

  const startIndex = Math.min(anchorIndex, targetIndex);
  const endIndex = Math.max(anchorIndex, targetIndex);

  return visibleAssetIds.slice(startIndex, endIndex + 1);
}

function getToggledAssetIds(assetId: string) {
  const selectedIds = new Set(currentSelectedAssetIds());

  if (selectedIds.has(assetId)) {
    selectedIds.delete(assetId);
  } else {
    selectedIds.add(assetId);
  }

  return sortAssetIdsByVisibleOrder([...selectedIds]);
}

function deleteSelectedAssets() {
  const assetIds = selectedDeletableAssetIds.value;

  if (assetIds.length === 0) {
    return;
  }

  if (assetIds.length === 1) {
    emit("deleteAsset", assetIds[0]);
    return;
  }

  emit("deleteAssets", assetIds);
}

function handleAssetKeydown(assetId: string, event: KeyboardEvent) {
  if (event.key === "Enter") {
    emit("selectAsset", assetId);
    return;
  }

  if (event.key !== "Delete" && event.key !== "Backspace") {
    return;
  }

  const asset = props.assets.find((item) => item.id === assetId);
  const selectedAssetIds = selectedDeletableAssetIds.value;

  if (selectedAssetIds.includes(assetId)) {
    event.preventDefault();
    event.stopPropagation();
    deleteSelectedAssets();
    return;
  }

  if (!asset || !isAssetDeletable(asset) || props.activeAssetId !== assetId) {
    return;
  }

  event.preventDefault();
  event.stopPropagation();
  emit("deleteAsset", assetId);
}

function handleAssetPanelKeydown(event: KeyboardEvent) {
  if (event.key !== "Delete" && event.key !== "Backspace") {
    return;
  }

  if (selectedDeletableAssetCount.value === 0) {
    return;
  }

  event.preventDefault();
  event.stopPropagation();
  deleteSelectedAssets();
}

function beginMarqueeSelection(event: PointerEvent) {
  if (!event.isPrimary || event.button !== 0 || isAssetSelectionBlockedTarget(event.target)) {
    return;
  }

  const scroller = assetScroller.value;

  if (!scroller) {
    return;
  }

  scroller.focus({ preventScroll: true });

  const contentPoint = getScrollerContentPoint(scroller, event.clientX, event.clientY);

  cleanupMarqueeSelectionListeners();
  marqueeSelectionState.value = {
    pointerId: event.pointerId,
    startClientX: event.clientX,
    startClientY: event.clientY,
    currentClientX: event.clientX,
    currentClientY: event.clientY,
    startContentX: contentPoint.x,
    startContentY: contentPoint.y,
    currentContentX: contentPoint.x,
    currentContentY: contentPoint.y,
    isActive: false,
    additive: event.shiftKey || event.metaKey || event.ctrlKey,
    baseAssetIds: currentSelectedAssetIds(),
  };

  window.addEventListener("pointermove", updateMarqueeSelection);
  window.addEventListener("pointerup", finishMarqueeSelection);
  window.addEventListener("pointercancel", cancelMarqueeSelection);
}

function isAssetSelectionBlockedTarget(target: EventTarget | null) {
  return (
    target instanceof Element &&
    target.closest("[data-media-asset-id], button, input, textarea, select, label, [data-asset-drag-block]") !== null
  );
}

function updateMarqueeSelection(event: PointerEvent) {
  const state = marqueeSelectionState.value;
  const scroller = assetScroller.value;

  if (!state || event.pointerId !== state.pointerId || !scroller) {
    return;
  }

  const contentPoint = getScrollerContentPoint(scroller, event.clientX, event.clientY);
  const movedDistance = Math.hypot(event.clientX - state.startClientX, event.clientY - state.startClientY);

  state.currentClientX = event.clientX;
  state.currentClientY = event.clientY;
  state.currentContentX = contentPoint.x;
  state.currentContentY = contentPoint.y;

  if (!state.isActive && movedDistance >= 4) {
    state.isActive = true;
  }

  if (!state.isActive) {
    return;
  }

  event.preventDefault();
  emit("selectAssets", resolveMarqueeAssetIds(state));
}

function finishMarqueeSelection(event: PointerEvent) {
  const state = marqueeSelectionState.value;

  if (!state || event.pointerId !== state.pointerId) {
    return;
  }

  if (state.isActive) {
    emit("selectAssets", resolveMarqueeAssetIds(state));
  }

  cleanupMarqueeSelectionListeners();
}

function cancelMarqueeSelection(event?: PointerEvent) {
  const state = marqueeSelectionState.value;

  if (event && state && event.pointerId !== state.pointerId) {
    return;
  }

  cleanupMarqueeSelectionListeners();
}

function cleanupMarqueeSelectionListeners() {
  window.removeEventListener("pointermove", updateMarqueeSelection);
  window.removeEventListener("pointerup", finishMarqueeSelection);
  window.removeEventListener("pointercancel", cancelMarqueeSelection);
  marqueeSelectionState.value = null;
}

function getScrollerContentPoint(scroller: HTMLElement, clientX: number, clientY: number) {
  const bounds = scroller.getBoundingClientRect();

  return {
    x: clientX - bounds.left + scroller.scrollLeft,
    y: clientY - bounds.top + scroller.scrollTop,
  };
}

function resolveMarqueeAssetIds(state: NonNullable<typeof marqueeSelectionState.value>) {
  const hitAssetIds = getMarqueeHitAssetIds(state);

  if (!state.additive) {
    return hitAssetIds;
  }

  return sortAssetIdsByVisibleOrder([...new Set([...state.baseAssetIds, ...hitAssetIds])]);
}

function getMarqueeHitAssetIds(state: NonNullable<typeof marqueeSelectionState.value>) {
  const scroller = assetScroller.value;

  if (!scroller) {
    return [];
  }

  const selectionRect = normalizeClientRect(
    state.startClientX,
    state.startClientY,
    state.currentClientX,
    state.currentClientY,
  );

  return Array.from(scroller.querySelectorAll<HTMLElement>("[data-media-asset-id]"))
    .filter((element) => rectanglesIntersect(selectionRect, element.getBoundingClientRect()))
    .map((element) => element.dataset.mediaAssetId)
    .filter((assetId): assetId is string => Boolean(assetId));
}

function normalizeClientRect(left: number, top: number, right: number, bottom: number) {
  return {
    left: Math.min(left, right),
    top: Math.min(top, bottom),
    right: Math.max(left, right),
    bottom: Math.max(top, bottom),
  };
}

function rectanglesIntersect(
  left: { left: number; top: number; right: number; bottom: number },
  right: { left: number; top: number; right: number; bottom: number },
) {
  return left.left <= right.right && left.right >= right.left && left.top <= right.bottom && left.bottom >= right.top;
}

function sortAssetIdsByVisibleOrder(assetIds: string[]) {
  const selectedIdSet = new Set(assetIds);
  const visibleIds = filteredAssets.value.map((asset) => asset.id);
  const visibleSelectedIds = visibleIds.filter((assetId) => selectedIdSet.has(assetId));
  const hiddenSelectedIds = assetIds.filter((assetId) => !visibleIds.includes(assetId));

  return [...hiddenSelectedIds, ...visibleSelectedIds];
}

function handleTextTemplatePointerDown(preset: TextTemplatePreset, event: PointerEvent) {
  if (!event.isPrimary || event.button !== 0) {
    return;
  }

  cleanupTextTemplatePointerListeners();
  textTemplatePointerState.value = {
    presetId: preset.id,
    pointerId: event.pointerId,
    startX: event.clientX,
    startY: event.clientY,
    didDrag: false,
  };

  emit("beginTextTemplateDrag", preset, event.pointerId, event.clientX, event.clientY);
  window.addEventListener("pointermove", trackTextTemplatePointerMove);
  window.addEventListener("pointerup", finishTextTemplatePointer);
  window.addEventListener("pointercancel", finishTextTemplatePointer);
}

function trackTextTemplatePointerMove(event: PointerEvent) {
  const state = textTemplatePointerState.value;

  if (!state || event.pointerId !== state.pointerId || state.didDrag) {
    return;
  }

  if (Math.hypot(event.clientX - state.startX, event.clientY - state.startY) >= 6) {
    state.didDrag = true;
    suppressedTextTemplateClickId.value = state.presetId;
  }
}

function finishTextTemplatePointer(event: PointerEvent) {
  const state = textTemplatePointerState.value;

  if (!state || event.pointerId !== state.pointerId) {
    return;
  }

  if (state.didDrag) {
    suppressedTextTemplateClickId.value = state.presetId;
    window.setTimeout(() => {
      if (suppressedTextTemplateClickId.value === state.presetId) {
        suppressedTextTemplateClickId.value = null;
      }
    }, 0);
  }

  textTemplatePointerState.value = null;
  cleanupTextTemplatePointerListeners();
}

function cleanupTextTemplatePointerListeners() {
  window.removeEventListener("pointermove", trackTextTemplatePointerMove);
  window.removeEventListener("pointerup", finishTextTemplatePointer);
  window.removeEventListener("pointercancel", finishTextTemplatePointer);
}

function getAudioPreviewContext() {
  if (audioPreviewContext) {
    return audioPreviewContext;
  }

  const AudioContextConstructor = window.AudioContext ?? (window as AudioWindow).webkitAudioContext;

  if (!AudioContextConstructor) {
    return undefined;
  }

  audioPreviewContext = new AudioContextConstructor();
  return audioPreviewContext;
}

function createPreviewVoice(
  context: AudioContext,
  destination: AudioNode,
  options: {
    frequency: number;
    gain: number;
    start: number;
    end: number;
    type: OscillatorType;
  },
) {
  const now = context.currentTime;
  const oscillator = context.createOscillator();
  const gain = context.createGain();

  oscillator.type = options.type;
  oscillator.frequency.setValueAtTime(options.frequency, now + options.start);
  gain.gain.setValueAtTime(0.0001, now + options.start);
  gain.gain.exponentialRampToValueAtTime(options.gain, now + options.start + 0.03);
  gain.gain.exponentialRampToValueAtTime(0.0001, now + options.end);
  oscillator.connect(gain).connect(destination);
  oscillator.start(now + options.start);
  oscillator.stop(now + options.end + 0.02);

  return oscillator;
}

function createAudioPreview(context: AudioContext, preset: AudioTrackPreset) {
  const now = context.currentTime;
  const master = context.createGain();
  const filter = context.createBiquadFilter();
  const sources: AudioScheduledSourceNode[] = [];
  const duration = preset.previewKind === "pulse" ? 1.8 : 2.4;

  master.gain.setValueAtTime(0.0001, now);
  master.gain.exponentialRampToValueAtTime(0.2, now + 0.04);
  master.gain.exponentialRampToValueAtTime(0.0001, now + duration);
  filter.type = "lowpass";
  filter.frequency.setValueAtTime(preset.previewKind === "chiptune" ? 1800 : 1050, now);
  filter.Q.setValueAtTime(preset.previewKind === "pulse" ? 8 : 1.2, now);
  filter.connect(master).connect(context.destination);

  if (preset.previewKind === "lofi") {
    for (const frequency of [220, 277.18, 329.63]) {
      sources.push(createPreviewVoice(context, filter, { frequency, gain: 0.085, start: 0, end: duration, type: "triangle" }));
    }
    sources.push(createPreviewVoice(context, filter, { frequency: 110, gain: 0.055, start: 0.28, end: duration, type: "sine" }));
  }

  if (preset.previewKind === "chiptune") {
    const notes = [523.25, 659.25, 783.99, 1046.5, 783.99, 659.25];
    notes.forEach((frequency, index) => {
      const start = index * 0.24;
      sources.push(createPreviewVoice(context, filter, { frequency, gain: 0.11, start, end: start + 0.2, type: "square" }));
    });
    sources.push(createPreviewVoice(context, filter, { frequency: 130.81, gain: 0.08, start: 0, end: duration, type: "square" }));
  }

  if (preset.previewKind === "pulse") {
    for (const start of [0, 0.3, 0.6, 0.9, 1.2]) {
      sources.push(createPreviewVoice(context, filter, { frequency: 164.81, gain: 0.13, start, end: start + 0.13, type: "sawtooth" }));
      sources.push(createPreviewVoice(context, filter, { frequency: 329.63, gain: 0.075, start: start + 0.06, end: start + 0.17, type: "square" }));
    }
  }

  return {
    durationMs: Math.round(duration * 1000),
    stop: () => {
      for (const source of sources) {
        try {
          source.stop();
        } catch {
          // The source may already have completed.
        }
      }

      filter.disconnect();
      master.disconnect();
    },
  };
}

function stopAudioPreview() {
  if (audioPreviewTimer !== undefined) {
    window.clearTimeout(audioPreviewTimer);
    audioPreviewTimer = undefined;
  }

  audioPreviewCleanup?.();
  audioPreviewCleanup = undefined;
  previewingAudioPresetId.value = null;
}

async function previewAudioPreset(preset: AudioTrackPreset) {
  if (previewingAudioPresetId.value === preset.id) {
    stopAudioPreview();
    return;
  }

  stopAudioPreview();

  previewingAudioPresetId.value = preset.id;
  audioPreviewTimer = window.setTimeout(stopAudioPreview, 900);

  const context = getAudioPreviewContext();

  if (!context) {
    return;
  }

  try {
    if (context.state === "suspended") {
      await context.resume();
    }

    const preview = createAudioPreview(context, preset);

    if (audioPreviewTimer !== undefined) {
      window.clearTimeout(audioPreviewTimer);
    }

    audioPreviewCleanup = preview.stop;
    audioPreviewTimer = window.setTimeout(stopAudioPreview, preview.durationMs);
  } catch {
    // The visible preview state remains active briefly when Web Audio is unavailable.
  }
}

function addAudioPresetToTimeline(preset: AudioTrackPreset) {
  stopAudioPreview();
  emit("addAudioPresetToTimeline", preset);
}

function addTextTemplateToTimeline(preset: TextTemplatePreset) {
  if (suppressedTextTemplateClickId.value === preset.id) {
    suppressedTextTemplateClickId.value = null;
    return;
  }

  emit("addTextTemplateToTimeline", preset);
}
</script>

<template>
  <UiCard
    as="aside"
    variant="subtle"
    class="min-h-0 overflow-hidden rounded-none border-0 border-r border-default bg-muted text-default ring-0"
    :class="compact ? 'border-b max-[900px]:min-h-[360px]' : 'max-[900px]:min-h-[480px]'"
    :ui="{ body: compact ? 'grid h-full min-h-0 overflow-hidden grid-cols-[64px_minmax(0,1fr)] p-0 sm:p-0' : 'grid h-full min-h-0 overflow-hidden grid-cols-[82px_minmax(0,1fr)] p-0 sm:p-0' }"
    aria-label="剪辑器媒体库"
  >
    <nav
      class="grid content-start justify-items-center overflow-visible border-r border-[#20242f] bg-[#0f121a] text-[#8b95a6]"
      :class="compact ? 'gap-1 px-1.5 py-2' : 'gap-2 px-2.5 py-4'"
      aria-label="剪辑工具栏"
    >
      <UiTooltip
        v-for="tab in toolTabs"
        :key="tab.id"
        :text="tab.label"
        :content="{ side: 'right', sideOffset: 8 }"
      >
        <UiButton
          :color="activeTool === tab.id ? 'secondary' : 'neutral'"
          :variant="activeTool === tab.id ? 'solid' : 'ghost'"
          square
          :size="compact ? 'sm' : 'lg'"
          :class="compact ? 'size-9 justify-center' : 'size-11 justify-center'"
          type="button"
          :aria-label="tab.label"
          :aria-current="activeTool === tab.id ? 'page' : undefined"
          @click="activeTool = tab.id"
        >
          <component :is="tab.icon" :size="navigationIconSize" />
        </UiButton>
      </UiTooltip>
    </nav>

    <section
      v-if="activeTool === 'audio'"
      class="editor-scrollbar min-h-0 overflow-y-auto bg-[#15161d]"
      :class="compact ? 'px-2 py-2' : 'px-2 py-2'"
      aria-labelledby="editor-audio-track-title"
    >
      <div class="grid gap-2">
        <header class="flex items-start justify-between gap-2 px-0.5">
          <h2 id="editor-audio-track-title" class="font-black leading-5 text-[#e5e7eb]" :class="compact ? 'text-[13px]' : 'text-[14px]'">高精音频轨</h2>
          <span class="shrink-0 font-mono text-[10px] font-bold leading-5 text-[#60a5fa]">Web Audio 支持</span>
        </header>

        <article class="grid gap-2 rounded-xl border border-[#245449] bg-[#10231f] p-2.5">
          <header class="flex items-center justify-between gap-2">
            <span class="inline-flex items-center gap-1.5 text-[12px] font-black text-[#d8fff4]"><Mic2 :size="14" />AI 配音</span>
            <span class="text-[9px] font-bold text-[#64cdb0]">Kokoro 82M · 本地</span>
          </header>

          <template v-if="!ttsStatus?.supported">
            <p class="text-[10px] leading-4 text-[#94a3b8]">{{ ttsStatus?.error || "仅桌面版 macOS Apple Silicon 支持。" }}</p>
          </template>
          <template v-else-if="ttsStatus.state !== 'ready'">
            <p class="text-[10px] leading-4 text-[#a7b8b3]">安装峰值需约 {{ (ttsStatus.requiredBytes / 1_000_000_000).toFixed(1) }} GB 可用空间，完成并清理缓存后约占 1.5 GB。<template v-if="ttsStatus.legacyBytes"> 将同时清理约 {{ (ttsStatus.legacyBytes / 1_000_000_000).toFixed(1) }} GB 旧版语音文件。</template></p>
            <UiProgress v-if="ttsBusy" :model-value="ttsProgress || 0" size="xs" color="secondary" />
            <p v-if="ttsTaskStatus || ttsError" class="text-[10px]" :class="ttsError ? 'text-[#fca5a5]' : 'text-[#8fd8c5]'">{{ ttsError || ttsTaskStatus }}</p>
            <div class="flex gap-2">
              <UiButton size="xs" color="secondary" type="button" :loading="ttsBusy" :disabled="ttsBusy" @click="emit('setupTts')">{{ ttsStatus.legacyBytes ? '清理旧版并安装' : '安装本地模型' }}</UiButton>
              <UiButton v-if="ttsBusy" size="xs" color="neutral" variant="soft" type="button" @click="emit('cancelTts')">取消</UiButton>
            </div>
          </template>
          <template v-else>
            <label class="grid gap-1 text-[10px] font-bold text-[#a7b8b3]">
              台词
              <UiTextarea v-model="speechText" class="min-h-[72px] resize-y rounded-lg border border-[#2d5148] bg-[#091411] px-2 py-1.5 text-[11px] leading-4 text-[#e5f5f0] outline-none focus:border-[#2dd4a3]" :maxlength="TTS_MAX_TEXT_LENGTH" placeholder="例如：今天天气很好" />
              <span class="text-right font-mono text-[9px]" :class="speechCharacterCount >= TTS_MAX_TEXT_LENGTH ? 'text-[#fca5a5]' : 'text-[#708d85]'">{{ speechCharacterCount }}/{{ TTS_MAX_TEXT_LENGTH }}</span>
            </label>
            <div class="grid grid-cols-2 gap-2">
              <label class="grid gap-1 text-[10px] font-bold text-[#a7b8b3]">音色
                <UiSelect v-model="speechVoice" :items="speechVoiceOptions" size="sm" />
              </label>
              <label class="grid gap-1 text-[10px] font-bold text-[#a7b8b3]">情绪
                <UiSelect v-model="speechEmotion" :items="speechEmotionOptions" size="sm" />
              </label>
            </div>
            <label class="grid gap-1 text-[10px] font-bold text-[#a7b8b3]">语速 {{ speechSpeed.toFixed(2) }}×
              <UiSlider v-model="speechSpeed" :min="0.75" :max="1.5" :step="0.05" />
            </label>
            <UiProgress v-if="ttsBusy" :model-value="ttsProgress || 0" size="xs" color="secondary" />
            <p v-if="ttsError || localSpeechError || ttsTaskStatus" class="text-[10px]" :class="ttsError || localSpeechError ? 'text-[#fca5a5]' : 'text-[#8fd8c5]'">{{ ttsError || localSpeechError || ttsTaskStatus }}</p>
            <div class="flex gap-2">
              <UiButton class="flex-1" size="xs" color="secondary" type="button" :loading="ttsBusy" :disabled="!canGenerateSpeech" @click="submitSpeech">生成并添加</UiButton>
              <UiButton v-if="ttsBusy" size="xs" color="neutral" variant="soft" type="button" @click="emit('cancelTts')">取消</UiButton>
            </div>
          </template>
        </article>

        <article
          v-for="track in audioTrackPresets"
          :key="track.id"
          class="grid gap-1.5 rounded-xl border border-[#2b3040] bg-[#1b1d27] px-2.5 py-1.5 shadow-[0_8px_18px_rgb(0_0_0/0.12)]"
        >
          <header class="grid grid-cols-[minmax(0,1fr)_auto] items-start gap-2">
            <h3 class="min-w-0 truncate text-[12px] font-black leading-4 text-[#d9dde7]">{{ track.title }}</h3>
            <span class="font-mono text-[10px] font-bold leading-4 text-[#858b99]">{{ track.durationLabel }}</span>
          </header>

          <div class="h-2.5 rounded-md bg-[#12141b]" aria-hidden="true"></div>

          <footer class="flex items-center justify-between gap-2">
            <button
              class="inline-flex h-[22px] min-w-[58px] items-center justify-center gap-1 rounded-md bg-[#a3a7b2] px-2 text-[10px] font-black text-white hover:bg-[#b5b9c3]"
              type="button"
              :aria-label="`${previewingAudioPresetId === track.id ? '停止试听' : '试听'} ${track.title}`"
              @click="previewAudioPreset(track)"
            >
              <CirclePlay :size="12" />
              {{ previewingAudioPresetId === track.id ? "试听中" : "试听" }}
            </button>

            <button
              class="inline-flex h-[22px] min-w-[86px] items-center justify-center gap-0.5 rounded bg-[#202638] px-2 text-[10px] font-black text-[#5fa7ff] hover:bg-[#25304a] hover:text-[#77b6ff]"
              type="button"
              :aria-label="`添加 ${track.title} 到时间线`"
              @click="addAudioPresetToTimeline(track)"
            >
              <Plus :size="13" stroke-width="1.7" />
              添加轨道
            </button>
          </footer>
        </article>
      </div>
    </section>

    <section
      v-else-if="activeTool === 'text'"
      class="editor-scrollbar min-h-0 overflow-y-auto bg-[#15161d]"
      :class="compact ? 'px-3 py-4' : 'px-5 py-6'"
      aria-labelledby="editor-text-template-title"
    >
      <header>
        <h2 id="editor-text-template-title" class="font-black leading-5 text-[#f8fafc]" :class="compact ? 'text-[14px]' : 'text-[18px]'">花字和字幕模板</h2>
      </header>

      <div class="mt-4 grid grid-cols-2 gap-3">
        <button
          v-for="preset in textTemplatePresets"
          :key="preset.id"
          class="group grid min-h-[80px] cursor-grab place-items-center rounded-xl border border-[#283044] bg-[#171b25] px-2.5 py-4 text-center outline-none transition hover:border-[#2f6df6] hover:bg-[#1a2030] active:cursor-grabbing focus-visible:ring-2 focus-visible:ring-[#2f6df6]/45"
          type="button"
          draggable="false"
          :aria-label="`添加文字模板 ${preset.title}`"
          @pointerdown="handleTextTemplatePointerDown(preset, $event)"
          @click="addTextTemplateToTimeline(preset)"
          @dragstart.prevent
        >
          <span class="grid justify-items-center gap-2">
            <strong
              class="max-w-full whitespace-nowrap font-black leading-4 tracking-normal"
              :class="[textTemplatePreviewClass(preset), compact ? 'text-[11px]' : 'text-[13px]']"
            >
              {{ preset.title }}
            </strong>
            <span class="text-[10px] font-semibold leading-3 text-[#7b8494] group-hover:text-[#9aa7bc]">{{ preset.subtitle }}</span>
          </span>
        </button>
      </div>
    </section>

    <section v-else class="grid min-h-0" :class="compact ? 'grid-rows-[44px_36px_minmax(0,1fr)]' : 'grid-rows-[74px_56px_minmax(0,1fr)]'">
      <header class="grid grid-cols-[minmax(0,1fr)_auto] items-center border-b border-[#20242f]" :class="compact ? 'gap-1.5 px-2.5' : 'gap-3 px-5'">
        <div class="min-w-0">
          <div class="flex min-w-0 items-center gap-2">
            <h2 class="truncate font-black text-highlighted" :class="compact ? 'text-[13px]' : 'text-[18px]'">{{ activeTabLabel }}</h2>
            <UiBadge color="neutral" variant="subtle" size="sm" class="shrink-0 px-1.5">{{ compact ? assets.length : assetCountLabel }}</UiBadge>
          </div>
          <p class="mt-0.5 truncate font-semibold text-muted" :class="compact ? 'text-[9px]' : 'text-[11px]'">{{ assetPanelDescription }}</p>
        </div>

        <div class="flex items-center" :class="compact ? 'gap-1' : 'gap-2'">
          <UiButton
            :color="viewMode === 'grid' ? 'secondary' : 'neutral'"
            :variant="viewMode === 'grid' ? 'soft' : 'ghost'"
            square
            :size="compact ? 'xs' : 'sm'"
            type="button"
            aria-label="网格视图"
            @click="viewMode = 'grid'"
          >
            <Grid2x2 :size="viewIconSize" />
          </UiButton>
          <UiButton
            :color="viewMode === 'list' ? 'secondary' : 'neutral'"
            :variant="viewMode === 'list' ? 'soft' : 'ghost'"
            square
            :size="compact ? 'xs' : 'sm'"
            type="button"
            aria-label="列表视图"
            @click="viewMode = 'list'"
          >
            <List :size="viewIconSize" />
          </UiButton>
          <UiButton
            v-if="selectedDeletableAssetCount > 1"
            color="error"
            variant="subtle"
            :size="compact ? 'xs' : 'sm'"
            type="button"
            :aria-label="`删除 ${selectedDeletableAssetCount} 个选中素材`"
            @click="deleteSelectedAssets"
          >
            <Trash2 :size="importIconSize" />
            删除 {{ selectedDeletableAssetCount }}
          </UiButton>
          <UiTooltip :text="isTauri() ? '分镜宫格图转视频' : '分镜转视频仅桌面版可用'">
            <UiButton color="neutral" variant="soft" :size="compact ? 'xs' : 'sm'" type="button" :disabled="!isTauri()" aria-label="分镜图转视频" @click="openStoryboardPicker">
              <Clapperboard :size="importIconSize" />
              <span v-if="!compact">分镜转视频</span>
            </UiButton>
          </UiTooltip>
          <UiButton color="secondary" variant="soft" :size="compact ? 'xs' : 'sm'" type="button" aria-label="导入图片序列" @click="openImageSequencePicker">
            <Grid2x2 :size="importIconSize" />
            <span v-if="!compact">图片序列</span>
          </UiButton>
          <UiButton color="secondary" variant="solid" :size="compact ? 'xs' : 'sm'" class="shadow-md shadow-secondary/15" type="button" @click="openFilePicker">
            <Upload :size="importIconSize" />
            导入
          </UiButton>
          <input ref="fileInput" class="sr-only" type="file" multiple accept="video/*,image/*,audio/*,.srt,.vtt,.txt" @change="handleFileInput" />
          <input ref="imageSequenceInput" class="sr-only" type="file" multiple accept="image/*,.jpg,.jpeg,.png,.webp,.gif" @change="handleImageSequenceInput" />
        </div>
      </header>

      <div class="grid items-center border-b border-[#20242f]" :class="compact ? 'px-2.5' : 'px-5'">
        <UiInput
          v-model="searchQuery"
          type="search"
          color="neutral"
          variant="subtle"
          :size="compact ? 'sm' : 'lg'"
          placeholder="搜索资源..."
          aria-label="搜索资源"
          class="w-full"
          :ui="{ base: [compact ? 'text-[11px]' : 'text-[13px]', 'font-semibold'], leading: compact ? 'ps-2' : 'ps-3' }"
        >
          <template #leading>
            <Search :size="compact ? 14 : 16" class="text-muted" />
          </template>
        </UiInput>
      </div>

      <div
        ref="assetScroller"
        class="editor-scrollbar relative min-h-0 select-none overflow-y-auto outline-none focus-visible:ring-2 focus-visible:ring-[#2f6df6]/35"
        :class="compact ? 'px-2.5 py-2.5' : 'px-5 py-4'"
        tabindex="0"
        aria-label="素材选择区域"
        @pointerdown="beginMarqueeSelection"
        @keydown="handleAssetPanelKeydown"
      >
        <UiCard
          v-if="isAssetLibraryEmpty"
          as="button"
          variant="outline"
          class="w-full border-dashed bg-elevated/55 text-center transition hover:border-secondary hover:bg-secondary/5"
          :class="[compact ? 'mb-2.5 min-h-[72px]' : 'mb-4 min-h-[118px]', isImportDragActive ? 'border-secondary bg-secondary/10' : 'border-default']"
          :ui="{ body: 'grid place-items-center p-3' }"
          type="button"
          aria-label="导入资源"
          @click="openFilePicker"
          @dragenter.prevent="isImportDragActive = true"
          @dragover.prevent="isImportDragActive = true"
          @dragleave.prevent="isImportDragActive = false"
          @drop.prevent="handleImportDrop"
        >
          <span class="grid justify-items-center" :class="compact ? 'gap-1.5' : 'gap-2'">
            <span class="grid place-items-center rounded-full bg-secondary/10 text-secondary" :class="compact ? 'size-8' : 'size-11'">
              <Upload :size="uploadIconSize" />
            </span>
            <span>
              <strong class="block font-black text-highlighted" :class="compact ? 'text-[11px]' : 'text-[14px]'">拖拽或点击导入资源</strong>
              <span class="mt-0.5 block font-semibold text-muted" :class="compact ? 'text-[8px]' : 'text-[11px]'">{{ compact ? "视频、图片、音频" : "支持视频、高清图像及主流音频文件" }}</span>
            </span>
          </span>
        </UiCard>

        <p v-if="!compact" class="mb-3 text-[11px] font-bold text-[#687386]">{{ activeTabLabel }} · 双击素材加入时间线，或拖到下方轨道</p>

        <div
          v-if="marqueeSelectionState?.isActive"
          class="pointer-events-none absolute z-30 rounded border border-[#67e8f9] bg-[#0891b2]/18 shadow-[0_0_0_1px_rgb(103_232_249/0.22),0_0_22px_rgb(34_211_238/0.16)]"
          :style="marqueeSelectionStyle"
          aria-hidden="true"
        ></div>

        <div
          v-if="viewMode === 'grid'"
          class="grid gap-2"
          :class="compact ? 'grid-cols-2 max-[900px]:grid-cols-3 max-[640px]:grid-cols-2' : 'grid-cols-4 max-[1220px]:grid-cols-2 max-[900px]:grid-cols-4 max-[640px]:grid-cols-2'"
        >
          <article
            v-for="(asset, index) in filteredAssets"
            :key="asset.id"
            :data-media-asset-id="asset.id"
            class="group relative cursor-grab rounded-lg transition hover:-translate-y-0.5 active:cursor-grabbing"
            draggable="false"
            @pointerdown="handleAssetPointerDown(asset, $event)"
            @dragstart="handleAssetDragStart(asset, $event)"
          >
            <div class="relative">
              <button
                class="block aspect-video w-full overflow-hidden rounded-lg border bg-gradient-to-br text-left outline-none transition hover:border-[#3f7cff] focus-visible:ring-2 focus-visible:ring-[#2f6df6]/55"
                :class="[cardTone(asset, index), gridAssetSelectionClass(asset.id)]"
                type="button"
                :aria-pressed="isAssetSelected(asset.id)"
                :aria-label="`选择素材 ${asset.name}`"
                @click="handleAssetClick(asset.id, $event)"
                @dblclick="$emit('addAssetToTimeline', asset.id)"
                @keydown="handleAssetKeydown(asset.id, $event)"
              >
                <span class="relative grid size-full place-items-center overflow-hidden px-2">
                  <img v-if="assetPreviewUrl(asset)" :src="assetPreviewUrl(asset)" alt="" class="absolute inset-0 size-full object-cover" draggable="false" loading="lazy" />
                  <span v-if="assetPreviewUrl(asset)" class="absolute inset-0 bg-gradient-to-t from-black/35 via-transparent to-black/20" aria-hidden="true"></span>
                  <span class="absolute left-1.5 top-1.5 grid size-5 place-items-center rounded-md bg-black/40 text-white/90">
                    <component :is="assetIcons[asset.type]" :size="11" />
                  </span>
                  <span
                    v-if="isVisualAddedToTimeline(asset)"
                    class="pointer-events-none absolute right-1.5 top-1.5 rounded-md border border-[#5eead4]/35 bg-[#0f2f2d]/90 px-1.5 py-0.5 text-[8px] font-black tracking-wide text-[#6ee7d8] shadow-[0_4px_12px_rgb(0_0_0/0.28)]"
                  >
                    已添加
                  </span>
                  <span
                    v-if="!assetPreviewUrl(asset)"
                    class="flex max-w-full items-baseline justify-center text-center text-[11px] font-black leading-4 text-white"
                    :title="assetDisplayName(asset)"
                  >
                    <template v-for="nameParts in [assetDisplayNameParts(asset)]" :key="`${asset.id}-preview-name`">
                      <span class="min-w-0 truncate">{{ nameParts.head }}</span>
                      <span v-if="nameParts.tail" class="shrink-0">...</span>
                      <span v-if="nameParts.tail" class="shrink-0">{{ nameParts.tail }}</span>
                    </template>
                  </span>
                </span>
              </button>

              <div
                v-if="isAssetDeletable(asset)"
                class="pointer-events-none absolute bottom-1.5 right-1.5 z-10 flex translate-y-1 items-center gap-1 opacity-0 transition duration-150 group-focus-within:pointer-events-auto group-focus-within:translate-y-0 group-focus-within:opacity-100 group-hover:pointer-events-auto group-hover:translate-y-0 group-hover:opacity-100"
                data-asset-drag-block
              >
                <button
                  class="grid size-6 place-items-center rounded-full bg-[#ef4444] text-white shadow-[0_8px_18px_rgb(0_0_0/0.32)] transition hover:bg-[#f87171] focus:outline-none focus:ring-2 focus:ring-white/70"
                  type="button"
                  :title="asset.type === 'audio' ? '删除音频' : asset.type === 'image' ? '删除图片' : '删除视频'"
                  data-asset-drag-block
                  draggable="false"
                  :aria-label="deleteAssetLabel(asset)"
                  @click.stop="$emit('deleteAsset', asset.id)"
                  @dragstart.stop.prevent
                >
                  <Trash2 :size="14" :stroke-width="2.4" />
                </button>

                <button
                  v-if="asset.type === 'video' || asset.type === 'audio' || asset.type === 'image'"
                  class="grid size-6 place-items-center rounded-full bg-[#2f9df5] text-white shadow-[0_8px_18px_rgb(0_0_0/0.32)] transition hover:bg-[#43afff] focus:outline-none focus:ring-2 focus:ring-white/70"
                  type="button"
                  title="添加到时间线"
                  data-asset-drag-block
                  draggable="false"
                  :aria-label="`添加${addAssetTypeLabel(asset)} ${asset.name} 到时间线`"
                  @click.stop="$emit('addAssetToTimeline', asset.id)"
                  @dblclick.stop
                  @dragstart.stop.prevent
                >
                  <Plus :size="14" :stroke-width="2.4" />
                </button>
              </div>
            </div>

            <button
              class="block w-full px-0.5 pb-1 pt-1.5 text-left outline-none"
              type="button"
              :aria-pressed="isAssetSelected(asset.id)"
              :aria-label="`选择素材 ${asset.name}`"
              @click="handleAssetClick(asset.id, $event)"
              @dblclick="$emit('addAssetToTimeline', asset.id)"
              @keydown="handleAssetKeydown(asset.id, $event)"
            >
              <strong class="flex min-w-0 max-w-full items-baseline text-[11px] font-black leading-4 text-[#eef2f7]" :title="asset.name">
                <template v-for="nameParts in [middleEllipsisNameParts(asset.name)]" :key="`${asset.id}-grid-name`">
                  <span class="min-w-0 truncate">{{ nameParts.head }}</span>
                  <span v-if="nameParts.tail" class="shrink-0">...</span>
                  <span v-if="nameParts.tail" class="shrink-0">{{ nameParts.tail }}</span>
                </template>
              </strong>
              <span class="mt-0.5 block truncate text-[9px] font-semibold text-[#687386]">{{ assetMeta(asset) }}</span>
            </button>
          </article>
        </div>

        <div v-else class="grid gap-2">
          <article
            v-for="(asset, index) in filteredAssets"
            :key="asset.id"
            :data-media-asset-id="asset.id"
            class="group grid min-h-[48px] cursor-grab grid-cols-[minmax(0,1fr)_32px] items-stretch gap-1 rounded-lg border bg-[#171b25] p-1 hover:border-[#3f7cff] active:cursor-grabbing"
            :class="listAssetSelectionClass(asset.id)"
            draggable="false"
            @pointerdown="handleAssetPointerDown(asset, $event)"
            @dragstart="handleAssetDragStart(asset, $event)"
          >
            <button
              class="grid min-w-0 grid-cols-[34px_1fr_auto] items-center gap-2 rounded-md px-1 text-left"
              type="button"
              :aria-pressed="isAssetSelected(asset.id)"
              :aria-label="`选择素材 ${asset.name}`"
              @click="handleAssetClick(asset.id, $event)"
              @dblclick="$emit('addAssetToTimeline', asset.id)"
              @keydown="handleAssetKeydown(asset.id, $event)"
            >
              <span class="relative grid size-8 place-items-center overflow-hidden rounded-md bg-gradient-to-br text-white" :class="cardTone(asset, index)">
                <img v-if="assetPreviewUrl(asset)" :src="assetPreviewUrl(asset)" alt="" class="absolute inset-0 size-full object-cover" draggable="false" loading="lazy" />
                <span v-if="assetPreviewUrl(asset)" class="absolute inset-0 bg-black/30" aria-hidden="true"></span>
                <component :is="assetIcons[asset.type]" class="relative" :size="14" />
              </span>
              <span class="min-w-0">
                <strong class="flex min-w-0 max-w-full items-baseline text-[12px] font-black leading-4 text-[#eef2f7]" :title="asset.name">
                  <template v-for="nameParts in [middleEllipsisNameParts(asset.name)]" :key="`${asset.id}-list-name`">
                    <span class="min-w-0 truncate">{{ nameParts.head }}</span>
                    <span v-if="nameParts.tail" class="shrink-0">...</span>
                    <span v-if="nameParts.tail" class="shrink-0">{{ nameParts.tail }}</span>
                  </template>
                </strong>
                <span class="mt-0.5 block truncate text-[10px] font-semibold text-[#687386]">{{ assetMeta(asset) }}</span>
              </span>
              <span class="flex items-center gap-1">
                <span v-if="isVisualAddedToTimeline(asset)" class="rounded border border-[#5eead4]/30 bg-[#0f2f2d] px-1.5 py-0.5 text-[8px] font-black text-[#6ee7d8]">已添加</span>
                <span class="rounded bg-black/35 px-1.5 py-0.5 text-[8px] font-black text-white/80">{{ assetKindLabel(asset.type) }}</span>
              </span>
            </button>

            <button
              v-if="isAssetDeletable(asset)"
              class="grid size-8 place-items-center self-center rounded-full bg-[#ef4444] text-white shadow-[0_8px_18px_rgb(0_0_0/0.24)] transition hover:bg-[#f87171] focus:outline-none focus:ring-2 focus:ring-white/70"
              type="button"
              :title="asset.type === 'audio' ? '删除音频' : asset.type === 'image' ? '删除图片' : '删除视频'"
              data-asset-drag-block
              draggable="false"
              :aria-label="deleteAssetLabel(asset)"
              @click.stop="$emit('deleteAsset', asset.id)"
              @dragstart.stop.prevent
            >
              <Trash2 :size="14" :stroke-width="2.4" />
            </button>
            <span v-else aria-hidden="true"></span>
          </article>
        </div>
      </div>
    </section>
  </UiCard>
</template>
