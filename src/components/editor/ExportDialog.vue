<script setup lang="ts">
import { computed } from "vue";
import { Download, FolderOpen, X } from "@lucide/vue";
import type { ExportPreset } from "../../types/editor";

const props = defineProps<{
  open: boolean;
  presets: ExportPreset[];
  selectedPreset: ExportPreset;
  isExporting: boolean;
  exportProgress: number;
  exportStatus: string;
  exportError?: string;
  exportOutputPath?: string;
  revealError?: string;
  hasExportResult: boolean;
}>();

const emit = defineEmits<{
  close: [];
  selectPreset: [presetId: string];
  export: [];
  cancel: [];
  openLocation: [];
  finish: [];
}>();

const normalizedProgress = computed(() => Math.min(Math.max(props.exportProgress, 0), 100));
const progressColor = computed(() => props.exportError ? "error" : props.hasExportResult ? "success" : "primary");
const statusLabel = computed(() => {
  if (props.exportError) {
    return "导出失败";
  }

  if (props.hasExportResult) {
    return "导出完成";
  }

  return props.exportStatus || "等待导出";
});

function updateOpen(open: boolean) {
  if (!open && !props.isExporting) {
    emit("close");
  }
}
</script>

<template>
  <UiModal
    :open="open"
    :dismissible="!isExporting"
    title="导出剪辑结果"
    class="w-auto max-w-none bg-transparent p-0 ring-0 shadow-none"
    :ui="{ overlay: 'z-[70] bg-black/60 backdrop-blur-sm', content: 'z-[70] max-h-none' }"
    @update:open="updateOpen"
  >
    <template #content>
    <section class="flex max-h-[calc(100dvh_-_48px)] w-[min(520px,calc(100vw_-_32px))] flex-col overflow-hidden rounded-xl border border-[#222228] bg-[#101014] shadow-[0_24px_80px_rgb(0_0_0/0.52)]">
      <header class="grid grid-cols-[1fr_auto] items-center gap-4 border-b border-[#222228] px-5 py-4">
        <div class="min-w-0">
          <p class="mb-1 text-[10px] font-extrabold leading-3 text-[#4b5563]">EXPORT</p>
          <h2 id="editor-export-title" aria-hidden="true" class="truncate text-[16px] font-bold leading-5 text-highlighted">导出剪辑结果</h2>
        </div>
        <UiButton class="grid size-8 place-items-center rounded-lg bg-[#15151a] p-0 text-[#9ca3af] ring-[#222228] hover:text-white" type="button" color="neutral" variant="outline" title="关闭导出" :disabled="isExporting" @click="emit('close')">
          <X :size="16" />
        </UiButton>
      </header>

      <section class="grid gap-3 overflow-y-auto px-5 py-5">
        <button
          v-for="preset in presets"
          :key="preset.id"
          class="grid min-h-[62px] grid-cols-[1fr_auto] items-center gap-3 rounded-lg border px-3 text-left"
          :class="selectedPreset.id === preset.id ? 'border-[#10b981] bg-[#10231d]' : 'border-[#222228] bg-[#15151a] hover:border-[#303038]'"
          type="button"
          :disabled="isExporting || hasExportResult"
          @click="$emit('selectPreset', preset.id)"
        >
          <span class="min-w-0">
            <strong class="block text-[12px] text-[#e5e7eb]">{{ preset.label }}</strong>
            <span class="mt-1 block text-[10px] text-[#6b7280]">{{ preset.resolution }} · {{ preset.fps }}fps · {{ preset.format.toUpperCase() }}</span>
          </span>
          <span class="rounded-full border border-[#222228] bg-[#0b0b0d] px-2 py-1 text-[9px] font-bold text-[#8a8a8f]">{{ preset.quality }}</span>
        </button>

        <section
          v-if="isExporting || exportProgress > 0 || exportError"
          class="grid gap-2 rounded-lg border border-[#222228] bg-[#15151a] p-3"
          aria-label="导出进度"
        >
          <div class="flex items-center justify-between gap-3">
            <span class="truncate text-[11px] font-black text-[#d1d5db]">{{ statusLabel }}</span>
            <span class="font-mono text-[11px] font-black" :class="exportError ? 'text-[#fca5a5]' : hasExportResult ? 'text-[#34d399]' : 'text-[#5eead4]'">{{ Math.round(normalizedProgress) }}%</span>
          </div>
          <UiProgress :model-value="normalizedProgress" :max="100" :color="progressColor" size="sm" class="rounded-full bg-[#070708] ring-1 ring-[#27313a]" />
          <p class="min-h-4 truncate text-[10px] font-semibold text-[#6b7280]">
            {{ hasExportResult ? "文件已导出，确认后回写到工作流。" : isExporting ? "请保持 Linglux 打开，正在写入本地文件。" : "可以重新选择预设后再次导出。" }}
          </p>
          <div v-if="hasExportResult && exportOutputPath" class="grid gap-1 border-t border-[#222228] pt-2">
            <span class="text-[9px] font-black uppercase text-[#4b5563]">导出位置</span>
            <code class="truncate font-mono text-[10px] font-bold text-[#9ca3af]" :title="exportOutputPath">{{ exportOutputPath }}</code>
          </div>
        </section>
      </section>

      <footer class="flex flex-wrap items-center justify-between gap-3 border-t border-[#222228] px-5 py-4">
        <span class="max-w-[310px] text-[11px]" :class="exportError || revealError ? 'font-semibold text-[#fca5a5]' : 'text-[#6b7280]'">
          {{ exportError || revealError || (hasExportResult ? "导出完成，可回到工作流继续编排。" : "使用本机 FFmpeg 导出到 Linglux 应用数据目录。") }}
        </span>
        <div class="flex flex-wrap items-center justify-end gap-2">
          <UiButton v-if="isExporting" class="h-9 min-w-[92px] justify-center bg-[#2a151a] px-3 text-[12px] font-black text-[#fca5a5] ring-[#4b2730] hover:bg-[#35191f] hover:ring-[#ef4444] hover:text-white" type="button" color="error" variant="outline" @click="emit('cancel')">
            <X :size="14" />
            取消导出
          </UiButton>
          <UiButton v-if="hasExportResult && exportOutputPath" class="h-9 min-w-[104px] justify-center bg-[#15151a] px-3 text-[12px] font-black text-[#d1d5db] ring-[#25303a] hover:bg-[#1d1d22] hover:ring-[#10b981]/70 hover:text-white" type="button" color="neutral" variant="outline" @click="emit('openLocation')">
            <FolderOpen :size="14" />
            打开文件夹
          </UiButton>
          <UiButton class="h-9 min-w-[116px] justify-center bg-[#10b981] px-4 text-[12px] font-black text-[#070708] hover:bg-[#18c991]" type="button" :disabled="isExporting" @click="hasExportResult ? emit('finish') : emit('export')">
            <Download :size="14" />
            {{ hasExportResult ? "回到工作流" : isExporting ? "导出中..." : "导出" }}
          </UiButton>
        </div>
      </footer>
    </section>
    </template>
  </UiModal>
</template>
