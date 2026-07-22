<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, reactive, ref, watch } from "vue";
import { ChevronDown, Maximize2, Pause, Play, Video } from "@lucide/vue";
import type { EditorProject, MediaAsset, TimelineClip, TimelineTrack } from "../../types/editor";
import { createDefaultTextClipStyle, formatTimecode } from "../../lib/editorProject";

const props = defineProps<{
  project: EditorProject;
  selectedClip?: TimelineClip;
  previewAsset?: MediaAsset;
  playhead: number;
  isPlaying: boolean;
  isPlayheadScrubbing?: boolean;
}>();

const emit = defineEmits<{
  togglePlayback: [];
  previewClockState: [isActive: boolean];
  previewEnded: [];
  previewPlayhead: [seconds: number];
  selectCaptionClip: [clipId: string];
}>();

const stageRef = ref<HTMLElement | null>(null);
const previewVideoRef = ref<HTMLVideoElement | null>(null);
const hasPreviewError = ref(false);
const standalonePreviewTime = ref(0);
const frameBounds = reactive({
  width: 0,
  height: 0,
});

let previewResizeObserver: ResizeObserver | undefined;
let previewProgressFrameId: number | undefined;
const timelineAudioElements = new Map<string, HTMLAudioElement>();
const timelineAudioLastCorrectionMs = new Map<string, number>();
let lastPreviewPlayheadEmitMs = 0;
let lastNativePlayheadSeconds = 0;
let lastTimelineAudioPlayheadSeconds = 0;
let lastPreviewClockState: boolean | undefined;
let lastPlayRequestKey = "";

const PREVIEW_PLAYHEAD_EMIT_INTERVAL_MS = 1000 / 60;
const PREVIEW_DRIFT_SEEK_SECONDS = 0.75;
const PREVIEW_MIN_SEEK_SECONDS = 0.05;
const AUDIO_DRIFT_SEEK_SECONDS = 0.45;
const AUDIO_DRIFT_CORRECTION_INTERVAL_MS = 450;
const AUDIO_PLAYHEAD_JUMP_SECONDS = 0.28;
const AUDIO_SCRUB_SEEK_INTERVAL_MS = 120;

interface TimelinePreviewSource {
  mode: "timeline";
  asset: MediaAsset;
  clip: TimelineClip;
  track: TimelineTrack;
}

interface AssetPreviewSource {
  mode: "asset";
  asset: MediaAsset;
}

type PreviewSource = TimelinePreviewSource | AssetPreviewSource;

interface CaptionOverlay {
  id: string;
  text: string;
  clip: TimelineClip;
}

interface TimelineAudioSource {
  key: string;
  asset: MediaAsset;
  clip: TimelineClip;
  track: TimelineTrack;
}

const hasTimelineContent = computed(() => props.project.tracks.some((track) => track.clips.length > 0));
const canPlayback = computed(() => hasTimelineContent.value && props.project.duration > 0);

const activePreviewSource = computed<PreviewSource | undefined>(() => {
  if (!hasTimelineContent.value) {
    return undefined;
  }

  if (props.previewAsset?.type === "video" || props.previewAsset?.type === "image") {
    return { mode: "asset", asset: props.previewAsset };
  }

  for (const track of props.project.tracks) {
    if (!isTrackPreviewable(track)) {
      continue;
    }

    const clip = track.clips.find((item) => isClipAtPlayhead(item) && isClipPreviewable(item));

    if (!clip) {
      continue;
    }

    const asset = props.project.assets.find((item) => item.id === clip.assetId);

    if (asset?.type === "video" || asset?.type === "image") {
      return { mode: "timeline", asset, clip, track };
    }
  }

  return undefined;
});

const activeCaptionOverlays = computed<CaptionOverlay[]>(() => {
  if (activePreviewSource.value?.mode === "asset") {
    return [];
  }

  const overlays: CaptionOverlay[] = [];

  for (const track of props.project.tracks) {
    if (!isTrackPreviewable(track)) {
      continue;
    }

    for (const clip of track.clips) {
      if (clip.type !== "caption" || !isClipAtPlayhead(clip) || !isClipPreviewable(clip)) {
        continue;
      }

      const text = (clip.captionText ?? clip.name).trim();

      if (!text) {
        continue;
      }

      overlays.push({
        id: `${track.id}:${clip.id}`,
        text,
        clip,
      });
    }
  }

  return overlays;
});

