<script setup lang="ts">
import { computed, nextTick, ref, watch } from "vue";
import { Clapperboard, Play, X } from "@lucide/vue";
import type { StoryboardFrameRect, StoryboardToVideoRequest } from "../../types/editor";

export interface StoryboardSource {
  name: string;
  managedPath: string;
  url: string;
}

const props = defineProps<{
  open: boolean;
  source?: StoryboardSource;
  projectId: string;
  projectFps: number;
  isGenerating: boolean;
  progress: number;
  status: string;
  error?: string;
}>();

const emit = defineEmits<{
  close: [];
  generate: [request: StoryboardToVideoRequest];
  cancel: [];
}>();

const imageElement = ref<HTMLImageElement | null>(null);
const sourceWidth = ref(0);
const sourceHeight = ref(0);
const columns = ref(5);
const rows = ref(5);
const frameCount = ref(24);
const outerLeft = ref(0);
const outerTop = ref(0);
const outerRight = ref(0);
const outerBottom = ref(0);
const gapX = ref(0);
const gapY = ref(0);
const topTrim = ref(0);
const fps = ref(24);
const outputName = ref("storyboard-animation");
const confidence = ref("等待识别");
const detectionError = ref("");
const detectedColumns = ref<Array<[number, number]>>([]);
const detectedRows = ref<Array<[number, number]>>([]);
const useDetectedGeometry = ref(false);
let applyingDetection = false;

watch([columns, rows, outerLeft, outerTop, outerRight, outerBottom, gapX, gapY], () => {
  if (!applyingDetection) useDetectedGeometry.value = false;
});

watch(() => props.open, (open) => {
  if (open) {
    fps.value = Math.max(1, Math.round(props.projectFps || 24));
    outputName.value = `${props.source?.name.replace(/\.[^.]+$/, "") || "storyboard"}-animation`;
    void nextTick(() => {
      if (imageElement.value?.complete) {
        analyzeImage();
      }
    });
  }
});

const cellWidth = computed(() => Math.floor((sourceWidth.value - outerLeft.value - outerRight.value - gapX.value * (columns.value - 1)) / Math.max(columns.value, 1)));
const cellHeight = computed(() => Math.floor((sourceHeight.value - outerTop.value - outerBottom.value - gapY.value * (rows.value - 1)) / Math.max(rows.value, 1)));
const frames = computed<StoryboardFrameRect[]>(() => {
  if (sourceWidth.value <= 0 || sourceHeight.value <= 0) return [];
  const count = Math.min(Math.max(0, Math.round(frameCount.value)), columns.value * rows.value);
  return Array.from({ length: count }, (_, index) => {
    const column = index % columns.value;
    const row = Math.floor(index / columns.value);
    if (useDetectedGeometry.value && detectedColumns.value[column] && detectedRows.value[row]) {
      const [xStart, xEnd] = detectedColumns.value[column];
      const [yStart, yEnd] = detectedRows.value[row];
      return { x: xStart, y: yStart + topTrim.value, width: Math.max(0, xEnd - xStart) & ~1, height: Math.max(0, yEnd - yStart - topTrim.value) & ~1 };
    }
    const width = Math.max(0, cellWidth.value) & ~1;
    const height = Math.max(0, cellHeight.value - topTrim.value) & ~1;
    return {
      x: Math.round(outerLeft.value + column * (cellWidth.value + gapX.value)),
      y: Math.round(outerTop.value + row * (cellHeight.value + gapY.value) + topTrim.value),
      width,
      height,
    };
  });
});
const canGenerate = computed(() => !!props.source && !props.isGenerating && frames.value.length > 0 && frames.value.every((frame) => frame.width >= 2 && frame.height >= 2 && frame.x + frame.width <= sourceWidth.value && frame.y + frame.height <= sourceHeight.value));
const durationLabel = computed(() => `${frames.value.length} 帧 · ${(frames.value.length / Math.max(fps.value, 1)).toFixed(2)} 秒`);

