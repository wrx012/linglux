<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref } from "vue";
import {
  Bookmark,
  Copy,
  Eye,
  EyeOff,
  Link,
  Minus,
  MoveHorizontal,
  Plus,
  Scissors,
  Snowflake,
  Trash2,
  Type as TypeIcon,
  Video,
  VideoOff,
  Volume2,
  VolumeX,
} from "@lucide/vue";
import AudioWaveform from "./AudioWaveform.vue";
import type { AudioBeatMarker, EditorProject, TimelineClip, TimelineTrack, TimelineTrackType } from "../../types/editor";
import {
  formatTimecode,
  isPrimaryTimelineTrack,
  TIMELINE_BASE_PIXELS_PER_SECOND,
  TIMELINE_MAX_SCALE,
  TIMELINE_MIN_SCALE,
} from "../../lib/editorProject";

const LINGLUX_ASSET_DRAG_TYPE = "application/x-linglux-asset";
const CLIP_CONTEXT_MENU_WIDTH = 176;
const CLIP_CONTEXT_MENU_HEIGHT = 86;
const CLIP_CONTEXT_MENU_MARGIN = 8;
const MIN_TIMELINE_WIDTH = 1900;
const RULER_LABEL_MIN_SPACING = 90;
const RULER_MAJOR_INTERVALS = [0.5, 1, 2, 5, 10, 20, 30, 60, 120, 300, 600];
const WHEEL_ZOOM_SENSITIVITY = 0.0016;
const WHEEL_DELTA_LINE_MODE = 1;
const WHEEL_DELTA_PAGE_MODE = 2;
const KEYBOARD_HORIZONTAL_SCROLL_RATIO = 0.62;
const KEYBOARD_HORIZONTAL_SCROLL_MIN = 180;
const KEYBOARD_HORIZONTAL_SCROLL_MAX = 720;
const TIMELINE_VIRTUAL_OVERSCAN_VIEWPORTS = 1;
const SHORTCUT_MODIFIERS: ShortcutModifier[] = ["Control", "Alt", "Shift", "Meta"];

interface ClipContextMenuState {
  clipId: string;
  x: number;
  y: number;
}

interface PlayheadDragState {
  pointerId: number;
  element: HTMLElement;
  lastSeconds: number;
}

interface ClipTrimDragState {
  clipId: string;
  edge: "start" | "end";
  pointerId: number;
  element: HTMLElement;
  startClientX: number;
}