const activeAudioSources = computed<TimelineAudioSource[]>(() => {
  if (activePreviewSource.value?.mode === "asset") {
    return [];
  }

  const sources: TimelineAudioSource[] = [];

  for (const track of props.project.tracks) {
    if (!isTrackAudioPlayable(track)) {
      continue;
    }

    for (const clip of track.clips) {
      if (clip.muted || !isClipAtPlayhead(clip) || !isClipPreviewable(clip)) {
        continue;
      }

      const asset = props.project.assets.find((item) => item.id === clip.assetId);

      if (asset?.type !== "audio") {
        continue;
      }

      sources.push({
        key: `${track.id}:${clip.id}:${asset.id}`,
        asset,
        clip,
        track,
      });
    }
  }

  return sources;
});

const activeAudioPlaybackKey = computed(() =>
  activeAudioSources.value.map((source) => `${source.key}:${source.asset.url}`).join("|"),
);

const activeAudioSettingsKey = computed(() =>
  activeAudioSources.value
    .map(
      (source) =>
        `${source.key}:${source.clip.volume}:${source.clip.muted}:${source.clip.trimStart}:${source.clip.trimEnd}:${source.clip.speed}:${source.track.muted}:${source.track.mediaEnabled}`,
    )
    .join("|"),
);

const activePreviewKey = computed(() => {
  const source = activePreviewSource.value;

  if (!source) {
    return "";
  }

  if (source.mode === "asset") {
    return `asset:${source.asset.id}:${source.asset.url}`;
  }

  return `${source.track.id}:${source.clip.id}:${source.asset.id}:${source.asset.url}`;
});

const activePreviewTime = computed(() => {
  const source = activePreviewSource.value;

  if (!source) {
    return 0;
  }

  if (source.mode === "asset") {
    return standalonePreviewTime.value;
  }

  return Math.max(0, props.playhead - source.clip.start + source.clip.trimStart);
});

const previewDisplayTime = computed(() => {
  if (!hasTimelineContent.value) {
    return 0;
  }

  return activePreviewSource.value?.mode === "asset" ? standalonePreviewTime.value : props.playhead;
});
const previewDisplayDuration = computed(() => {
  if (!hasTimelineContent.value) {
    return 0;
  }

  const source = activePreviewSource.value;

  if (source?.mode === "asset") {
    return source.asset.duration;
  }

  return props.project.duration;
});
const activePreviewName = computed(() => {
  const source = activePreviewSource.value;

  return source?.mode === "asset" ? source.asset.name : source?.clip.name ?? "";
});
const activePreviewResolution = computed(() => {
  const asset = activePreviewSource.value?.asset;

  return {
    width: asset?.width ?? props.project.resolution.width,
    height: asset?.height ?? props.project.resolution.height,
  };
});

const activePreviewVolume = computed(() => {
  const source = activePreviewSource.value;

  if (!source) {
    return 1;
  }

  if (source.mode === "asset") {
    return 1;
  }

  return clamp(source.clip.volume, 0, 1);
});

const isPreviewMuted = computed(() => {
  const source = activePreviewSource.value;

  if (!source) {
    return true;
  }

  return source.mode === "timeline" && (source.clip.muted || source.track.muted);
});

const hasNativeVideoClock = computed(() => {
  const source = activePreviewSource.value;

  return Boolean(source && source.asset.type === "video" && !hasPreviewError.value);
});

const previewFrameStyle = computed(() => {
  if (frameBounds.width <= 0 || frameBounds.height <= 0) {
    return {};
  }

  return {
    width: `${Math.round(frameBounds.width)}px`,
    height: `${Math.round(frameBounds.height)}px`,
  };
});

onMounted(() => {
  updatePreviewFrameBounds();
  emitPreviewClockState();

  previewResizeObserver = new ResizeObserver(updatePreviewFrameBounds);

  if (stageRef.value) {
    previewResizeObserver.observe(stageRef.value);
  }

  window.addEventListener("resize", updatePreviewFrameBounds);
});

