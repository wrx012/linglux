<script setup lang="ts">
import { Download, X } from "@lucide/vue";
import type { ExportPreset } from "../../types/editor";

defineProps<{
  open: boolean;
  presets: ExportPreset[];
  selectedPreset: ExportPreset;
  isExporting: boolean;
}>();

defineEmits<{
  close: [];
  selectPreset: [presetId: string];
  export: [];
}>();
</script>

<template>
  <div
    v-if="open"
    class="fixed inset-0 z-[70] grid place-items-center bg-black/60 px-4 py-6 backdrop-blur-sm"
    role="dialog"
    aria-modal="true"
    aria-labelledby="editor-export-title"
    @click.self="$emit('close')"
  >
    <section class="flex max-h-[calc(100dvh_-_48px)] w-[min(520px,calc(100vw_-_32px))] flex-col overflow-hidden rounded-xl border border-[#222228] bg-[#101014] shadow-[0_24px_80px_rgb(0_0_0/0.52)]">
      <header class="grid grid-cols-[1fr_auto] items-center gap-4 border-b border-[#222228] px-5 py-4">
        <div class="min-w-0">
          <p class="mb-1 text-[10px] font-extrabold leading-3 text-[#4b5563]">EXPORT</p>
          <h2 id="editor-export-title" class="truncate text-[16px] font-bold leading-5 text-white">导出剪辑结果</h2>
        </div>
        <button class="grid size-8 place-items-center rounded-lg border border-[#222228] bg-[#15151a] text-[#9ca3af] hover:text-white" type="button" title="关闭导出" @click="$emit('close')">
          <X :size="16" />
        </button>
      </header>

      <section class="grid gap-3 overflow-y-auto px-5 py-5">
        <button
          v-for="preset in presets"
          :key="preset.id"
          class="grid min-h-[62px] grid-cols-[1fr_auto] items-center gap-3 rounded-lg border px-3 text-left"
          :class="selectedPreset.id === preset.id ? 'border-[#10b981] bg-[#10231d]' : 'border-[#222228] bg-[#15151a] hover:border-[#303038]'"
          type="button"
          @click="$emit('selectPreset', preset.id)"
        >
          <span class="min-w-0">
            <strong class="block text-[12px] text-[#e5e7eb]">{{ preset.label }}</strong>
            <span class="mt-1 block text-[10px] text-[#6b7280]">{{ preset.resolution }} · {{ preset.fps }}fps · {{ preset.format.toUpperCase() }}</span>
          </span>
          <span class="rounded-full border border-[#222228] bg-[#0b0b0d] px-2 py-1 text-[9px] font-bold text-[#8a8a8f]">{{ preset.quality }}</span>
        </button>
      </section>

      <footer class="flex flex-wrap items-center justify-between gap-3 border-t border-[#222228] px-5 py-4">
        <span class="text-[11px] text-[#6b7280]">V1 使用模拟导出，接口已按真实导出预留。</span>
        <button class="inline-flex h-9 min-w-[116px] items-center justify-center gap-2 rounded-lg border-0 bg-[#10b981] px-4 text-[12px] font-black text-[#070708] hover:brightness-110 disabled:cursor-wait disabled:opacity-75" type="button" :disabled="isExporting" @click="$emit('export')">
          <Download :size="14" />
          {{ isExporting ? "导出中..." : "导出" }}
        </button>
      </footer>
    </section>
  </div>
</template>
