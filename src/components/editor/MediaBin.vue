<script setup lang="ts">
import {
  Captions,
  ChevronsRight,
  Folder,
  Headphones,
  Image as ImageIcon,
  Music,
  Settings,
  SlidersHorizontal,
  Smile,
  Type,
  WandSparkles,
  Video,
} from "@lucide/vue";
import type { Component } from "vue";
import type { MediaAsset, MediaAssetType } from "../../types/editor";

defineProps<{
  assets: MediaAsset[];
  activeAssetId?: string;
}>();

defineEmits<{
  selectAsset: [assetId: string];
}>();

const assetIcons: Record<MediaAssetType, Component> = {
  video: Video,
  image: ImageIcon,
  audio: Music,
  caption: Type,
};

function formatDuration(duration: number) {
  return `${duration.toFixed(1)}s`;
}
</script>

<template>
  <aside class="grid min-h-0 grid-cols-[48px_1fr] overflow-hidden rounded-md border border-[#d7d7d7] bg-white text-[#171717]" aria-label="剪辑器媒体库">
    <nav class="grid content-start justify-items-center gap-4 border-r border-[#e5e5e5] bg-[#fbfcfd] py-4 text-[#737373]" aria-label="剪辑工具栏">
      <Folder :size="17" />
      <Headphones :size="17" />
      <span class="grid size-9 place-items-center rounded-lg bg-[#e0f2fe] text-[#0284c7]">
        <Type :size="18" />
      </span>
      <Smile :size="17" />
      <WandSparkles :size="17" />
      <ChevronsRight :size="17" />
      <Captions :size="17" />
      <SlidersHorizontal :size="17" />
      <Settings :size="17" />
    </nav>

    <section class="grid min-h-0 grid-rows-[52px_1fr]">
      <header class="flex items-center border-b border-[#e5e5e5] px-4">
        <h2 class="text-[14px] font-medium text-[#737373]">Text</h2>
      </header>

      <div class="min-h-0 overflow-y-auto p-3">
        <button
          class="mb-5 grid h-[134px] w-[136px] place-items-center rounded-md bg-[#ececec] text-[13px] font-medium text-[#171717]"
          type="button"
          aria-label="添加默认文本"
        >
          Default text
        </button>

        <div class="grid gap-2">
          <button
            v-for="asset in assets"
            :key="asset.id"
            class="grid min-h-[54px] grid-cols-[30px_1fr_auto] items-center gap-2 rounded-md border px-2 text-left"
            :class="activeAssetId === asset.id ? 'border-[#0ea5e9] bg-[#eff6ff]' : 'border-[#e5e5e5] bg-white hover:bg-[#f8fafc]'"
            type="button"
            :aria-label="`选择素材 ${asset.name}`"
            @click="$emit('selectAsset', asset.id)"
          >
            <span class="grid size-7 place-items-center rounded bg-[#f1f5f9] text-[#0284c7]">
              <component :is="assetIcons[asset.type]" :size="15" />
            </span>
            <span class="min-w-0">
              <strong class="block truncate text-[11px] font-semibold text-[#262626]">{{ asset.name }}</strong>
              <span class="mt-0.5 block truncate text-[9px] text-[#8a8a8a]">{{ asset.type.toUpperCase() }}</span>
            </span>
            <span class="text-[9px] font-medium text-[#8a8a8a]">{{ formatDuration(asset.duration) }}</span>
          </button>
        </div>
      </div>
    </section>
  </aside>
</template>
