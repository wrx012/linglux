<script setup lang="ts">
import {
  AlignEndHorizontal,
  AlignStartHorizontal,
  Bookmark,
  ChartLine,
  Copy,
  Eye,
  Link,
  Magnet,
  Minus,
  MoveHorizontal,
  Plus,
  Scissors,
  Snowflake,
  Trash2,
  Type as TypeIcon,
  Video,
  Volume2,
  VolumeX,
} from "@lucide/vue";
import type { EditorProject, TimelineClip } from "../../types/editor";

const props = defineProps<{
  project: EditorProject;
  selectedClipId?: string;
  playhead: number;
  timelineScale: number;
  snapEnabled: boolean;
}>();

const emit = defineEmits<{
  selectClip: [clipId: string];
  updatePlayhead: [seconds: number];
  trimClip: [clipId: string, edge: "start" | "end"];
  splitSelected: [];
  deleteSelected: [];
  zoomIn: [];
  zoomOut: [];
}>();

function pixelsPerSecond() {
  return 132 * props.timelineScale;
}

function clipStyle(clip: TimelineClip) {
  const pps = pixelsPerSecond();

  return {
    left: `${clip.start * pps}px`,
    width: `${Math.max(clip.duration * pps, 150)}px`,
  };
}

function trackWidth() {
  return `${Math.max(props.project.duration * pixelsPerSecond(), 1900)}px`;
}

function setPlayhead(event: MouseEvent) {
  const target = event.currentTarget;

  if (!(target instanceof HTMLElement)) {
    return;
  }

  const bounds = target.getBoundingClientRect();
  const seconds = (event.clientX - bounds.left + target.scrollLeft) / pixelsPerSecond();
  emit("updatePlayhead", Math.max(0, Math.min(props.project.duration, seconds)));
}

function clipTone(clip: TimelineClip) {
  if (clip.type === "caption" || clip.type === "overlay") {
    return "border-[#0ea5e9] bg-[#5bc1aa] text-white";
  }

  return "border-[#d7d7d7] bg-white text-[#262626]";
}

function isVisualClip(clip: TimelineClip) {
  return clip.type === "video" || clip.type === "audio";
}
</script>

