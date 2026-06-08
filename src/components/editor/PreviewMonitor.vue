<script setup lang="ts">
import { Captions, ChevronDown, Maximize2, Play, Video } from "@lucide/vue";
import type { EditorProject, TimelineClip } from "../../types/editor";
import { formatTimecode } from "../../lib/editorProject";

defineProps<{
  project: EditorProject;
  selectedClip?: TimelineClip;
  playhead: number;
  isPlaying: boolean;
}>();
</script>

<template>
  <section class="grid min-h-0 grid-rows-[1fr_58px] overflow-hidden rounded-md border border-[#d7d7d7] bg-[#f7f8fa]" aria-label="剪辑预览器">
    <div class="grid min-h-0 place-items-center px-6 pt-3">
      <div class="relative aspect-video w-[min(72vw,696px)] max-w-full overflow-hidden bg-black shadow-sm">
        <div class="absolute inset-x-[7%] top-[6%] h-[72%] border border-[#d1d5db] bg-[#f8fafc]">
          <div class="h-[12%] bg-[#e5e7eb] text-[7px] text-[#4b5563]">文件　编辑　视图　工具　窗口</div>
          <div class="mx-auto mt-[3%] h-[7%] w-[86%] rounded-sm bg-[#0b6b45]"></div>
          <div class="mx-auto mt-[2%] grid h-[53%] w-[86%] grid-cols-[18%_1fr_18%] gap-2 bg-white text-[7px] text-[#4b5563]">
            <div class="bg-[#f4f4f5] p-2">业务管理<br />付款管理<br />财务管理</div>
            <div class="grid place-items-center">
              <span class="text-[42px] font-semibold tracking-normal text-black">Loading</span>
            </div>
            <div class="bg-[#f4f4f5] p-2">服务平台<br />新增客户<br />统计信息</div>
          </div>
          <div class="mx-auto mt-[1.5%] h-[4%] w-[86%] bg-[#0b6b45]"></div>
        </div>
        <div class="absolute inset-x-0 bottom-0 h-[9%] bg-[#111827]"></div>

        <div class="absolute left-3 top-3 inline-flex h-6 items-center gap-1.5 rounded bg-white/90 px-2 text-[10px] font-medium text-[#525252]">
          <Video :size="12" class="text-[#0284c7]" />
          {{ project.resolution.width }}x{{ project.resolution.height }}
        </div>
        <div v-if="selectedClip?.captionText" class="absolute inset-x-16 bottom-7 inline-flex items-center justify-center gap-2 text-[18px] font-semibold text-black">
          <Captions :size="16" />
          {{ selectedClip.captionText }}
        </div>
      </div>
    </div>

    <footer class="grid grid-cols-[1fr_auto_1fr] items-center px-6 text-[13px] text-[#737373]">
      <span>
        <strong class="font-mono text-[#0ea5e9]">{{ formatTimecode(playhead) }}</strong>
        <span class="mx-2">/</span>
        <span class="font-mono">{{ formatTimecode(project.duration) }}</span>
      </span>
      <button class="grid size-9 place-items-center rounded-full text-[#171717] hover:bg-[#eef2f7]" type="button" :aria-label="isPlaying ? '暂停预览' : '播放预览'">
        <Play :size="18" />
      </button>
      <span class="flex justify-end gap-3">
        <button class="inline-flex h-9 items-center gap-2 rounded-xl border border-[#d7d7d7] bg-[#ededed] px-3 text-[#171717]" type="button">
          Fit
          <ChevronDown :size="14" />
        </button>
        <button class="grid size-9 place-items-center rounded-lg text-[#525252] hover:bg-[#eef2f7]" type="button" aria-label="全屏预览">
          <Maximize2 :size="18" />
        </button>
      </span>
    </footer>
  </section>
</template>