function analyzeImage() {
  const image = imageElement.value;
  if (!image?.naturalWidth || !image.naturalHeight) return;
  sourceWidth.value = image.naturalWidth;
  sourceHeight.value = image.naturalHeight;
  const canvas = document.createElement("canvas");
  const scale = Math.min(1, 1400 / Math.max(image.naturalWidth, image.naturalHeight));
  canvas.width = Math.max(1, Math.round(image.naturalWidth * scale));
  canvas.height = Math.max(1, Math.round(image.naturalHeight * scale));
  const context = canvas.getContext("2d", { willReadFrequently: true });
  if (!context) return;
  context.drawImage(image, 0, 0, canvas.width, canvas.height);
  const pixels = context.getImageData(0, 0, canvas.width, canvas.height);
  const vertical = separatorBands(pixels.data, canvas.width, canvas.height, true);
  const horizontal = separatorBands(pixels.data, canvas.width, canvas.height, false);
  const xGaps = contentGaps(vertical, canvas.width);
  const yGaps = contentGaps(horizontal, canvas.height);
  if (xGaps.length >= 2 && yGaps.length >= 2 && xGaps.length <= 12 && yGaps.length <= 12) {
    applyingDetection = true;
    columns.value = xGaps.length;
    rows.value = yGaps.length;
    outerLeft.value = Math.round(xGaps[0][0] / scale);
    outerRight.value = Math.max(0, image.naturalWidth - Math.round(xGaps[xGaps.length - 1][1] / scale));
    outerTop.value = Math.round(yGaps[0][0] / scale);
    outerBottom.value = Math.max(0, image.naturalHeight - Math.round(yGaps[yGaps.length - 1][1] / scale));
    gapX.value = averageGap(xGaps, scale);
    gapY.value = averageGap(yGaps, scale);
    topTrim.value = Math.max(0, Math.round(Math.min(cellHeight.value * 0.13, 34)));
    detectedColumns.value = xGaps.map(([start, end]) => [Math.round(start / scale), Math.round(end / scale)]);
    detectedRows.value = yGaps.map(([start, end]) => [Math.round(start / scale), Math.round(end / scale)]);
    useDetectedGeometry.value = true;
    frameCount.value = detectFrameCount(context, scale);
    confidence.value = `已识别 ${columns.value}×${rows.value}，${frameCount.value} 个有效画格`;
    detectionError.value = "";
    void nextTick(() => { applyingDetection = false; });
  } else {
    applyingDetection = true;
    detectedColumns.value = [];
    detectedRows.value = [];
    useDetectedGeometry.value = false;
    columns.value = 5;
    rows.value = 5;
    frameCount.value = 24;
    outerLeft.value = Math.round(image.naturalWidth * 0.028);
    outerRight.value = Math.round(image.naturalWidth * 0.028);
    outerTop.value = Math.round(image.naturalHeight * 0.028);
    outerBottom.value = Math.round(image.naturalHeight * 0.028);
    gapX.value = Math.max(2, Math.round(image.naturalWidth * 0.006));
    gapY.value = Math.max(2, Math.round(image.naturalHeight * 0.006));
    topTrim.value = Math.round((image.naturalHeight / 5) * 0.11);
    confidence.value = "已载入建议参数";
    detectionError.value = "未能可靠识别全部分隔线，请核对预览框。";
    void nextTick(() => { applyingDetection = false; });
  }
}

function separatorBands(data: Uint8ClampedArray, width: number, height: number, vertical: boolean) {
  const length = vertical ? width : height;
  const cross = vertical ? height : width;
  const flags: boolean[] = [];
  for (let axis = 0; axis < length; axis += 1) {
    let dark = 0;
    let luminance = 0;
    let samples = 0;
    for (let offset = 0; offset < cross; offset += 3) {
      const x = vertical ? axis : offset;
      const y = vertical ? offset : axis;
      const index = (y * width + x) * 4;
      const value = data[index] * 0.2126 + data[index + 1] * 0.7152 + data[index + 2] * 0.0722;
      luminance += value;
      dark += value < 48 ? 1 : 0;
      samples += 1;
    }
    flags.push(luminance / samples < 58 && dark / samples > 0.58);
  }
  const bands: Array<[number, number]> = [];
  let start = -1;
  flags.forEach((flag, index) => {
    if (flag && start < 0) start = index;
    if ((!flag || index === flags.length - 1) && start >= 0) {
      const end = flag && index === flags.length - 1 ? index + 1 : index;
      if (end - start >= 2) bands.push([start, end]);
      start = -1;
    }
  });
  return bands;
}