onUnmounted(() => {
  previewVideoRef.value?.pause();
  stopTimelineAudio();
  stopNativeVideoClock();
  previewResizeObserver?.disconnect();
  window.removeEventListener("resize", updatePreviewFrameBounds);
});

watch(
  activePreviewKey,
  () => {
    hasPreviewError.value = false;
    standalonePreviewTime.value = 0;
    lastNativePlayheadSeconds = props.playhead;
    lastPlayRequestKey = "";
    stopNativeVideoClock();
    void nextTick(() => syncPreviewVideo({ forceSeek: true }));
  },
  { flush: "post" },
);

watch(
  hasNativeVideoClock,
  () => {
    emitPreviewClockState();

    if (!hasNativeVideoClock.value) {
      stopNativeVideoClock();
    }
  },
  { flush: "post" },
);

watch(
  () => props.isPlaying,
  (isPlaying) => {
    if (isPlaying) {
      lastNativePlayheadSeconds = props.playhead;
      lastTimelineAudioPlayheadSeconds = props.playhead;
    }

    syncTimelineAudio({ forceSeek: true });
    void nextTick(() => syncPreviewVideo({ forceSeek: true }));
  },
  { flush: "post" },
);

watch(
  () => props.playhead,
  () => {
    const isScrubbing = props.isPlayheadScrubbing === true;
    const playheadDelta = Math.abs(props.playhead - lastTimelineAudioPlayheadSeconds);
    const isTimelineAudioJump = !props.isPlaying || playheadDelta > AUDIO_PLAYHEAD_JUMP_SECONDS;

    if (isScrubbing && props.isPlaying) {
      lastNativePlayheadSeconds = props.playhead;
      stopNativeVideoClock();
    } else if (!props.isPlaying || Math.abs(props.playhead - lastNativePlayheadSeconds) > 0.18) {
      lastNativePlayheadSeconds = props.playhead;
      stopNativeVideoClock();
      void nextTick(() => syncPreviewVideo({ forceSeek: true }));
    }

    lastTimelineAudioPlayheadSeconds = props.playhead;
    syncTimelineAudio({
      forceSeek: isScrubbing ? true : isTimelineAudioJump,
      seekThrottleMs: isScrubbing && props.isPlaying ? AUDIO_SCRUB_SEEK_INTERVAL_MS : undefined,
      allowDriftCorrection: props.isPlaying && !isTimelineAudioJump && !isScrubbing,
    });
  },
  { flush: "post" },
);

watch(
  () => props.isPlayheadScrubbing,
  (isScrubbing, wasScrubbing) => {
    if (isScrubbing) {
      lastNativePlayheadSeconds = props.playhead;
      lastTimelineAudioPlayheadSeconds = props.playhead;
      return;
    }

    if (!wasScrubbing) {
      return;
    }

    lastNativePlayheadSeconds = props.playhead;
    lastTimelineAudioPlayheadSeconds = props.playhead;
    syncTimelineAudio({ forceSeek: true });
    void nextTick(() => syncPreviewVideo({ forceSeek: true }));
  },
  { flush: "post" },
);

watch(
  () => [isPreviewMuted.value, activePreviewVolume.value],
  () => {
    applyPreviewVideoSettings();
  },
  { flush: "post" },
);

watch(
  activeAudioPlaybackKey,
  () => {
    syncTimelineAudio({ forceSeek: true });
  },
  { flush: "post" },
);

watch(
  activeAudioSettingsKey,
  () => {
    syncTimelineAudio({ forceSeek: !props.isPlaying });
  },
  { flush: "post" },
);

function updatePreviewFrameBounds() {
  const stage = stageRef.value;

  if (!stage) {
    return;
  }

  const style = window.getComputedStyle(stage);
  const horizontalPadding = Number.parseFloat(style.paddingLeft) + Number.parseFloat(style.paddingRight);
  const verticalPadding = Number.parseFloat(style.paddingTop) + Number.parseFloat(style.paddingBottom);
  const availableWidth = Math.max(stage.clientWidth - horizontalPadding, 220);
  const availableHeight = Math.max(stage.clientHeight - verticalPadding, 160);
  const maxWidth = Math.min(availableWidth * 0.92, 980);
  const maxHeight = availableHeight;
  let width = maxWidth;
  let height = width * 9 / 16;

  if (height > maxHeight) {
    height = maxHeight;
    width = height * 16 / 9;
  }

  frameBounds.width = width;
  frameBounds.height = height;
}