<template>
  <section class="grid min-h-[420px] grid-rows-[48px_1fr] overflow-hidden rounded-md border border-[#d7d7d7] bg-[#f7f8fa] text-[#171717]" aria-label="多轨时间线">
    <header class="grid grid-cols-[360px_1fr_360px] items-center border-b border-[#d7d7d7] bg-white px-4">
      <div class="flex items-center gap-4 text-[#525252]">
        <button class="grid size-7 place-items-center rounded-md hover:bg-[#eef2f7]" type="button" title="分割" aria-label="分割选中片段" @click="emit('splitSelected')">
          <Scissors :size="18" />
        </button>
        <AlignStartHorizontal :size="18" />
        <AlignEndHorizontal :size="18" />
        <Link :size="18" />
        <Copy :size="18" />
        <Snowflake :size="18" />
        <button class="grid size-7 place-items-center rounded-md text-[#525252] hover:bg-[#fee2e2] hover:text-[#dc2626]" type="button" title="删除" aria-label="删除选中片段" @click="emit('deleteSelected')">
          <Trash2 :size="18" />
        </button>
        <span class="h-7 w-px bg-[#d7d7d7]"></span>
        <Bookmark :size="18" />
        <ChartLine :size="18" />
      </div>

      <div class="justify-self-center rounded-full border border-[#d7d7d7] bg-[#ededed] px-4 py-1 text-[14px] shadow-sm">
        Main scene
      </div>

      <div class="flex items-center justify-end gap-4 text-[#525252]">
        <span class="grid size-8 place-items-center rounded-lg bg-[#e0f2fe] text-[#0284c7]">
          <Magnet :size="18" />
        </span>
        <MoveHorizontal :size="19" />
        <button class="grid size-8 place-items-center rounded-lg hover:bg-[#eef2f7]" type="button" aria-label="缩小时间线" @click="emit('zoomOut')">
          <Minus :size="17" />
        </button>
        <div class="h-1.5 w-36 rounded-full bg-[#e5e5e5]">
          <div class="h-full rounded-full bg-[#38bdf8]" :style="{ width: `${Math.min(Math.round(timelineScale * 42), 100)}%` }"></div>
        </div>
        <button class="grid size-8 place-items-center rounded-lg hover:bg-[#eef2f7]" type="button" aria-label="放大时间线" @click="emit('zoomIn')">
          <Plus :size="17" />
        </button>
      </div>
    </header>

    <div class="grid min-h-0 grid-cols-[136px_1fr] overflow-hidden">
      <div class="border-r border-[#d7d7d7] bg-white pt-10">
        <div v-for="track in project.tracks" :key="track.id" class="grid h-[76px] grid-cols-[24px_24px_24px] items-center justify-end gap-3 border-b border-[#ececec] pr-4 text-[#737373]">
          <Volume2 v-if="track.type === 'audio' && !track.muted" :size="17" />
          <VolumeX v-else-if="track.type === 'audio'" :size="17" class="text-[#ef4444]" />
          <Eye v-else :size="17" />
          <TypeIcon v-if="track.type === 'caption' || track.type === 'overlay'" :size="17" />
          <Video v-else :size="17" />
        </div>
      </div>

      <div class="min-w-0 overflow-auto" @click="setPlayhead">
        <div class="relative min-h-full" :style="{ width: trackWidth() }">
          <div class="sticky top-0 z-10 h-10 border-b border-[#ececec] bg-[#f7f8fa]">
            <span
              v-for="tick in Math.ceil(project.duration) + 1"
              :key="tick"
              class="absolute top-0 h-full border-l border-[#d7d7d7] pl-1 text-[12px] leading-10 text-[#8a8a8a]"
              :style="{ left: `${(tick - 1) * pixelsPerSecond()}px` }"
            >
              {{ tick === 1 ? "0:00" : `${tick - 1}f` }}
            </span>
          </div>

          <div
            class="pointer-events-none absolute bottom-0 top-0 z-20 w-px bg-[#0ea5e9]"
            :style="{ left: `${playhead * pixelsPerSecond()}px` }"
          ></div>

          <div v-for="track in project.tracks" :key="track.id" class="relative h-[76px] border-b border-[#ececec] bg-[#f7f8fa]">
            <div
              v-for="clip in track.clips"
              :key="clip.id"
              class="absolute top-1.5 grid h-16 items-center overflow-hidden rounded-md border text-left text-[12px]"
              :class="[clipTone(clip), selectedClipId === clip.id ? 'ring-1 ring-[#0ea5e9]' : '']"
              :style="clipStyle(clip)"
              role="button"
              tabindex="0"
              :aria-label="`选择片段 ${clip.name}`"
              @click.stop="emit('selectClip', clip.id)"
              @keydown.enter.stop="emit('selectClip', clip.id)"
            >
              <template v-if="isVisualClip(clip)">
                <div class="grid h-full grid-cols-6 gap-1 bg-white px-1 py-1">
                  <span v-for="frame in 6" :key="frame" class="relative overflow-hidden rounded-sm border border-[#cbd5e1] bg-[#eef2f7]">
                    <span class="absolute inset-x-[12%] top-[20%] h-[46%] border border-[#94a3b8] bg-white"></span>
                    <span class="absolute inset-x-[12%] top-[18%] h-1 bg-[#0b6b45]"></span>
                    <span class="absolute inset-x-0 bottom-0 h-2 bg-[#111827]"></span>
                  </span>
                </div>
                <span class="pointer-events-none absolute left-2 top-1 text-[9px] text-white drop-shadow">{{ clip.name }}</span>
              </template>
              <span v-else class="truncate px-3 font-medium">
                {{ clip.captionText ?? clip.name }}
              </span>
            </div>
          </div>
        </div>
      </div>
    </div>
  </section>
</template>