function contentGaps(bands: Array<[number, number]>, length: number) {
  const gaps: Array<[number, number]> = [];
  const edges = [[0, 0] as [number, number], ...bands, [length, length] as [number, number]];
  for (let index = 0; index < edges.length - 1; index += 1) {
    const start = edges[index][1];
    const end = edges[index + 1][0];
    if (end - start > length * 0.1) gaps.push([start, end]);
  }
  return gaps;
}

function averageGap(gaps: Array<[number, number]>, scale: number) {
  if (gaps.length < 2) return 0;
  const values = gaps.slice(0, -1).map((gap, index) => gaps[index + 1][0] - gap[1]);
  return Math.max(0, Math.round(values.reduce((sum, value) => sum + value, 0) / values.length / scale));
}

function detectFrameCount(context: CanvasRenderingContext2D, scale: number) {
  let count = columns.value * rows.value;
  while (count > 1) {
    const index = count - 1;
    const column = index % columns.value;
    const row = Math.floor(index / columns.value);
    const x = Math.round((outerLeft.value + column * (cellWidth.value + gapX.value)) * scale);
    const y = Math.round((outerTop.value + row * (cellHeight.value + gapY.value) + topTrim.value) * scale);
    const width = Math.max(1, Math.round(cellWidth.value * scale));
    const height = Math.max(1, Math.round((cellHeight.value - topTrim.value) * scale));
    const image = context.getImageData(x, y, Math.min(width, context.canvas.width - x), Math.min(height, context.canvas.height - y));
    let sum = 0;
    let sumSquares = 0;
    let samples = 0;
    for (let pixel = 0; pixel < image.data.length; pixel += 16) {
      const value = image.data[pixel] * 0.2126 + image.data[pixel + 1] * 0.7152 + image.data[pixel + 2] * 0.0722;
      sum += value;
      sumSquares += value * value;
      samples += 1;
    }
    const mean = sum / Math.max(samples, 1);
    const variance = sumSquares / Math.max(samples, 1) - mean * mean;
    if (mean > 34 || variance > 180) break;
    count -= 1;
  }
  return count;
}

function frameStyle(frame: StoryboardFrameRect) {
  return { left: `${frame.x / sourceWidth.value * 100}%`, top: `${frame.y / sourceHeight.value * 100}%`, width: `${frame.width / sourceWidth.value * 100}%`, height: `${frame.height / sourceHeight.value * 100}%` };
}

function submit() {
  if (!props.source || !canGenerate.value) return;
  emit("generate", { projectId: props.projectId, sourcePath: props.source.managedPath, sourceWidth: sourceWidth.value, sourceHeight: sourceHeight.value, frames: frames.value, fps: Math.round(fps.value), outputName: outputName.value.trim() || "storyboard-animation" });
}

function updateOpen(open: boolean) {
  if (!open && !props.isGenerating) emit("close");
}
</script>