function isTrackPreviewable(track: TimelineTrack) {
  return track.visible !== false && track.mediaEnabled !== false;
}

function isTrackAudioPlayable(track: TimelineTrack) {
  return track.mediaEnabled !== false && track.muted !== true;
}

function isClipPreviewable(clip: TimelineClip) {
  return clip.visible !== false;
}

function isClipAtPlayhead(clip: TimelineClip) {
  const clipEnd = clip.start + clip.duration;

  if (props.playhead >= clip.start && props.playhead < clipEnd) {
    return true;
  }

  const isPausedAtProjectEnd =
    !props.isPlaying &&
    props.project.duration > 0 &&
    Math.abs(props.playhead - props.project.duration) < PREVIEW_MIN_SEEK_SECONDS;

  return isPausedAtProjectEnd && Math.abs(clipEnd - props.project.duration) < PREVIEW_MIN_SEEK_SECONDS;
}

function captionOverlayStyle(overlay: CaptionOverlay, index: number) {
  const transform = overlay.clip.transform;
  const style = {
    ...createDefaultTextClipStyle(),
    ...overlay.clip.textStyle,
  };
  const x = Number.isFinite(transform.x) ? transform.x : 0;
  const y = Number.isFinite(transform.y) ? transform.y : 0;
  const scale = Number.isFinite(transform.scale) ? transform.scale : 1;
  const rotation = Number.isFinite(transform.rotation) ? transform.rotation : 0;
  const opacity = Number.isFinite(transform.opacity) ? clamp(transform.opacity, 0, 1) : 1;
  const stackedOffset = index * 42;
  const scaledFontSize = clamp(style.fontSize, 12, 180);

  return {
    opacity,
    transform: `translate(-50%, -50%) translate(${x}px, ${y + stackedOffset}px) rotate(${rotation}deg) scale(${scale})`,
    fontFamily: style.fontFamily,
    fontSize: `${scaledFontSize}px`,
    color: safeColorValue(style.color, "#FFFFFF"),
    letterSpacing: `${style.letterSpacing}px`,
    lineHeight: `${style.lineHeight}`,
  };
}

function captionOverlayBackgroundStyle(overlay: CaptionOverlay) {
  const style = {
    ...createDefaultTextClipStyle(),
    ...overlay.clip.textStyle,
  };
  const fontSize = clamp(style.fontSize, 12, 180);
  const lines = overlay.text.split("\n");
  const longestLineLength = lines.reduce((max, line) => Math.max(max, line.length), 0);
  const estimatedTextWidth = longestLineLength * (fontSize * 0.62 + Math.max(style.letterSpacing, 0)) + 32;
  const estimatedTextHeight = Math.max(lines.length, 1) * fontSize * style.lineHeight + 16;
  const width = Math.max(style.backgroundWidth, estimatedTextWidth);
  const height = Math.max(style.backgroundHeight, estimatedTextHeight);

  return {
    width: `${width}px`,
    height: `${height}px`,
    backgroundColor: safeColorValue(style.backgroundColor, "#000000"),
    borderRadius: `${Math.max(style.backgroundCornerRadius, 0)}px`,
    transform: `translate(-50%, -50%) translate(${style.backgroundXOffset}px, ${style.backgroundYOffset}px)`,
  };
}

function safeColorValue(color: string, fallback: string) {
  return /^#[0-9a-f]{6}$/i.test(color) ? color : fallback;
}