interface RulerTick {
  seconds: number;
  isMajor: boolean;
  isMidpoint: boolean;
  label: string;
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

type ShortcutModifier = "Alt" | "Control" | "Meta" | "Shift";

interface TimelineShortcutBinding {
  key: string;
  modifiers: ShortcutModifier[];
  primaryModifier?: boolean;
}

const props = defineProps<{
  project: EditorProject;
  selectedClipId?: string;
  playhead: number;
  timelineScale: number;
  mainTrackMagnetEnabled: boolean;
  isAssetDragActive: boolean;
  assetDragTargetTrackId?: string;
  assetDragCompatibleTrackIds: string[];
  draggingClipId?: string;
  dragPreview?: TimelineDropPreview;
  trimSnapGuideTime?: number;
  scrollLeftShortcut: TimelineShortcutBinding | null;
  scrollRightShortcut: TimelineShortcutBinding | null;
}>();

const emit = defineEmits<{
  selectClip: [clipId: string];
  beginClipDrag: [clipId: string, pointerId: number, clientX: number, clientY: number, grabOffsetSeconds: number];
  beginPlayheadScrub: [];
  endPlayheadScrub: [seconds: number];
  updatePlayhead: [seconds: number];
  trimClip: [clipId: string, edge: "start" | "end"];
  beginClipTrim: [clipId: string, edge: "start" | "end"];
  updateClipTrim: [clipId: string, edge: "start" | "end", deltaSeconds: number];
  endClipTrim: [clipId: string];
  splitSelected: [];
  deleteSelected: [];
  deleteClip: [clipId: string];
  toggleClipVisibility: [clipId: string];
  setTimelineScale: [scale: number];
  zoomIn: [];
  zoomOut: [];
  toggleAudioBeatMarkers: [];
  alignSelectedVideoToBeatMarkers: [];
  toggleMainTrackMagnet: [];
  dropAsset: [assetId: string, trackId: string, seconds: number];
  updateTrack: [trackId: string, patch: Partial<Pick<TimelineTrack, "muted" | "visible" | "mediaEnabled">>];
}>();

const clipContextMenu = ref<ClipContextMenuState | null>(null);
const timelineScroller = ref<HTMLElement | null>(null);
const timelineViewportWidth = ref(0);
const timelineScrollLeft = ref(0);
const playheadDragState = ref<PlayheadDragState | null>(null);
const clipTrimDragState = ref<ClipTrimDragState | null>(null);
const isPlayheadDragging = computed(() => playheadDragState.value !== null);
const activeSnapGuideTime = computed(() => props.trimSnapGuideTime ?? props.dragPreview?.snapGuideTime);
const isTimelineEmpty = computed(() => !props.project.tracks.some((track) => track.clips.length > 0));
const selectedClipTrackId = computed(() => {
  if (!props.selectedClipId) {
    return "";
  }

  return props.project.tracks.find((track) => track.clips.some((clip) => clip.id === props.selectedClipId))?.id ?? "";
});
const selectedClip = computed(() =>
  props.project.tracks.flatMap((track) => track.clips).find((clip) => clip.id === props.selectedClipId),
);
const canToggleAudioBeatMarkers = computed(() => selectedClip.value?.type === "audio");
const canAlignSelectedVideoToBeatMarkers = computed(() => {
  const clip = selectedClip.value;

  if (!clip || props.project.assets.find((asset) => asset.id === clip.assetId)?.type !== "video") {
    return false;
  }

  const track = props.project.tracks.find((item) => item.id === clip.trackId);

  return !track?.locked && props.project.tracks.some((item) =>
    item.clips.some((candidate) =>
      candidate.type === "audio"
      && candidate.beatMode === "auto"
      && candidate.beatMarkers?.some((marker) =>
        Number.isFinite(marker.time) && marker.time >= 0 && marker.time <= candidate.duration,
      ),
    ),
  );
});
const isSelectedAudioBeatActive = computed(() => {
  const clip = selectedClip.value;

  return clip?.beatMode === "auto" && Boolean(clip.beatMarkers?.length);
});
const displayedTracks = computed(() => {
  const primaryTrack = props.project.tracks.find((track) => track.type === "video");

  if (isTimelineEmpty.value && !props.isAssetDragActive) {
    return primaryTrack ? [primaryTrack] : [];
  }

  const tracksWithContent = props.project.tracks.filter(
    (track) =>
      track.id !== primaryTrack?.id &&
      (track.clips.length > 0 ||
        track.id === selectedClipTrackId.value ||
        track.id === props.assetDragTargetTrackId ||
        (props.isAssetDragActive && props.assetDragCompatibleTrackIds.includes(track.id))),
  );

  return primaryTrack ? [primaryTrack, ...tracksWithContent] : tracksWithContent;
});

const clipContextMenuTarget = computed(() => {
  if (!clipContextMenu.value) {
    return undefined;
  }

  return props.project.tracks.flatMap((track) => track.clips).find((clip) => clip.id === clipContextMenu.value?.clipId);
});
const timelineScalePercent = computed(() => {
  const scaleRatio = (props.timelineScale - TIMELINE_MIN_SCALE) / (TIMELINE_MAX_SCALE - TIMELINE_MIN_SCALE);

  return clamp(Math.round(scaleRatio * 100), 0, 100);
});

function pixelsPerSecond() {
  return TIMELINE_BASE_PIXELS_PER_SECOND * props.timelineScale;
}

const rulerDuration = computed(() =>
  Math.max(
    Math.ceil(props.project.duration),
    isTimelineEmpty.value
      ? timelineViewportWidth.value / pixelsPerSecond()
      : Math.ceil(MIN_TIMELINE_WIDTH / pixelsPerSecond()),
  ),
);

const rulerMajorInterval = computed(() => {
  const interval = RULER_MAJOR_INTERVALS.find(
    (candidate) => candidate * pixelsPerSecond() >= RULER_LABEL_MIN_SPACING,
  );

  return interval ?? RULER_MAJOR_INTERVALS[RULER_MAJOR_INTERVALS.length - 1];
});

const virtualTimeRange = computed(() => {
  const viewportWidth = Math.max(timelineViewportWidth.value, 1);
  const overscan = viewportWidth * TIMELINE_VIRTUAL_OVERSCAN_VIEWPORTS;
  const startPixels = Math.max(0, timelineScrollLeft.value - overscan);
  const endPixels = timelineScrollLeft.value + viewportWidth + overscan;

  return {
    start: startPixels / pixelsPerSecond(),
    end: Math.min(rulerDuration.value, endPixels / pixelsPerSecond()),
  };
});

function formatRulerLabel(seconds: number) {
  const wholeSeconds = Math.max(0, Math.round(seconds));
  const hours = Math.floor(wholeSeconds / 3600);
  const minutes = Math.floor((wholeSeconds % 3600) / 60);
  const remainingSeconds = wholeSeconds % 60;

  if (hours > 0) {
    return [hours, minutes, remainingSeconds]
      .map((value) => value.toString().padStart(2, "0"))
      .join(":");
  }

  return `${minutes.toString().padStart(2, "0")}:${remainingSeconds.toString().padStart(2, "0")}`;
}

const rulerTicks = computed(() => {
  const ticks: RulerTick[] = [];
  const subdivisions = 10;
  const minorInterval = rulerMajorInterval.value / subdivisions;
  const firstIndex = Math.max(0, Math.floor(virtualTimeRange.value.start / minorInterval));
  const lastIndex = Math.min(
    Math.ceil(rulerDuration.value / minorInterval),
    Math.ceil(virtualTimeRange.value.end / minorInterval),
  );

  for (let index = firstIndex; index <= lastIndex; index += 1) {
    const seconds = Number((index * minorInterval).toFixed(4));
    const isMajor = index % subdivisions === 0;

    ticks.push({
      seconds,
      isMajor,
      isMidpoint: !isMajor && index % (subdivisions / 2) === 0,
      label: isMajor ? formatRulerLabel(seconds) : "",
    });
  }

  return ticks;
});

const magneticPreviewClipStarts = computed(() => {
  const starts = new Map<string, number>();
  const preview = props.dragPreview;

  if (!preview?.isCompatible || !props.mainTrackMagnetEnabled) {
    return starts;
  }

  const targetTrack = props.project.tracks.find((track) => track.id === preview.trackId);

  if (!targetTrack || !isPrimaryTimelineTrack(targetTrack)) {
    return starts;
  }

  const clips = targetTrack.clips
    .filter((clip) => clip.id !== props.draggingClipId)
    .sort((left, right) => left.start - right.start);
  const insertionIndex = clips.findIndex(
    (clip) => preview.start < clip.start + clip.duration / 2,
  );
  const orderedItems: Array<TimelineClip | null> = [...clips];

  orderedItems.splice(insertionIndex === -1 ? orderedItems.length : insertionIndex, 0, null);

  let nextStart = 0;

  for (const item of orderedItems) {
    if (item) {
      starts.set(item.id, nextStart);
      nextStart += item.duration;
    } else {
      nextStart += preview.duration;
    }
  }

  return starts;
});

const visibleClipsByTrack = computed(() => {
  const range = virtualTimeRange.value;
  const visible = new Map<string, TimelineClip[]>();

  for (const track of displayedTracks.value) {
    visible.set(
      track.id,
      track.clips.filter((clip) => {
        const effectiveStart = magneticPreviewClipStarts.value.get(clip.id) ?? clip.start;
        const effectiveEnd = effectiveStart + clip.duration;
        const forced = clip.id === props.draggingClipId;

        return forced || (effectiveEnd >= range.start && effectiveStart <= range.end);
      }),
    );
  }

  return visible;
});

function visibleTrackClips(track: TimelineTrack) {
  return visibleClipsByTrack.value.get(track.id) ?? [];
}

function clipStyle(clip: TimelineClip) {
  const pps = pixelsPerSecond();
  const previewStart = magneticPreviewClipStarts.value.get(clip.id);

  return {
    left: `${(previewStart ?? clip.start) * pps}px`,
    width: `${Math.max(clip.duration * pps, 1)}px`,
  };
}

function dropPreviewStyle(preview: TimelineDropPreview) {
  const pps = pixelsPerSecond();

  return {
    left: `${preview.start * pps}px`,
    width: `${Math.max(preview.duration * pps, 2)}px`,
  };
}

function dropPreviewHeightClass(preview: TimelineDropPreview) {
  return preview.clipType === "video" ? "top-1 h-[70px] rounded-md" : "top-2 h-[38px] rounded-lg";
}

function beginClipPointerDrag(clip: TimelineClip, event: PointerEvent) {
  if (event.button !== 0) {
    return;
  }

  const target = event.currentTarget;

  if (!(target instanceof HTMLElement)) {
    return;
  }

  target.focus({ preventScroll: true });
  event.preventDefault();
  event.stopPropagation();
  closeClipContextMenu();
  const bounds = target.getBoundingClientRect();
  const grabOffsetSeconds = clamp((event.clientX - bounds.left) / pixelsPerSecond(), 0, clip.duration);
  emit("beginClipDrag", clip.id, event.pointerId, event.clientX, event.clientY, grabOffsetSeconds);
}

function beginClipTrim(clip: TimelineClip, edge: "start" | "end", event: PointerEvent) {
  if (event.button !== 0) {
    return;
  }

  const target = event.currentTarget;

  if (!(target instanceof HTMLElement)) {
    return;
  }

  event.preventDefault();
  event.stopPropagation();
  closeClipContextMenu();
  emit("selectClip", clip.id);
  target.setPointerCapture(event.pointerId);
  clipTrimDragState.value = {
    clipId: clip.id,
    edge,
    pointerId: event.pointerId,
    element: target,
    startClientX: event.clientX,
  };
  emit("beginClipTrim", clip.id, edge);
  window.addEventListener("pointermove", handleClipTrimPointerMove);
  window.addEventListener("pointerup", endClipTrim);
  window.addEventListener("pointercancel", endClipTrim);
}

function handleClipTrimPointerMove(event: PointerEvent) {
  const drag = clipTrimDragState.value;

  if (!drag || event.pointerId !== drag.pointerId) {
    return;
  }

  event.preventDefault();
  emit("updateClipTrim", drag.clipId, drag.edge, (event.clientX - drag.startClientX) / pixelsPerSecond());
}

function endClipTrim(event?: Event) {
  const drag = clipTrimDragState.value;

  if (event instanceof PointerEvent && drag && event.pointerId !== drag.pointerId) {
    return;
  }

  if (drag?.element.hasPointerCapture(drag.pointerId)) {
    drag.element.releasePointerCapture(drag.pointerId);
  }

  clipTrimDragState.value = null;
  window.removeEventListener("pointermove", handleClipTrimPointerMove);
  window.removeEventListener("pointerup", endClipTrim);
  window.removeEventListener("pointercancel", endClipTrim);

  if (drag) {
    emit("endClipTrim", drag.clipId);
  }
}

function trackWidth() {
  if (isTimelineEmpty.value) {
    return "100%";
  }

  return `${Math.max(rulerDuration.value * pixelsPerSecond(), MIN_TIMELINE_WIDTH)}px`;
}

function clampPlayhead(seconds: number) {
  return clamp(seconds, 0, props.project.duration);
}

function playheadSecondsFromClientX(clientX: number, scroller = timelineScroller.value) {
  if (!scroller) {
    return props.playhead;
  }

  const bounds = scroller.getBoundingClientRect();
  return (clientX - bounds.left + scroller.scrollLeft) / pixelsPerSecond();
}

function updatePlayheadFromClientX(clientX: number) {
  const seconds = clampPlayhead(playheadSecondsFromClientX(clientX));

  if (playheadDragState.value) {
    playheadDragState.value.lastSeconds = seconds;
  }

  emit("updatePlayhead", seconds);
}

function setPlayhead(event: MouseEvent) {
  updatePlayheadFromClientX(event.clientX);
}

function focusTimelineScroller() {
  timelineScroller.value?.focus({ preventScroll: true });
}

async function handleTimelineWheel(event: WheelEvent) {
  if (!event.metaKey && !event.ctrlKey) {
    return;
  }

  const scroller = timelineScroller.value;

  if (!scroller) {
    return;
  }

  event.preventDefault();
  event.stopPropagation();
  closeClipContextMenu();

  const bounds = scroller.getBoundingClientRect();
  const viewportX = clamp(event.clientX - bounds.left, 0, scroller.clientWidth);
  const anchorSeconds = (scroller.scrollLeft + viewportX) / pixelsPerSecond();
  const normalizedDeltaY = normalizedWheelDeltaY(event, scroller);
  const nextScale = clamp(
    Number((props.timelineScale * Math.exp(-normalizedDeltaY * WHEEL_ZOOM_SENSITIVITY)).toFixed(3)),
    TIMELINE_MIN_SCALE,
    TIMELINE_MAX_SCALE,
  );

  if (nextScale === props.timelineScale) {
    return;
  }

  emit("setTimelineScale", nextScale);
  await nextTick();

  const nextPixelsPerSecond = TIMELINE_BASE_PIXELS_PER_SECOND * nextScale;
  const maxScrollLeft = Math.max(0, scroller.scrollWidth - scroller.clientWidth);
  scroller.scrollLeft = clamp(anchorSeconds * nextPixelsPerSecond - viewportX, 0, maxScrollLeft);
}

function normalizedWheelDeltaY(event: WheelEvent, scroller: HTMLElement) {
  if (event.deltaMode === WHEEL_DELTA_PAGE_MODE) {
    return event.deltaY * scroller.clientHeight;
  }

  if (event.deltaMode === WHEEL_DELTA_LINE_MODE) {
    return event.deltaY * 16;
  }

  return event.deltaY;
}

function handleTimelineScrollerKeydown(event: KeyboardEvent) {
  if (event.defaultPrevented || event.isComposing) {
    return;
  }

  const direction = matchesShortcut(event, props.scrollLeftShortcut)
    ? -1
    : matchesShortcut(event, props.scrollRightShortcut)
      ? 1
      : 0;

  if (direction === 0) {
    return;
  }

  const scroller = timelineScroller.value;

  if (!scroller) {
    return;
  }

  event.preventDefault();
  event.stopPropagation();
  closeClipContextMenu();

  const distance = clamp(
    scroller.clientWidth * KEYBOARD_HORIZONTAL_SCROLL_RATIO,
    KEYBOARD_HORIZONTAL_SCROLL_MIN,
    KEYBOARD_HORIZONTAL_SCROLL_MAX,
  );

  scroller.scrollBy({
    left: direction * distance,
    behavior: event.repeat ? "auto" : "smooth",
  });
}

function matchesShortcut(event: KeyboardEvent, binding: TimelineShortcutBinding | null) {
  if (!binding || normalizeShortcutKey(event.key) !== normalizeShortcutKey(binding.key)) {
    return false;
  }

  return bindingModifierSignatures(binding).includes(eventModifierSignature(event));
}

function bindingModifierSignatures(binding: TimelineShortcutBinding) {
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

function normalizeShortcutKey(key: string) {
  return key.length === 1 && key !== " " ? key.toLowerCase() : key;
}

function nudgePlayhead(seconds: number) {
  emit("updatePlayhead", clampPlayhead(props.playhead + seconds));
}

function handlePlayheadKeydown(event: KeyboardEvent) {
  const smallStep = event.shiftKey ? 1 : 0.1;
  const isTimelineScrollShortcut =
    matchesShortcut(event, props.scrollLeftShortcut) || matchesShortcut(event, props.scrollRightShortcut);

  if (event.key === "ArrowLeft") {
    if (isTimelineScrollShortcut) {
      return;
    }

    event.preventDefault();
    nudgePlayhead(-smallStep);
  }

  if (event.key === "ArrowRight") {
    if (isTimelineScrollShortcut) {
      return;
    }

    event.preventDefault();
    nudgePlayhead(smallStep);
  }

  if (event.key === "Home") {
    event.preventDefault();
    emit("updatePlayhead", 0);
  }

  if (event.key === "End") {
    event.preventDefault();
    emit("updatePlayhead", props.project.duration);
  }
}

function handleClipKeydown(clip: TimelineClip, event: KeyboardEvent) {
  if (event.key === "Enter") {
    event.preventDefault();
    event.stopPropagation();
    emit("selectClip", clip.id);
    return;
  }

  if (event.key !== "Delete" && event.key !== "Backspace") {
    return;
  }

  event.preventDefault();
  event.stopPropagation();
  emit("deleteClip", clip.id);
}

function beginPlayheadDrag(event: PointerEvent) {
  if (event.button !== 0) {
    return;
  }

  const target = event.currentTarget;

  if (!(target instanceof HTMLElement)) {
    return;
  }

  event.preventDefault();
  event.stopPropagation();
  closeClipContextMenu();
  target.setPointerCapture(event.pointerId);
  emit("beginPlayheadScrub");

  playheadDragState.value = {
    pointerId: event.pointerId,
    element: target,
    lastSeconds: clampPlayhead(playheadSecondsFromClientX(event.clientX)),
  };

  updatePlayheadFromClientX(event.clientX);
  window.addEventListener("pointermove", handlePlayheadPointerMove);
  window.addEventListener("pointerup", endPlayheadDrag);
  window.addEventListener("pointercancel", endPlayheadDrag);
  window.addEventListener("mouseup", handlePlayheadMouseUp);
}

function handlePlayheadPointerMove(event: PointerEvent) {
  const drag = playheadDragState.value;

  if (!drag || event.pointerId !== drag.pointerId) {
    return;
  }

  event.preventDefault();
  updatePlayheadFromClientX(event.clientX);
}

function handlePlayheadMouseUp() {
  endPlayheadDrag();
}

function endPlayheadDrag(event?: Event) {
  const drag = playheadDragState.value;

  if (event instanceof PointerEvent && drag && event.pointerId !== drag.pointerId) {
    return;
  }

  if (drag?.element.hasPointerCapture(drag.pointerId)) {
    drag.element.releasePointerCapture(drag.pointerId);
  }

  playheadDragState.value = null;
  window.removeEventListener("pointermove", handlePlayheadPointerMove);
  window.removeEventListener("pointerup", endPlayheadDrag);
  window.removeEventListener("pointercancel", endPlayheadDrag);
  window.removeEventListener("mouseup", handlePlayheadMouseUp);

  if (drag) {
    emit("endPlayheadScrub", drag.lastSeconds);
  }
}

function isLingluxAssetDrag(event: DragEvent) {
  return Array.from(event.dataTransfer?.types ?? []).includes(LINGLUX_ASSET_DRAG_TYPE);
}

function handleTrackDragOver(event: DragEvent) {
  if (!isLingluxAssetDrag(event)) {
    return;
  }

  event.preventDefault();

  if (event.dataTransfer) {
    event.dataTransfer.dropEffect = "copy";
  }
}

function dropAssetOnTrack(trackId: string, event: DragEvent) {
  const assetId = event.dataTransfer?.getData(LINGLUX_ASSET_DRAG_TYPE);
  const target = event.currentTarget;

  if (!assetId || !(target instanceof HTMLElement)) {
    return;
  }

  const bounds = target.getBoundingClientRect();
  const seconds = Math.max(0, (event.clientX - bounds.left) / pixelsPerSecond());
  emit("dropAsset", assetId, trackId, seconds);
}

function clipTone(clip: TimelineClip) {
  if (clip.type === "caption" || clip.type === "overlay") {
    return "border-[#8b5cf6] bg-[#2c1b50] text-[#d8b4fe]";
  }

  if (clip.type === "audio") {
    return "border-[#16a34a] bg-[#063b2a] text-[#7df0a6]";
  }

  return "border-[#0c7b82] bg-[#063b40] text-[#d9ffff]";
}

function isClipVisible(clip: TimelineClip) {
  return clip.visible !== false;
}

function isVideoClip(clip: TimelineClip) {
  return clip.type === "video" || clip.type === "overlay";
}

function isTimelineVideoClip(clip: TimelineClip) {
  return clip.type === "video";
}

function trackHeightClass(track: TimelineTrack) {
  if (isTimelineEmpty.value && track.type === "video") {
    return "h-[108px]";
  }

  return track.type === "video" || track.clips.some((clip) => isTimelineVideoClip(clip)) ? "h-[78px]" : "h-[54px]";
}

function emptyTrackLabel(track: TimelineTrack) {
  if (isPrimaryTimelineTrack(track)) {
    return "主轨";
  }

  if (track.type === "audio") {
    return "音频";
  }

  return track.label;
}

function emptyVideoTrackHint(track: TimelineTrack) {
  if (!props.isAssetDragActive) {
    return "拖入视频或图片，开始创作";
  }

  if (props.assetDragCompatibleTrackIds.includes(track.id)) {
    return "松开以添加到主轨";
  }

  const compatibleTrack = displayedTracks.value.find((item) => item.id !== track.id && props.assetDragCompatibleTrackIds.includes(item.id));

  if (compatibleTrack?.type === "audio") {
    return "拖到下方音频轨";
  }

  if (compatibleTrack) {
    return `拖到下方${compatibleTrack.label}`;
  }

  return "此素材不能放入主轨";
}

function emptyAudioTrackHint(track: TimelineTrack) {
  if (props.isAssetDragActive && props.assetDragCompatibleTrackIds.includes(track.id)) {
    return "松开以放到主轨下方";
  }

  return "音频会放在主轨下方";
}

function clipThumbnailUrl(clip: TimelineClip) {
  const asset = clipAsset(clip);

  if (!asset) {
    return "";
  }

  return asset.thumbnailUrl ?? (asset.type === "image" ? asset.url : "");
}

function clipAsset(clip: TimelineClip) {
  return props.project.assets.find((item) => item.id === clip.assetId);
}

function clipFrameStripStyle(clip: TimelineClip) {
  const thumbnailUrl = clipThumbnailUrl(clip);

  if (!thumbnailUrl) {
    return {};
  }

  return {
    backgroundImage: `linear-gradient(to right, transparent calc(100% - 1px), rgb(5 18 21 / 0.72) calc(100% - 1px)), url("${thumbnailUrl}")`,
    backgroundPosition: "left top",
    backgroundRepeat: "repeat-x",
    backgroundSize: "72px 100%",
  };
}

function clipDurationTimecode(clip: TimelineClip) {
  const safeDuration = Math.max(clip.duration, 0);
  const hours = Math.floor(safeDuration / 3600);
  const minutes = Math.floor((safeDuration % 3600) / 60);
  const seconds = Math.floor(safeDuration % 60);
  const frames = Math.floor((safeDuration - Math.floor(safeDuration)) * props.project.fps);

  return [hours, minutes, seconds, frames].map((value) => value.toString().padStart(2, "0")).join(":");
}

function clipPlayedRatio(clip: TimelineClip) {
  if (clip.duration <= 0) {
    return 0;
  }

  return clamp((props.playhead - clip.start) / clip.duration, 0, 1);
}

function audioBeatButtonClass() {
  if (!canToggleAudioBeatMarkers.value) {
    return "cursor-not-allowed text-[#3f4a5e] opacity-55";
  }

  if (isSelectedAudioBeatActive.value) {
    return "bg-[#06343d] text-[#22d3ee] shadow-[inset_0_0_0_1px_rgb(34_211_238/0.22),0_0_14px_rgb(34_211_238/0.12)]";
  }

  return "text-[#aeb7c7] hover:bg-[#202636] hover:text-white";
}

function audioBeatButtonTitle() {
  if (!canToggleAudioBeatMarkers.value) {
    return "选择音频片段后自动卡点";
  }

  return isSelectedAudioBeatActive.value ? "关闭自动卡点" : "自动卡点";
}

function visibleAudioBeatMarkers(clip: TimelineClip): AudioBeatMarker[] {
  if (clip.type !== "audio" || clip.beatMode !== "auto" || clip.duration <= 0) {
    return [];
  }

  return (clip.beatMarkers ?? [])
    .filter((marker) => Number.isFinite(marker.time) && marker.time >= 0 && marker.time <= clip.duration)
    .sort((left, right) => left.time - right.time)
    .slice(0, 160);
}

function audioBeatMarkerStyle(marker: AudioBeatMarker, clip: TimelineClip) {
  return {
    left: `${clamp((marker.time / Math.max(clip.duration, 0.001)) * 100, 0, 100)}%`,
  };
}

function audioBeatMarkerDotStyle(marker: AudioBeatMarker) {
  return {
    opacity: `${clamp(marker.intensity, 0.45, 1)}`,
  };
}

function audioBeatMarkerStemStyle(marker: AudioBeatMarker) {
  return {
    height: `${Math.round(12 + clamp(marker.intensity, 0.25, 1) * 13)}px`,
    opacity: `${clamp(marker.intensity * 0.7, 0.22, 0.72)}`,
  };
}

function trackAccent(trackType: string) {
  if (trackType === "audio") {
    return "text-[#34d399]";
  }

  if (trackType === "caption" || trackType === "overlay") {
    return "text-[#c084fc]";
  }

  return "text-[#60a5fa]";
}

function trackSupportsAudio(type: TimelineTrackType) {
  return type === "audio" || type === "video";
}

function trackSupportsVisibility(type: TimelineTrackType) {
  return type === "video" || type === "overlay" || type === "caption";
}

function trackSupportsMediaOutput(type: TimelineTrackType) {
  return type === "video" || type === "overlay" || type === "caption";
}

function isTrackVisible(track: TimelineTrack) {
  return track.visible !== false;
}

function isTrackMediaEnabled(track: TimelineTrack) {
  return track.mediaEnabled !== false;
}

function trackControlClass(isSupported: boolean, isOn: boolean) {
  if (!isSupported) {
    return "cursor-not-allowed text-[#3f4a5e] opacity-45";
  }

  if (!isOn) {
    return "bg-[#3a1820]/50 text-[#ef4444] hover:bg-[#4a1d24] hover:text-[#fca5a5]";
  }

  return "text-[#8ea0b8] hover:bg-[#202a3a] hover:text-white";
}

function clipTrackStateClass(track: TimelineTrack) {
  if (track.type === "audio" && track.muted) {
    return "opacity-45 saturate-50";
  }

  if (!isTrackVisible(track) || !isTrackMediaEnabled(track)) {
    return "opacity-45 saturate-50";
  }

  return "";
}

function clipStateClass(clip: TimelineClip) {
  return isClipVisible(clip) ? "" : "opacity-40 saturate-50";
}

function toggleTrackAudio(track: TimelineTrack) {
  if (!trackSupportsAudio(track.type)) {
    return;
  }

  emit("updateTrack", track.id, { muted: !track.muted });
}

function toggleTrackVisibility(track: TimelineTrack) {
  if (!trackSupportsVisibility(track.type)) {
    return;
  }

  emit("updateTrack", track.id, { visible: !isTrackVisible(track) });
}

function toggleTrackMediaOutput(track: TimelineTrack) {
  if (!trackSupportsMediaOutput(track.type)) {
    return;
  }

  emit("updateTrack", track.id, { mediaEnabled: !isTrackMediaEnabled(track) });
}

function trackDropClass(trackId: string) {
  if (!props.isAssetDragActive) {
    return "";
  }

  const isCompatible = props.assetDragCompatibleTrackIds.includes(trackId);

  if (props.assetDragTargetTrackId === trackId) {
    return isCompatible
      ? "border-[#2f6df6]/75 bg-[#10264b] shadow-[inset_0_0_0_1px_rgb(47_109_246/0.55),inset_0_0_30px_rgb(47_109_246/0.16)]"
      : "border-[#ef4444]/55 bg-[#2a1218] shadow-[inset_0_0_0_1px_rgb(239_68_68/0.36)]";
  }

  return isCompatible ? "bg-[#101824]" : "bg-[#090d14] opacity-55";
}

function trackLabelDropClass(trackId: string) {
  if (!props.isAssetDragActive) {
    return "";
  }

  const isCompatible = props.assetDragCompatibleTrackIds.includes(trackId);

  if (props.assetDragTargetTrackId === trackId) {
    return isCompatible
      ? "bg-[#132a50] text-[#bfdbfe] shadow-[inset_3px_0_0_rgb(47_109_246)]"
      : "bg-[#351821] text-[#fecaca] shadow-[inset_3px_0_0_rgb(239_68_68)]";
  }

  return isCompatible ? "bg-[#121a28] text-[#9aa7bc]" : "bg-[#0b1018] text-[#536077]";
}

function trackSelectionClass(trackId: string) {
  if (props.isAssetDragActive || selectedClipTrackId.value !== trackId) {
    return "";
  }

  return "border-white/45 bg-white/[0.055] shadow-[inset_3px_0_0_rgb(255_255_255/0.82),inset_0_0_28px_rgb(255_255_255/0.1)]";
}

function trackLabelSelectionClass(trackId: string) {
  if (props.isAssetDragActive || selectedClipTrackId.value !== trackId) {
    return "";
  }

  return "bg-white/[0.08] text-white shadow-[inset_3px_0_0_rgb(255_255_255/0.82)]";
}

function openClipContextMenu(clip: TimelineClip, event: MouseEvent) {
  event.preventDefault();
  event.stopPropagation();
  emit("selectClip", clip.id);

  const maxX = Math.max(CLIP_CONTEXT_MENU_MARGIN, window.innerWidth - CLIP_CONTEXT_MENU_WIDTH - CLIP_CONTEXT_MENU_MARGIN);
  const maxY = Math.max(CLIP_CONTEXT_MENU_MARGIN, window.innerHeight - CLIP_CONTEXT_MENU_HEIGHT - CLIP_CONTEXT_MENU_MARGIN);

  clipContextMenu.value = {
    clipId: clip.id,
    x: clamp(event.clientX, CLIP_CONTEXT_MENU_MARGIN, maxX),
    y: clamp(event.clientY, CLIP_CONTEXT_MENU_MARGIN, maxY),
  };
}

function closeClipContextMenu() {
  clipContextMenu.value = null;
}

function toggleContextMenuClipVisibility() {
  const clipId = clipContextMenu.value?.clipId;

  if (!clipId) {
    return;
  }

  emit("toggleClipVisibility", clipId);
  closeClipContextMenu();
}

function deleteContextMenuClip() {
  const clipId = clipContextMenu.value?.clipId;

  if (!clipId) {
    return;
  }

  emit("deleteClip", clipId);
  closeClipContextMenu();
}

function handleWindowKeydown(event: KeyboardEvent) {
  if (event.key === "Escape") {
    closeClipContextMenu();
  }
}

function clamp(value: number, min: number, max: number) {
  return Math.min(Math.max(value, min), max);
}

let timelineResizeObserver: ResizeObserver | undefined;

function updateTimelineViewportWidth() {
  timelineViewportWidth.value = timelineScroller.value?.clientWidth ?? 0;
  timelineScrollLeft.value = timelineScroller.value?.scrollLeft ?? 0;
}

function handleTimelineScroll() {
  timelineScrollLeft.value = timelineScroller.value?.scrollLeft ?? 0;
}

onMounted(() => {
  updateTimelineViewportWidth();
  timelineResizeObserver = new ResizeObserver(updateTimelineViewportWidth);

  if (timelineScroller.value) {
    timelineResizeObserver.observe(timelineScroller.value);
  }

  window.addEventListener("pointerdown", closeClipContextMenu);
  window.addEventListener("keydown", handleWindowKeydown);
  window.addEventListener("resize", closeClipContextMenu);
});

onUnmounted(() => {
  endPlayheadDrag();
  endClipTrim();
  timelineResizeObserver?.disconnect();
  window.removeEventListener("pointerdown", closeClipContextMenu);
  window.removeEventListener("keydown", handleWindowKeydown);
  window.removeEventListener("resize", closeClipContextMenu);
});
</script>

<template>
  <section
    class="grid min-h-0 grid-rows-[50px_1fr] overflow-hidden border-t border-[#20242f] bg-[#0d1017] text-[#d8deea] transition-[box-shadow,border-color,background-color] duration-150"
    :class="isAssetDragActive ? 'border-[#2f6df6]/70 shadow-[0_-18px_48px_rgb(47_109_246/0.18),inset_0_0_0_1px_rgb(47_109_246/0.28)]' : ''"
    aria-label="多轨时间线"
  >
    <UiDashboardToolbar
      as="header"
      class="border-b border-default bg-elevated/75 px-4 backdrop-blur"
      :ui="{ root: 'grid min-h-[50px] grid-cols-[minmax(260px,1fr)_auto] items-center gap-3 max-[760px]:grid-cols-1 max-[760px]:py-2', left: 'min-w-0', right: 'justify-end' }"
    >
      <template #left>
      <div class="flex min-w-0 flex-wrap items-center gap-2">
        <div class="min-w-0">
          <h2 class="truncate text-[13px] font-black text-highlighted">剪辑时间轴</h2>
        </div>
        <UiBadge color="neutral" variant="subtle" size="sm" class="font-mono">{{ formatTimecode(project.duration) }}</UiBadge>
        <UiSeparator orientation="vertical" class="h-5" />
        <UiButton color="neutral" variant="ghost" square size="xs" type="button" title="分割" aria-label="分割选中片段" @click="emit('splitSelected')">
          <Scissors :size="16" />
        </UiButton>
        <Link :size="16" class="text-[#687386]" />
        <Copy :size="16" class="text-[#687386]" />
        <Snowflake :size="16" class="text-[#687386]" />
        <button
          class="grid size-7 place-items-center rounded-lg transition-[color,background-color,box-shadow,opacity] duration-150"
          :class="audioBeatButtonClass()"
          type="button"
          :title="audioBeatButtonTitle()"
          :aria-label="audioBeatButtonTitle()"
          :aria-pressed="isSelectedAudioBeatActive"
          :disabled="!canToggleAudioBeatMarkers"
          @click="emit('toggleAudioBeatMarkers')"
        >
          <svg class="size-[17px]" viewBox="0 0 24 24" fill="none" aria-hidden="true">
            <path d="M4 15.5c1.6 0 1.5-6 3-6s1.4 6 3 6 1.6-9 3.2-9 1.5 9 3.1 9 1.5-5 3.7-5" stroke="currentColor" stroke-width="1.9" stroke-linecap="round" />
            <path d="M6.5 4.5v4M13 3.5v4M19 5v4" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" />
            <path d="m6.5 4.5 1.45 1.45L6.5 7.4 5.05 5.95 6.5 4.5Zm6.5-1 1.45 1.45L13 6.4l-1.45-1.45L13 3.5ZM19 5l1.45 1.45L19 7.9l-1.45-1.45L19 5Z" fill="currentColor" />
          </svg>
        </button>
        <UiButton
          color="neutral"
          variant="ghost"
          square
          size="xs"
          type="button"
          title="将视频首尾对齐到最近标记点"
          aria-label="将视频首尾对齐到最近标记点"
          :disabled="!canAlignSelectedVideoToBeatMarkers"
          @click="emit('alignSelectedVideoToBeatMarkers')"
        >
          <svg class="size-[17px]" viewBox="0 0 24 24" fill="none" aria-hidden="true">
            <path d="M4 5v14M20 5v14" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" />
            <path d="M8 8.5h8v7H8z" stroke="currentColor" stroke-width="1.8" stroke-linejoin="round" />
            <path d="m10 6.5-2 2 2 2M14 13.5l2 2-2 2" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" />
          </svg>
        </UiButton>
        <UiButton color="error" variant="ghost" square size="xs" type="button" title="删除" aria-label="删除选中片段" @click="emit('deleteSelected')">
          <Trash2 :size="16" />
        </UiButton>
        <Bookmark :size="16" class="text-[#687386]" />
      </div>
      </template>

      <template #right>
      <div class="flex items-center justify-end gap-2 text-toned">
        <UiButton
          :color="mainTrackMagnetEnabled ? 'primary' : 'neutral'"
          :variant="mainTrackMagnetEnabled ? 'soft' : 'ghost'"
          square
          size="sm"
          type="button"
          :title="mainTrackMagnetEnabled ? '关闭主轨磁吸' : '开启主轨磁吸'"
          :aria-label="mainTrackMagnetEnabled ? '关闭主轨磁吸' : '开启主轨磁吸'"
          :aria-pressed="mainTrackMagnetEnabled"
          @click="emit('toggleMainTrackMagnet')"
        >
          <svg class="size-[18px]" viewBox="0 0 24 24" fill="none" aria-hidden="true">
            <path d="M3 7.25h5v9.5H3zM16 7.25h5v9.5h-5" stroke="currentColor" stroke-width="2" stroke-linejoin="round" />
            <path d="M8 12h8M10.75 9.5 8 12l2.75 2.5M13.25 9.5 16 12l-2.75 2.5" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" />
          </svg>
        </UiButton>
        <MoveHorizontal :size="17" />
        <UiButton color="neutral" variant="ghost" square size="xs" type="button" aria-label="缩小时间线" @click="emit('zoomOut')">
          <Minus :size="15" />
        </UiButton>
        <UiProgress :model-value="timelineScalePercent" :max="100" color="secondary" size="sm" class="w-28" :title="`时间线缩放 ${Math.round(timelineScale * 100)}%`" />
        <UiButton color="neutral" variant="ghost" square size="xs" type="button" aria-label="放大时间线" @click="emit('zoomIn')">
          <Plus :size="15" />
        </UiButton>
      </div>
      </template>
    </UiDashboardToolbar>

    <div class="grid min-h-0 grid-cols-[108px_1fr] overflow-hidden">
      <div class="border-r border-[#20242f] bg-[#11151e] pt-8">
        <div
          v-for="track in displayedTracks"
          :key="track.id"
          class="grid grid-cols-3 items-center justify-center gap-1 border-b border-[#20242f] px-3 text-[#8993a3] transition-[height,color,background-color] duration-150"
          :class="[trackHeightClass(track), trackLabelDropClass(track.id), trackLabelSelectionClass(track.id)]"
        >
          <template v-if="!isTimelineEmpty">
          <button
            class="grid size-7 place-items-center rounded-md transition-colors"
            :class="trackControlClass(trackSupportsAudio(track.type), !track.muted)"
            type="button"
            :disabled="!trackSupportsAudio(track.type)"
            :aria-label="`${track.muted ? '取消静音' : '静音'} ${track.label}`"
            @click.stop="toggleTrackAudio(track)"
          >
            <VolumeX v-if="track.muted" :size="16" />
            <Volume2 v-else :size="16" />
          </button>
          <button
            class="grid size-7 place-items-center rounded-md transition-colors"
            :class="trackControlClass(trackSupportsVisibility(track.type), isTrackVisible(track))"
            type="button"
            :disabled="!trackSupportsVisibility(track.type)"
            :aria-label="`${isTrackVisible(track) ? '隐藏' : '显示'} ${track.label}`"
            @click.stop="toggleTrackVisibility(track)"
          >
            <Eye v-if="isTrackVisible(track)" :size="16" />
            <EyeOff v-else :size="16" />
          </button>
          <button
            class="grid size-7 place-items-center rounded-md transition-colors"
            :class="[trackControlClass(trackSupportsMediaOutput(track.type), isTrackMediaEnabled(track)), isTrackMediaEnabled(track) && trackSupportsMediaOutput(track.type) ? trackAccent(track.type) : '']"
            type="button"
            :disabled="!trackSupportsMediaOutput(track.type)"
            :aria-label="`${isTrackMediaEnabled(track) ? '关闭画面输出' : '开启画面输出'} ${track.label}`"
            @click.stop="toggleTrackMediaOutput(track)"
          >
            <TypeIcon v-if="track.type === 'caption'" :size="16" />
            <VideoOff v-else-if="!isTrackMediaEnabled(track)" :size="16" />
            <Video v-else :size="16" />
          </button>
          </template>
          <span v-else class="col-span-3 text-center text-[10px] font-black tracking-[0.08em] text-[#536079]">{{ emptyTrackLabel(track) }}</span>
        </div>
      </div>

      <div
        ref="timelineScroller"
        class="scrollbar-hidden min-w-0 overflow-auto outline-none focus-visible:ring-1 focus-visible:ring-[#2f6df6]/80"
        data-timeline-scroller
        tabindex="0"
        aria-label="时间线时间戳与轨道"
        @pointerdown="focusTimelineScroller"
        @click="setPlayhead"
        @keydown="handleTimelineScrollerKeydown"
        @wheel="handleTimelineWheel"
        @scroll="handleTimelineScroll"
      >
        <div class="relative min-h-full" :style="{ width: trackWidth() }">
          <div
            v-if="activeSnapGuideTime !== undefined"
            class="pointer-events-none absolute bottom-0 top-0 z-[26] w-[2px] -translate-x-1/2 bg-[#67e8f9] shadow-[0_0_12px_rgb(34_211_238/0.82)]"
            :style="{ left: `${activeSnapGuideTime * pixelsPerSecond()}px` }"
            data-timeline-snap-guide
            aria-hidden="true"
          >
            <span class="absolute left-1 top-1 z-10 whitespace-nowrap rounded border border-[#155e75] bg-[#083344]/95 px-1.5 py-0.5 font-mono text-[9px] font-black leading-none text-[#a5f3fc] shadow-lg">
              对齐 · {{ formatTimecode(activeSnapGuideTime) }}
            </span>
          </div>

          <div class="sticky top-0 z-10 h-8 border-b border-[#20242f] bg-[#11151e]">
            <span
              v-for="tick in rulerTicks"
              :key="tick.seconds"
              class="absolute bottom-0 w-px"
              :class="
                tick.isMajor
                  ? 'h-[18px] bg-[#596273]'
                  : tick.isMidpoint
                    ? 'h-3 bg-[#3d4655]'
                    : 'h-2 bg-[#303744]'
              "
              :style="{ left: `${tick.seconds * pixelsPerSecond()}px` }"
            >
              <span
                v-if="tick.isMajor"
                class="absolute left-1.5 top-[-12px] whitespace-nowrap font-mono text-[10px] font-semibold leading-none text-[#788395]"
              >
                {{ tick.label }}
              </span>
            </span>
          </div>

          <div
            class="group absolute bottom-0 top-0 z-30 w-4 -translate-x-1/2 touch-none"
            :class="isPlayheadDragging ? 'cursor-grabbing' : 'cursor-grab'"
            :style="{ left: `${playhead * pixelsPerSecond()}px` }"
            role="slider"
            tabindex="0"
            aria-label="拖动播放头"
            :aria-valuemin="0"
            :aria-valuemax="project.duration"
            :aria-valuenow="Number(playhead.toFixed(2))"
            @pointerdown="beginPlayheadDrag"
            @pointerup="endPlayheadDrag"
            @pointercancel="endPlayheadDrag"
            @lostpointercapture="endPlayheadDrag"
            @keydown="handlePlayheadKeydown"
            @click.stop
          >
            <span
              class="absolute bottom-0 left-1/2 top-0 w-[2px] -translate-x-1/2 transition-[background-color,box-shadow,filter] duration-150"
              :class="
                isPlayheadDragging
                  ? 'bg-[#ff7373] shadow-[0_0_24px_rgb(255_91_91/0.82)] brightness-125'
                  : 'bg-[#ff4b4b] shadow-[0_0_16px_rgb(255_75_75/0.45)] group-hover:bg-[#ff5f5f] group-hover:shadow-[0_0_20px_rgb(255_75_75/0.65)]'
              "
              aria-hidden="true"
            ></span>
            <span
              class="absolute left-1/2 top-8 size-2.5 -translate-x-1/2 rounded-full transition-[background-color,box-shadow,filter] duration-150"
              :class="
                isPlayheadDragging
                  ? 'bg-[#ff7373] shadow-[0_0_18px_rgb(255_91_91/0.86)] brightness-125'
                  : 'bg-[#ff4b4b] group-hover:bg-[#ff5f5f] group-hover:shadow-[0_0_12px_rgb(255_75_75/0.7)]'
              "
              aria-hidden="true"
            ></span>
          </div>

          <div
            v-for="track in displayedTracks"
            :key="track.id"
            class="relative border-b border-[#20242f] bg-[#0d1017] transition-[height,background-color,border-color,box-shadow] duration-150"
            :class="[trackHeightClass(track), trackDropClass(track.id), trackSelectionClass(track.id)]"
            :data-timeline-track-id="track.id"
            @dragenter="handleTrackDragOver"
            @dragover="handleTrackDragOver"
            @drop.prevent="dropAssetOnTrack(track.id, $event)"
          >
            <div
              v-if="isTimelineEmpty && track.type === 'video'"
              class="pointer-events-none absolute inset-0 flex items-center justify-center gap-2 border border-dashed text-[12px] font-bold transition-[border-color,background-color,color,box-shadow] duration-150"
              :class="
                isAssetDragActive && assetDragCompatibleTrackIds.includes(track.id)
                  ? 'border-[#2f6df6]/85 bg-[#15315b]/80 text-[#bfdbfe] shadow-[inset_0_0_24px_rgb(47_109_246/0.16)]'
                  : 'border-[#30394a] bg-[#111722]/70 text-[#718097]'
              "
            >
              <Video :size="15" />
              <span>{{ emptyVideoTrackHint(track) }}</span>
            </div>
            <div
              v-else-if="isTimelineEmpty && track.type === 'audio'"
              class="pointer-events-none absolute inset-0 flex items-center justify-center gap-2 border border-dashed text-[12px] font-bold transition-[border-color,background-color,color,box-shadow] duration-150"
              :class="
                isAssetDragActive && assetDragCompatibleTrackIds.includes(track.id)
                  ? 'border-[#2dd4bf]/85 bg-[#0f2f2d]/80 text-[#99f6e4] shadow-[inset_0_0_22px_rgb(45_212_191/0.14)]'
                  : 'border-[#30394a] bg-[#0f151d]/70 text-[#718097]'
              "
            >
              <Volume2 :size="15" />
              <span>{{ emptyAudioTrackHint(track) }}</span>
            </div>
            <template v-if="dragPreview?.trackId === track.id">
              <div
                class="pointer-events-none absolute bottom-0 top-0 z-20 w-px"
                :class="dragPreview.isCompatible ? 'bg-[#67e8f9] shadow-[0_0_12px_rgb(34_211_238/0.72)]' : 'bg-[#fb7185] shadow-[0_0_12px_rgb(251_113_133/0.58)]'"
                :style="{ left: `${dragPreview.start * pixelsPerSecond()}px` }"
                aria-hidden="true"
              >
                <span
                  class="absolute left-1 top-1 whitespace-nowrap rounded border px-1.5 py-0.5 font-mono text-[9px] font-black leading-none shadow-lg"
                  :class="dragPreview.isCompatible ? 'border-[#155e75] bg-[#083344] text-[#a5f3fc]' : 'border-[#881337] bg-[#4c0519] text-[#fecdd3]'"
                >
                  {{ dragPreview.isCompatible ? `${dragPreview.isSnapped ? "吸附 · " : ""}${formatTimecode(dragPreview.start)}` : "不可放置" }}
                </span>
              </div>
              <div
                class="pointer-events-none absolute z-[15] overflow-hidden border-2 border-dashed transition-[left,width,opacity,background-color,border-color] duration-100"
                :class="[
                  dropPreviewHeightClass(dragPreview),
                  dragPreview.isCompatible
                    ? 'border-[#67e8f9] bg-[#0e7490]/28 shadow-[inset_0_0_18px_rgb(34_211_238/0.18),0_0_16px_rgb(34_211_238/0.16)]'
                    : 'border-[#fb7185] bg-[#881337]/24 shadow-[inset_0_0_18px_rgb(251_113_133/0.12)]',
                ]"
                :style="dropPreviewStyle(dragPreview)"
                aria-hidden="true"
              >
                <span class="flex h-full min-w-0 items-center gap-1.5 px-2 text-[10px] font-black" :class="dragPreview.isCompatible ? 'text-[#cffafe]' : 'text-[#fecdd3]'">
                  <MoveHorizontal :size="13" class="shrink-0" />
                  <span class="truncate">{{ dragPreview.label }}</span>
                </span>
              </div>
            </template>
            <div
              v-for="clip in visibleTrackClips(track)"
              :key="clip.id"
              class="absolute grid touch-none select-none items-center overflow-hidden border text-left text-[12px] shadow-[0_8px_20px_rgb(0_0_0/0.18)] transition-[left,opacity,filter,box-shadow] duration-100"
              :class="[
                clipTone(clip),
                clipTrackStateClass(track),
                clipStateClass(clip),
                isTimelineVideoClip(clip) ? 'top-1 h-[70px] rounded-md' : 'top-2 h-[38px] rounded-lg',
                selectedClipId === clip.id ? 'ring-2 ring-inset ring-white shadow-[0_0_18px_rgb(255_255_255/0.18)]' : '',
                draggingClipId === clip.id ? 'pointer-events-none z-0 cursor-grabbing !opacity-20 saturate-50' : 'z-[5] cursor-grab active:cursor-grabbing',
              ]"
              :style="clipStyle(clip)"
              role="button"
              tabindex="0"
              :aria-label="`选择或拖动片段 ${clip.name}`"
              @pointerdown="beginClipPointerDrag(clip, $event)"
              @click.stop="emit('selectClip', clip.id)"
              @contextmenu.stop.prevent="openClipContextMenu(clip, $event)"
              @keydown="handleClipKeydown(clip, $event)"
            >
              <button
                class="group/trim absolute inset-y-0 left-0 z-20 w-3 cursor-ew-resize touch-none bg-transparent outline-none"
                type="button"
                :aria-label="`拖动裁剪 ${clip.name} 的开头`"
                @pointerdown="beginClipTrim(clip, 'start', $event)"
                @click.stop
              >
                <span class="absolute inset-y-0 left-0 w-[2px] bg-white/0 transition-[background-color,box-shadow] group-hover/trim:bg-[#67e8f9] group-focus-visible/trim:bg-[#67e8f9] group-active/trim:bg-[#67e8f9] group-hover/trim:shadow-[0_0_9px_rgb(34_211_238/0.9)]"></span>
              </button>
              <button
                class="group/trim absolute inset-y-0 right-0 z-20 w-3 cursor-ew-resize touch-none bg-transparent outline-none"
                type="button"
                :aria-label="`拖动裁剪 ${clip.name} 的结尾`"
                @pointerdown="beginClipTrim(clip, 'end', $event)"
                @click.stop
              >
                <span class="absolute inset-y-0 right-0 w-[2px] bg-white/0 transition-[background-color,box-shadow] group-hover/trim:bg-[#67e8f9] group-focus-visible/trim:bg-[#67e8f9] group-active/trim:bg-[#67e8f9] group-hover/trim:shadow-[0_0_9px_rgb(34_211_238/0.9)]"></span>
              </button>
              <template v-if="isTimelineVideoClip(clip)">
                <div class="grid size-full grid-rows-[18px_minmax(0,1fr)_15px] bg-[#042f34]">
                  <header class="flex min-w-0 items-center justify-between gap-2 bg-[#09636a] px-1.5 font-mono text-[9px] font-bold leading-[18px] text-[#d9ffff]">
                    <strong class="min-w-0 truncate font-semibold" :title="clip.name">{{ clip.name }}</strong>
                    <span class="shrink-0 text-[#c9fbff]">{{ clipDurationTimecode(clip) }}</span>
                  </header>

                  <div class="relative overflow-hidden border-y border-[#062a2e] bg-[#0a1619]" :style="clipFrameStripStyle(clip)">
                    <div v-if="!clipThumbnailUrl(clip)" class="absolute inset-0 flex" aria-hidden="true">
                      <span v-for="frame in 16" :key="frame" class="relative w-[72px] shrink-0 border-r border-[#173940] bg-[#10252a]">
                        <span class="absolute inset-x-[18%] top-[24%] h-[38%] rounded-sm bg-[#17616a]"></span>
                        <span class="absolute inset-x-0 bottom-0 h-1.5 bg-black/35"></span>
                      </span>
                    </div>
                  </div>

                  <div class="overflow-hidden bg-[#08727a] px-px py-[2px]" aria-hidden="true">
                    <AudioWaveform
                      :peaks="clipAsset(clip)?.waveformPeaks"
                      :source-duration="clipAsset(clip)?.duration"
                      :trim-start="clip.trimStart"
                      :trim-end="clip.trimEnd"
                      :clip-duration="clip.duration"
                      :speed="clip.speed"
                      :played-ratio="clipPlayedRatio(clip)"
                      color="rgb(68 227 230 / 0.48)"
                      played-color="rgb(153 246 251 / 0.98)"
                      baseline-color="rgb(153 246 251 / 0.2)"
                      :vertical-padding="1"
                    />
                  </div>
                </div>
              </template>
              <template v-else-if="isVideoClip(clip)">
                <div class="size-full bg-[#0f172a] bg-cover bg-center" :style="clipFrameStripStyle(clip)"></div>
                <span class="pointer-events-none absolute left-2 top-1.5 max-w-[70%] truncate text-[11px] font-black text-white drop-shadow">{{ clip.name }}</span>
              </template>
              <template v-else-if="clip.type === 'audio'">
                <div class="relative size-full overflow-hidden bg-[#052e22]">
                  <div class="absolute inset-x-1 bottom-1 top-1">
                    <AudioWaveform
                      :peaks="clipAsset(clip)?.waveformPeaks"
                      :source-duration="clipAsset(clip)?.duration"
                      :trim-start="clip.trimStart"
                      :trim-end="clip.trimEnd"
                      :clip-duration="clip.duration"
                      :speed="clip.speed"
                      :played-ratio="clipPlayedRatio(clip)"
                      color="rgb(110 231 183 / 0.46)"
                      played-color="rgb(209 250 229 / 0.98)"
                      baseline-color="rgb(110 231 183 / 0.22)"
                    />
                  </div>
                  <div v-if="visibleAudioBeatMarkers(clip).length > 0" class="pointer-events-none absolute inset-x-0 top-0 h-full" aria-hidden="true">
                    <span class="absolute inset-x-0 top-0 h-px bg-[#22d3ee] shadow-[0_0_10px_rgb(34_211_238/0.72)]"></span>
                    <span
                      v-for="marker in visibleAudioBeatMarkers(clip)"
                      :key="`${clip.id}-beat-${marker.time}`"
                      class="absolute top-0 h-full w-0"
                      :style="audioBeatMarkerStyle(marker, clip)"
                    >
                      <span
                        class="absolute left-1/2 top-0 size-[6px] -translate-x-1/2 rotate-45 rounded-[1px] bg-[#22d3ee] shadow-[0_0_8px_rgb(34_211_238/0.7)]"
                        :style="audioBeatMarkerDotStyle(marker)"
                      ></span>
                      <span
                        class="absolute left-1/2 top-[5px] w-px -translate-x-1/2 bg-[#22d3ee] shadow-[0_0_6px_rgb(34_211_238/0.44)]"
                        :style="audioBeatMarkerStemStyle(marker)"
                      ></span>
                    </span>
                  </div>
                  <div
                    class="pointer-events-none absolute inset-x-1.5 flex min-w-0 items-center justify-between gap-2 text-[9px] leading-none text-[#d1fae5] drop-shadow-[0_1px_2px_rgb(0_0_0/0.95)]"
                    :class="visibleAudioBeatMarkers(clip).length > 0 ? 'top-2.5' : 'top-1'"
                  >
                    <strong class="min-w-0 truncate font-black" :title="clip.name">{{ clip.name }}</strong>
                    <span class="shrink-0 font-mono font-bold text-[#a7f3d0]">{{ clipDurationTimecode(clip) }}</span>
                  </div>
                </div>
              </template>
              <span v-else class="truncate px-3 font-mono font-black">
                {{ clip.captionText ?? clip.name }}
              </span>
            </div>
          </div>
        </div>
      </div>
    </div>

    <div
      v-if="clipContextMenu && clipContextMenuTarget"
      class="fixed z-[90] w-44 rounded-lg border border-[#2a3142] bg-[#151923] p-1 shadow-[0_18px_44px_rgb(0_0_0/0.45)]"
      :style="{ left: `${clipContextMenu.x}px`, top: `${clipContextMenu.y}px` }"
      role="menu"
      :aria-label="`片段菜单：${clipContextMenuTarget.name}`"
      @pointerdown.stop
      @contextmenu.prevent
    >
      <button
        class="flex h-9 w-full items-center gap-2 rounded-md px-2.5 text-left text-[12px] font-bold text-[#d8deea] hover:bg-[#202a3a] hover:text-white"
        type="button"
        role="menuitem"
        @click="toggleContextMenuClipVisibility"
      >
        <EyeOff v-if="isClipVisible(clipContextMenuTarget)" :size="14" />
        <Eye v-else :size="14" />
        <span class="truncate">{{ isClipVisible(clipContextMenuTarget) ? "隐藏片段" : "显示片段" }}</span>
      </button>
      <button
        class="flex h-9 w-full items-center gap-2 rounded-md px-2.5 text-left text-[12px] font-bold text-[#fca5a5] hover:bg-[#3a1820] hover:text-[#fecaca]"
        type="button"
        role="menuitem"
        @click="deleteContextMenuClip"
      >
        <Trash2 :size="14" />
        <span class="truncate">删除片段</span>
      </button>
    </div>
  </section>
</template>