<template>
  <UiModal :open="open" :dismissible="!isGenerating" title="分镜图转视频" class="w-auto max-w-none bg-transparent p-0 ring-0 shadow-none" :ui="{ overlay: 'z-[75] bg-black/65 backdrop-blur-sm', content: 'z-[75] max-h-none' }" @update:open="updateOpen">
    <template #content>
      <section class="flex max-h-[calc(100dvh_-_32px)] w-[min(980px,calc(100vw_-_32px))] flex-col overflow-hidden rounded-xl border border-[#26313a] bg-[#0d1117] shadow-[0_24px_90px_rgb(0_0_0/0.6)]">
        <header class="flex items-center justify-between border-b border-[#222b34] px-5 py-4">
          <div><p class="text-[10px] font-black text-[#5eead4]">FRAME SEQUENCE</p><h2 class="text-[16px] font-bold text-white">分镜图转视频</h2></div>
          <UiButton square color="neutral" variant="ghost" type="button" :disabled="isGenerating" aria-label="关闭" @click="emit('close')"><X :size="16" /></UiButton>
        </header>
        <div class="grid min-h-0 flex-1 grid-cols-[minmax(0,1fr)_300px] overflow-hidden max-[800px]:grid-cols-1">
          <section class="editor-scrollbar overflow-auto bg-[#080b0f] p-5">
            <div class="relative mx-auto w-fit max-w-full overflow-hidden rounded-lg border border-[#29343e] bg-black">
              <img v-if="source" ref="imageElement" :src="source.url" class="block max-h-[620px] max-w-full object-contain" alt="分镜源图" @load="analyzeImage" />
              <span v-for="(frame, index) in frames" :key="index" class="pointer-events-none absolute border-2 border-[#2dd4bf] bg-[#14b8a6]/8" :style="frameStyle(frame)"><b class="absolute left-0 top-0 bg-[#0f766e] px-1 text-[9px] text-white">{{ index + 1 }}</b></span>
            </div>
          </section>
          <aside class="editor-scrollbar grid content-start gap-4 overflow-y-auto border-l border-[#222b34] bg-[#10151c] p-4">
            <div><p class="text-[12px] font-black text-[#dbeafe]">{{ confidence }}</p><p v-if="detectionError" class="mt-1 text-[10px] text-[#fbbf24]">{{ detectionError }}</p><p class="mt-1 text-[10px] text-[#64748b]">绿色框即最终画面，顺序从左到右、从上到下。</p></div>
            <div class="grid grid-cols-3 gap-2">
              <label class="text-[10px] text-[#94a3b8]">列数<UiInput v-model.number="columns" type="number" :min="1" :max="12" size="xs" /></label>
              <label class="text-[10px] text-[#94a3b8]">行数<UiInput v-model.number="rows" type="number" :min="1" :max="12" size="xs" /></label>
              <label class="text-[10px] text-[#94a3b8]">有效帧<UiInput v-model.number="frameCount" type="number" :min="1" :max="columns * rows" size="xs" /></label>
            </div>
            <div class="grid grid-cols-2 gap-2">
              <label class="text-[10px] text-[#94a3b8]">左边界<UiInput v-model.number="outerLeft" type="number" :min="0" size="xs" /></label>
              <label class="text-[10px] text-[#94a3b8]">右边界<UiInput v-model.number="outerRight" type="number" :min="0" size="xs" /></label>
              <label class="text-[10px] text-[#94a3b8]">上边界<UiInput v-model.number="outerTop" type="number" :min="0" size="xs" /></label>
              <label class="text-[10px] text-[#94a3b8]">下边界<UiInput v-model.number="outerBottom" type="number" :min="0" size="xs" /></label>
              <label class="text-[10px] text-[#94a3b8]">横间距<UiInput v-model.number="gapX" type="number" :min="0" size="xs" /></label>
              <label class="text-[10px] text-[#94a3b8]">纵间距<UiInput v-model.number="gapY" type="number" :min="0" size="xs" /></label>
              <label class="text-[10px] text-[#94a3b8]">编号裁切<UiInput v-model.number="topTrim" type="number" :min="0" size="xs" /></label>
            </div>
            <label class="text-[10px] text-[#94a3b8]">输出名称<UiInput v-model="outputName" size="sm" /></label>
            <label class="text-[10px] text-[#94a3b8]">FPS<UiInput v-model.number="fps" type="number" :min="1" :max="120" size="sm" /></label>
            <div class="rounded-lg border border-[#26313a] bg-[#0b1016] p-3"><p class="text-[11px] font-black text-[#5eead4]">{{ durationLabel }}</p><p class="mt-1 text-[9px] text-[#64748b]">H.264 · 无声 MP4 · {{ frames[0]?.width || 0 }}×{{ frames[0]?.height || 0 }}</p></div>
            <div v-if="isGenerating || error" class="grid gap-2"><UiProgress :model-value="progress" /><p class="text-[10px]" :class="error ? 'text-[#fca5a5]' : 'text-[#94a3b8]'">{{ error || status }}</p></div>
          </aside>
        </div>
        <footer class="flex items-center justify-between gap-3 border-t border-[#222b34] px-5 py-4"><span class="text-[10px] text-[#64748b]">生成结果会加入素材库，不自动放入时间线。</span><div class="flex gap-2"><UiButton v-if="isGenerating" color="error" variant="outline" type="button" @click="emit('cancel')">取消</UiButton><UiButton color="primary" type="button" :disabled="!canGenerate" @click="submit"><Play :size="14" />{{ isGenerating ? '生成中…' : '生成视频' }}</UiButton></div></footer>
      </section>
    </template>
  </UiModal>
</template>