function syncPreviewVideo(options: { forceSeek?: boolean } = {}) {
  const video = previewVideoRef.value;
  const source = activePreviewSource.value;

  if (!video || !source || source.asset.type !== "video" || hasPreviewError.value) {
    stopNativeVideoClock();
    emitPreviewClockState();
    return;
  }

  emitPreviewClockState();
  applyPreviewVideoSettings();

  const duration = Number.isFinite(video.duration) && video.duration > 0 ? video.duration : source.asset.duration;
  const targetTime = clamp(activePreviewTime.value, 0, Math.max(duration - 0.02, 0));
  const drift = Math.abs(video.currentTime - targetTime);
  const shouldSeek = options.forceSeek || !props.isPlaying || drift > PREVIEW_DRIFT_SEEK_SECONDS;

  if (shouldSeek && drift > PREVIEW_MIN_SEEK_SECONDS) {
    stopNativeVideoClock();
    seekPreviewVideo(video, targetTime);
  }

  if (props.isPlaying) {
    requestPreviewVideoPlay(video);
  } else {
    lastPlayRequestKey = "";
    video.pause();
    stopNativeVideoClock();
  }
}

function applyPreviewVideoSettings() {
  const video = previewVideoRef.value;

  if (!video) {
    return;
  }

  video.muted = isPreviewMuted.value;
  video.volume = activePreviewVolume.value;
}

function requestPreviewVideoPlay(video: HTMLVideoElement) {
  const playRequestKey = activePreviewKey.value;

  if (!video.paused && lastPlayRequestKey === playRequestKey) {
    startNativeVideoClock();
    return;
  }

  lastPlayRequestKey = playRequestKey;

  void video
    .play()
    .then(() => startNativeVideoClock())
    .catch(() => {
      // Keep the current frame visible if playback is blocked by the browser.
    });
}

function seekPreviewVideo(video: HTMLVideoElement, seconds: number) {
  try {
    video.currentTime = seconds;
  } catch {
    // Some videos reject seeking until metadata is fully ready.
  }
}

function startNativeVideoClock() {
  const video = previewVideoRef.value;

  if (
    previewProgressFrameId !== undefined ||
    !props.isPlaying ||
    !hasNativeVideoClock.value ||
    !video ||
    video.paused ||
    video.seeking ||
    video.readyState < 2
  ) {
    return;
  }

  previewProgressFrameId = window.requestAnimationFrame(stepNativeVideoClock);
}

function stopNativeVideoClock() {
  if (previewProgressFrameId !== undefined) {
    window.cancelAnimationFrame(previewProgressFrameId);
  }

  previewProgressFrameId = undefined;
  lastPreviewPlayheadEmitMs = 0;
}

function stepNativeVideoClock(timestamp: number) {
  previewProgressFrameId = undefined;
  const video = previewVideoRef.value;

  if (!props.isPlaying || !hasNativeVideoClock.value || !video || video.paused || video.seeking || video.readyState < 2) {
    return;
  }

  emitNativeVideoPlayhead(timestamp);
  previewProgressFrameId = window.requestAnimationFrame(stepNativeVideoClock);
}

function emitNativeVideoPlayhead(timestamp: number) {
  const video = previewVideoRef.value;
  const source = activePreviewSource.value;

  if (!video || !source || source.asset.type !== "video") {
    return;
  }

  if (source.mode === "asset") {
    const duration = Number.isFinite(video.duration) && video.duration > 0 ? video.duration : source.asset.duration;
    standalonePreviewTime.value = clamp(video.currentTime, 0, duration);
    return;
  }

  const clipStart = source.clip.start;
  const clipEnd = source.clip.start + source.clip.duration;
  const nativeTimelineTime = clamp(source.clip.start + video.currentTime - source.clip.trimStart, clipStart, clipEnd);
  const timelineTime = Math.max(nativeTimelineTime, lastNativePlayheadSeconds);
  const shouldEmit = timestamp - lastPreviewPlayheadEmitMs >= PREVIEW_PLAYHEAD_EMIT_INTERVAL_MS || timelineTime >= clipEnd - 0.02;

  if (!shouldEmit || timelineTime <= lastNativePlayheadSeconds) {
    return;
  }

  lastPreviewPlayheadEmitMs = timestamp;
  lastNativePlayheadSeconds = timelineTime;
  emit("previewPlayhead", timelineTime);

  if (timelineTime >= props.project.duration - 0.02) {
    emit("previewEnded");
  }
}

