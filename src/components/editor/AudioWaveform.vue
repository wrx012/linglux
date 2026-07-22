<script setup lang="ts">
import { onMounted, onUnmounted, ref, watch } from "vue";

const props = withDefaults(
  defineProps<{
    peaks?: number[];
    sourceDuration?: number;
    trimStart?: number;
    trimEnd?: number;
    clipDuration?: number;
    speed?: number;
    playedRatio?: number;
    color?: string;
    playedColor?: string;
    baselineColor?: string;
    verticalPadding?: number;
  }>(),
  {
    peaks: () => [],
    sourceDuration: 0,
    trimStart: 0,
    trimEnd: 0,
    clipDuration: 0,
    speed: 1,
    playedRatio: 0,
    color: "rgb(125 240 166 / 0.48)",
    playedColor: "rgb(167 243 208 / 0.96)",
    baselineColor: "rgb(167 243 208 / 0.2)",
    verticalPadding: 2,
  },
);

const container = ref<HTMLElement | null>(null);
const canvas = ref<HTMLCanvasElement | null>(null);
let resizeObserver: ResizeObserver | undefined;
let renderFrameId: number | undefined;

function clamp(value: number, min: number, max: number) {
  return Math.min(Math.max(value, min), max);
}

function scheduleRender() {
  if (renderFrameId !== undefined) {
    return;
  }

  renderFrameId = requestAnimationFrame(() => {
    renderFrameId = undefined;
    renderWaveform();
  });
}

function renderWaveform() {
  const canvasElement = canvas.value;
  const bounds = container.value?.getBoundingClientRect();

  if (!canvasElement || !bounds || bounds.width <= 0 || bounds.height <= 0) {
    return;
  }

  const width = Math.max(1, Math.round(bounds.width));
  const height = Math.max(1, Math.round(bounds.height));
  const pixelRatio = Math.min(window.devicePixelRatio || 1, 2);
  const renderWidth = Math.round(width * pixelRatio);
  const renderHeight = Math.round(height * pixelRatio);

  if (canvasElement.width !== renderWidth || canvasElement.height !== renderHeight) {
    canvasElement.width = renderWidth;
    canvasElement.height = renderHeight;
  }

  const context = canvasElement.getContext("2d");

  if (!context) {
    return;
  }

  context.setTransform(pixelRatio, 0, 0, pixelRatio, 0, 0);
  context.clearRect(0, 0, width, height);

  const centerY = height / 2;
  context.fillStyle = props.baselineColor;
  context.fillRect(0, Math.floor(centerY), width, Math.max(1 / pixelRatio, 0.5));

  const amplitudes = visibleAmplitudes(width);

  if (amplitudes.length === 0) {
    return;
  }

  drawEnvelope(context, amplitudes, width, height, props.color);

  const playedWidth = width * clamp(props.playedRatio, 0, 1);

  if (playedWidth > 0) {
    context.save();
    context.beginPath();
    context.rect(0, 0, playedWidth, height);
    context.clip();
    drawEnvelope(context, amplitudes, width, height, props.playedColor);
    context.restore();
  }
}

function visibleAmplitudes(width: number) {
  const peaks = props.peaks;

  if (peaks.length === 0) {
    return [];
  }

  const sourceDuration = Math.max(props.sourceDuration, 0);
  const sourceStart = sourceDuration > 0 ? clamp(props.trimStart, 0, sourceDuration) : 0;
  const sourceStartRatio = sourceDuration > 0 ? sourceStart / sourceDuration : 0;
  const availableSourceEnd = sourceDuration > 0
    ? clamp(sourceDuration - props.trimEnd, sourceStart, sourceDuration)
    : sourceDuration;
  const requestedSourceEnd = sourceStart + Math.max(props.clipDuration, 0) * Math.max(props.speed, 0.01);
  const sourceEnd = sourceDuration > 0
    ? clamp(Math.min(availableSourceEnd, requestedSourceEnd), sourceStart, sourceDuration)
    : sourceDuration;
  const sourceEndRatio = sourceDuration > 0 ? clamp(sourceEnd / sourceDuration, sourceStartRatio, 1) : 1;
  const peakStart = sourceStartRatio * peaks.length;
  const peakEnd = Math.max(peakStart + 1, sourceEndRatio * peaks.length);
  const visiblePeakCount = Math.max(1, peakEnd - peakStart);
  const columnCount = Math.max(2, Math.min(Math.ceil(width), 2048));
  const amplitudes = new Array<number>(columnCount);

  for (let column = 0; column < columnCount; column += 1) {
    const rangeStart = peakStart + (column / columnCount) * visiblePeakCount;
    const rangeEnd = peakStart + ((column + 1) / columnCount) * visiblePeakCount;
    amplitudes[column] = peakAmplitude(peaks, rangeStart, rangeEnd);
  }

  return amplitudes;
}

function peakAmplitude(peaks: number[], rangeStart: number, rangeEnd: number) {
  if (rangeEnd - rangeStart < 1) {
    const index = clamp(Math.floor(rangeStart), 0, peaks.length - 1);
    const nextIndex = Math.min(index + 1, peaks.length - 1);
    const progress = rangeStart - Math.floor(rangeStart);
    return clamp((peaks[index] ?? 0) * (1 - progress) + (peaks[nextIndex] ?? 0) * progress, 0, 1);
  }

  const startIndex = clamp(Math.floor(rangeStart), 0, peaks.length - 1);
  const endIndex = clamp(Math.ceil(rangeEnd), startIndex + 1, peaks.length);
  let peak = 0;

  for (let index = startIndex; index < endIndex; index += 1) {
    peak = Math.max(peak, Math.abs(peaks[index] ?? 0));
  }

  return clamp(peak, 0, 1);
}

function drawEnvelope(
  context: CanvasRenderingContext2D,
  amplitudes: number[],
  width: number,
  height: number,
  color: string,
) {
  const centerY = height / 2;
  const maximumAmplitude = Math.max(0, centerY - Math.max(0, props.verticalPadding));
  const xStep = amplitudes.length > 1 ? width / (amplitudes.length - 1) : width;

  context.beginPath();
  context.moveTo(0, centerY);

  for (let index = 0; index < amplitudes.length; index += 1) {
    const amplitude = Math.max(amplitudes[index] ?? 0, 0.015) * maximumAmplitude;
    context.lineTo(index * xStep, centerY - amplitude);
  }

  for (let index = amplitudes.length - 1; index >= 0; index -= 1) {
    const amplitude = Math.max(amplitudes[index] ?? 0, 0.015) * maximumAmplitude;
    context.lineTo(index * xStep, centerY + amplitude);
  }

  context.closePath();
  context.fillStyle = color;
  context.fill();
}

watch(
  () => [
    props.peaks,
    props.sourceDuration,
    props.trimStart,
    props.trimEnd,
    props.clipDuration,
    props.speed,
    props.playedRatio,
    props.color,
    props.playedColor,
    props.baselineColor,
    props.verticalPadding,
  ],
  scheduleRender,
);

onMounted(() => {
  resizeObserver = new ResizeObserver(scheduleRender);

  if (container.value) {
    resizeObserver.observe(container.value);
  }

  scheduleRender();
});

onUnmounted(() => {
  resizeObserver?.disconnect();

  if (renderFrameId !== undefined) {
    cancelAnimationFrame(renderFrameId);
  }
});
</script>

<template>
  <div ref="container" class="relative size-full overflow-hidden">
    <canvas ref="canvas" class="absolute inset-0 size-full" aria-hidden="true"></canvas>
  </div>
</template>