function handleNativeVideoEnded() {
  const source = activePreviewSource.value;

  if (!source) {
    return;
  }

  stopNativeVideoClock();

  if (source.mode === "timeline") {
    const clipEnd = Math.min(source.clip.start + source.clip.duration, props.project.duration);
    lastNativePlayheadSeconds = Math.max(lastNativePlayheadSeconds, clipEnd);
    emit("previewPlayhead", clipEnd);
    return;
  }

  const video = previewVideoRef.value;
  const duration = video && Number.isFinite(video.duration) && video.duration > 0 ? video.duration : source.asset.duration;
  standalonePreviewTime.value = duration;
  emit("previewEnded");
}

function handleNativeVideoPlaying() {
  startNativeVideoClock();
}

function handleNativeVideoClockSuspended() {
  stopNativeVideoClock();
}

function handleNativeVideoSeeked() {
  if (props.isPlaying) {
    startNativeVideoClock();
  }
}

function emitPreviewClockState() {
  const isActive = hasNativeVideoClock.value;

  if (lastPreviewClockState === isActive) {
    return;
  }

  lastPreviewClockState = isActive;
  emit("previewClockState", isActive);
}

function handlePreviewError() {
  hasPreviewError.value = true;
  stopNativeVideoClock();
  emitPreviewClockState();
}

function syncTimelineAudio(options: { forceSeek?: boolean; allowDriftCorrection?: boolean; seekThrottleMs?: number } = {}) {
  const activeKeys = new Set<string>();

  for (const source of activeAudioSources.value) {
    activeKeys.add(source.key);
    const audio = getTimelineAudioElement(source);

    applyTimelineAudioSettings(audio, source);
    syncTimelineAudioTime(audio, source, options);

    if (props.isPlaying) {
      requestTimelineAudioPlay(audio);
    } else {
      audio.pause();
    }
  }

  for (const [key, audio] of timelineAudioElements) {
    if (activeKeys.has(key)) {
      continue;
    }

    audio.pause();
    audio.removeAttribute("src");
    audio.load();
    timelineAudioElements.delete(key);
    timelineAudioLastCorrectionMs.delete(key);
  }
}

function getTimelineAudioElement(source: TimelineAudioSource) {
  const existingAudio = timelineAudioElements.get(source.key);

  if (existingAudio) {
    if (existingAudio.getAttribute("src") !== source.asset.url) {
      existingAudio.pause();
      existingAudio.src = source.asset.url;
      existingAudio.load();
      timelineAudioLastCorrectionMs.delete(source.key);
    }

    return existingAudio;
  }

  const audio = new Audio(source.asset.url);

  audio.preload = "auto";
  audio.addEventListener("loadedmetadata", () => syncTimelineAudio({ forceSeek: true }));
  audio.addEventListener("error", () => {
    audio.pause();
  });
  timelineAudioElements.set(source.key, audio);

  return audio;
}

function applyTimelineAudioSettings(audio: HTMLAudioElement, source: TimelineAudioSource) {
  const speed = Number.isFinite(source.clip.speed) && source.clip.speed > 0 ? source.clip.speed : 1;
  const muted = source.clip.muted || source.track.muted;
  const volume = clamp(source.clip.volume, 0, 1);

  if (audio.muted !== muted) {
    audio.muted = muted;
  }

  if (Math.abs(audio.volume - volume) > 0.001) {
    audio.volume = volume;
  }

  if (Math.abs(audio.playbackRate - speed) > 0.001) {
    audio.playbackRate = speed;
  }
}

function syncTimelineAudioTime(
  audio: HTMLAudioElement,
  source: TimelineAudioSource,
  options: { forceSeek?: boolean; allowDriftCorrection?: boolean; seekThrottleMs?: number },
) {
  const targetTime = timelineAudioTime(source);
  const drift = Math.abs(audio.currentTime - targetTime);

  if (options.forceSeek === true) {
    if (options.seekThrottleMs && props.isPlaying) {
      const now = performance.now();
      const lastSeekMs = timelineAudioLastCorrectionMs.get(source.key) ?? 0;

      if (now - lastSeekMs < options.seekThrottleMs) {
        return;
      }

      timelineAudioLastCorrectionMs.set(source.key, now);
    }

    seekTimelineAudio(audio, targetTime);
    return;
  }

  if (!props.isPlaying || options.allowDriftCorrection !== true || drift <= AUDIO_DRIFT_SEEK_SECONDS) {
    return;
  }

  const now = performance.now();
  const lastCorrectionMs = timelineAudioLastCorrectionMs.get(source.key) ?? 0;

  if (now - lastCorrectionMs < AUDIO_DRIFT_CORRECTION_INTERVAL_MS) {
    return;
  }

  timelineAudioLastCorrectionMs.set(source.key, now);
  seekTimelineAudio(audio, targetTime);
}

function timelineAudioTime(source: TimelineAudioSource) {
  const speed = Number.isFinite(source.clip.speed) && source.clip.speed > 0 ? source.clip.speed : 1;
  const localTime = Math.max(0, props.playhead - source.clip.start) * speed;
  const mediaStart = Math.max(0, source.clip.trimStart);
  const mediaEnd = Math.max(mediaStart, source.asset.duration - Math.max(0, source.clip.trimEnd));

  return clamp(mediaStart + localTime, mediaStart, mediaEnd);
}

function requestTimelineAudioPlay(audio: HTMLAudioElement) {
  if (!audio.paused) {
    return;
  }

  void audio.play().catch(() => {
    // The preview clock can keep moving if the browser blocks audio playback.
  });
}

function seekTimelineAudio(audio: HTMLAudioElement, seconds: number) {
  if (Math.abs(audio.currentTime - seconds) <= PREVIEW_MIN_SEEK_SECONDS) {
    return;
  }

  try {
    const seekableAudio = audio as HTMLAudioElement & { fastSeek?: (time: number) => void };

    if (typeof seekableAudio.fastSeek === "function") {
      seekableAudio.fastSeek(seconds);
    } else {
      audio.currentTime = seconds;
    }
  } catch {
    // Some audio files reject seeking until metadata is ready.
  }
}

function stopTimelineAudio() {
  for (const audio of timelineAudioElements.values()) {
    audio.pause();
    audio.removeAttribute("src");
    audio.load();
  }

  timelineAudioElements.clear();
  timelineAudioLastCorrectionMs.clear();
}

function clamp(value: number, min: number, max: number) {
  return Math.min(Math.max(value, min), max);
}
</script>

<template>
  <section class="grid min-h-0 grid-rows-[minmax(0,1fr)_52px] overflow-hidden bg-[#05070a]" aria-label="剪辑预览器">
    <div ref="stageRef" class="relative grid min-h-0 place-items-center overflow-hidden px-8 py-6 max-[900px]:px-5">
      <div class="relative overflow-hidden border border-[#101318] bg-black shadow-[0_30px_70px_rgb(0_0_0/0.42)]" :style="previewFrameStyle">
        <video
          v-if="activePreviewSource?.asset.type === 'video' && !hasPreviewError"
          ref="previewVideoRef"
          class="absolute inset-0 size-full bg-black object-contain"
          :src="activePreviewSource.asset.url"
          playsinline
          preload="auto"
          @loadedmetadata="syncPreviewVideo({ forceSeek: true })"
          @canplay="syncPreviewVideo()"
          @playing="handleNativeVideoPlaying"
          @pause="handleNativeVideoClockSuspended"
          @waiting="handleNativeVideoClockSuspended"
          @seeking="handleNativeVideoClockSuspended"
          @seeked="handleNativeVideoSeeked"
          @ended="handleNativeVideoEnded"
          @error="handlePreviewError"
        ></video>
        <img
          v-else-if="activePreviewSource?.asset.type === 'image' && !hasPreviewError"
          class="absolute inset-0 size-full bg-black object-contain"
          :src="activePreviewSource.asset.url"
          alt=""
          @error="handlePreviewError"
        />

        <template v-else>
          <div class="absolute inset-0 bg-black">
            <span v-if="hasPreviewError" class="absolute inset-x-8 top-1/2 -translate-y-1/2 text-center text-[14px] font-semibold text-[#d8deea]">视频预览加载失败</span>
          </div>
        </template>

        <div v-if="activeCaptionOverlays.length > 0" class="pointer-events-none absolute inset-0 z-10 overflow-hidden">
          <button
            v-for="(overlay, index) in activeCaptionOverlays"
            :key="overlay.id"
            class="pointer-events-auto absolute left-1/2 top-[72%] max-w-[82%] overflow-visible whitespace-pre-wrap break-words border-0 bg-transparent text-center font-black tracking-normal drop-shadow-[0_3px_14px_rgb(0_0_0/0.85)] outline-none transition-[outline-color,filter] duration-150 hover:drop-shadow-[0_5px_18px_rgb(47_109_246/0.55)] focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-4 focus-visible:outline-[#60a5fa]"
            :class="selectedClip?.id === overlay.clip.id ? 'outline outline-2 outline-offset-4 outline-[#60a5fa]/80' : ''"
            :style="captionOverlayStyle(overlay, index)"
            type="button"
            :aria-label="`选择文字片段 ${overlay.text}`"
            @click.stop="emit('selectCaptionClip', overlay.clip.id)"
          >
            <span
              v-if="overlay.clip.textStyle?.backgroundEnabled"
              class="pointer-events-none absolute left-1/2 top-1/2 z-0"
              :style="captionOverlayBackgroundStyle(overlay)"
              aria-hidden="true"
            ></span>
            <span class="relative z-[1]">{{ overlay.text }}</span>
          </button>
        </div>

        <div v-if="activePreviewSource && !hasPreviewError" class="absolute left-5 top-5 inline-flex h-8 items-center gap-2 rounded-lg border border-white/10 bg-black/25 px-3 text-[11px] font-black text-[#c9d5e6]">
          <Video :size="14" class="text-[#60a5fa]" />
          {{ activePreviewResolution.width }}x{{ activePreviewResolution.height }}
        </div>
        <UButton v-if="activePreviewSource && !hasPreviewError" color="neutral" variant="soft" square size="sm" class="absolute right-5 top-5 bg-black/35" type="button" aria-label="全屏预览">
          <Maximize2 :size="15" />
        </UButton>

        <div v-if="activePreviewSource && !hasPreviewError" class="absolute bottom-4 left-5 max-w-[min(460px,calc(100%_-_40px))] rounded-lg bg-black/55 px-3 py-2 text-left shadow-[0_10px_24px_rgb(0_0_0/0.28)]">
          <strong class="block truncate text-[12px] font-black text-white">{{ activePreviewName }}</strong>
          <span class="mt-0.5 block font-mono text-[10px] font-bold text-[#9fb4d2]">{{ formatTimecode(activePreviewTime) }}</span>
        </div>
      </div>
    </div>

    <footer class="grid h-[52px] grid-cols-[1fr_auto_1fr] items-center border-t border-default bg-default/90 px-6 text-default backdrop-blur">
      <UBadge color="neutral" variant="subtle" size="lg" class="w-fit min-w-0 truncate font-mono font-black">
        <strong class="text-[#60a5fa]">{{ formatTimecode(previewDisplayTime) }}</strong>
        <span class="mx-3 text-[#5f6b7d]">/</span>
        <span class="text-[#8993a3]">{{ formatTimecode(previewDisplayDuration) }}</span>
      </UBadge>

      <UButton
        color="neutral"
        variant="ghost"
        square
        size="lg"
        type="button"
        :disabled="!canPlayback"
        :aria-label="canPlayback ? (isPlaying ? '暂停预览' : '播放预览') : '时间线为空，无法播放'"
        @click="$emit('togglePlayback')"
      >
        <Pause v-if="isPlaying" :size="19" />
        <Play v-else :size="19" />
      </UButton>

      <span class="flex items-center justify-end gap-3">
        <UButton color="neutral" variant="soft" size="md" type="button">
          Fit
          <ChevronDown :size="15" />
        </UButton>
        <USeparator orientation="vertical" class="h-6" />
        <UButton color="neutral" variant="ghost" square size="md" type="button" aria-label="全屏预览">
          <Maximize2 :size="18" />
        </UButton>
      </span>
    </footer>
  </section>
</template>
