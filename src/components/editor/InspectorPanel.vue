<script setup lang="ts">
import { Droplet, Expand, Type } from "@lucide/vue";
import type { TimelineClip } from "../../types/editor";

const props = defineProps<{
  selectedClip?: TimelineClip;
}>();

const emit = defineEmits<{
  updateClip: [clipId: string, patch: Partial<TimelineClip>];
}>();

function numberValue(event: Event) {
  const input = event.target as HTMLInputElement;
  return Number(input.value);
}

function booleanValue(event: Event) {
  const input = event.target as HTMLInputElement;
  return input.checked;
}

function updateTransform(key: "x" | "y" | "scale" | "rotation" | "opacity", value: number) {
  if (!props.selectedClip) {
    return;
  }

  emit("updateClip", props.selectedClip.id, {
    transform: {
      ...props.selectedClip.transform,
      [key]: value,
    },
  });
}

function updateEffectIntensity(value: number) {
  if (!props.selectedClip) {
    return;
  }

  const [firstEffect, ...rest] = props.selectedClip.effects;

  if (!firstEffect) {
    return;
  }

  emit("updateClip", props.selectedClip.id, {
    effects: [{ ...firstEffect, intensity: value }, ...rest],
  });
}
</script>

<template>
  <aside class="grid min-h-0 grid-cols-[48px_1fr] overflow-hidden rounded-md border border-[#d7d7d7] bg-white text-[#171717]" aria-label="剪辑属性面板">
    <nav class="grid content-start justify-items-center gap-5 border-r border-[#e5e5e5] bg-[#fbfcfd] py-4 text-[#737373]" aria-label="属性工具栏">
      <span class="grid size-9 place-items-center rounded-lg bg-[#e0f2fe] text-[#0284c7]">
        <Type :size="18" />
      </span>
      <Expand :size="17" />
      <Droplet :size="17" />
    </nav>

    <section v-if="selectedClip" class="min-h-0 overflow-y-auto">
      <details class="border-b border-[#e5e5e5] p-4" open>
        <summary class="flex cursor-pointer list-none items-center justify-between text-[15px] font-semibold text-[#171717]">
          Content
          <span class="text-[#737373]">⌄</span>
        </summary>
        <textarea
          class="mt-4 h-24 w-full resize-none rounded-xl border-0 bg-[#ececec] px-4 py-3 text-[14px] text-[#171717] outline-none focus:ring-2 focus:ring-[#0ea5e9]/30"
          :value="selectedClip.captionText ?? selectedClip.name"
          @input="emit('updateClip', selectedClip.id, { captionText: ($event.target as HTMLTextAreaElement).value })"
        ></textarea>
      </details>

      <details class="border-b border-[#e5e5e5] p-4" open>
        <summary class="flex cursor-pointer list-none items-center justify-between text-[15px] font-semibold text-[#171717]">
          Typography
          <span class="text-[#737373]">⌄</span>
        </summary>
        <div class="mt-4 grid gap-4">
          <label class="grid gap-2">
            <span class="text-[13px] text-[#8a8a8a]">Font</span>
            <span class="grid h-9 grid-cols-[24px_1fr_auto] items-center rounded-xl border border-[#d7d7d7] bg-[#ededed] px-3 text-[13px] text-[#525252]">
              <Type :size="14" />
              Arial
              <span>⌄</span>
            </span>
          </label>
          <label class="grid gap-2">
            <span class="text-[13px] text-[#8a8a8a]">Size</span>
            <input class="h-9 rounded-xl border border-[#d7d7d7] bg-[#ededed] px-3 text-[13px] text-[#525252] outline-none" type="number" value="15" />
          </label>
          <label class="grid gap-2">
            <span class="flex items-center gap-2 text-[13px] text-[#8a8a8a]">
              <span class="rotate-45 text-[15px]">◇</span>
              Color
            </span>
            <span class="grid h-9 grid-cols-[24px_1fr] items-center rounded-xl border border-[#d7d7d7] bg-[#ededed] px-3 text-[13px] text-[#525252]">
              <span class="size-5 rounded-md bg-[#141313]"></span>
              141313
            </span>
          </label>
        </div>
      </details>

      <details class="border-b border-[#e5e5e5] p-4" open>
        <summary class="flex cursor-pointer list-none items-center justify-between text-[15px] font-semibold text-[#171717]">
          Spacing
          <span class="text-[#737373]">⌄</span>
        </summary>
        <div class="mt-4 grid grid-cols-2 gap-3">
          <label class="grid gap-2">
            <span class="text-[12px] text-[#8a8a8a]">X</span>
            <input class="h-8 rounded-lg border border-[#d7d7d7] bg-[#f4f4f5] px-2 text-[12px]" type="number" :value="selectedClip.transform.x" @input="updateTransform('x', numberValue($event))" />
          </label>
          <label class="grid gap-2">
            <span class="text-[12px] text-[#8a8a8a]">Y</span>
            <input class="h-8 rounded-lg border border-[#d7d7d7] bg-[#f4f4f5] px-2 text-[12px]" type="number" :value="selectedClip.transform.y" @input="updateTransform('y', numberValue($event))" />
          </label>
          <label class="grid gap-2">
            <span class="text-[12px] text-[#8a8a8a]">Scale</span>
            <input class="h-8 rounded-lg border border-[#d7d7d7] bg-[#f4f4f5] px-2 text-[12px]" type="number" min="0.1" max="4" step="0.05" :value="selectedClip.transform.scale" @input="updateTransform('scale', numberValue($event))" />
          </label>
          <label class="grid gap-2">
            <span class="text-[12px] text-[#8a8a8a]">Rotate</span>
            <input class="h-8 rounded-lg border border-[#d7d7d7] bg-[#f4f4f5] px-2 text-[12px]" type="number" :value="selectedClip.transform.rotation" @input="updateTransform('rotation', numberValue($event))" />
          </label>
        </div>
      </details>

      <details class="p-4">
        <summary class="flex cursor-pointer list-none items-center justify-between text-[15px] font-semibold text-[#171717]">
          Clip
          <span class="text-[#737373]">⌄</span>
        </summary>
        <div class="mt-4 grid gap-3 text-[13px]">
          <label class="grid gap-2">
            <span class="text-[#8a8a8a]">Opacity {{ Math.round(selectedClip.transform.opacity * 100) }}%</span>
            <input class="range-control" type="range" min="0" max="1" step="0.01" :value="selectedClip.transform.opacity" :style="{ '--fill': `${selectedClip.transform.opacity * 100}%` }" @input="updateTransform('opacity', numberValue($event))" />
          </label>
          <label class="grid gap-2">
            <span class="text-[#8a8a8a]">Volume {{ Math.round(selectedClip.volume * 100) }}%</span>
            <input class="range-control" type="range" min="0" max="1" step="0.01" :value="selectedClip.volume" :style="{ '--fill': `${selectedClip.volume * 100}%` }" @input="emit('updateClip', selectedClip.id, { volume: numberValue($event) })" />
          </label>
          <label class="flex items-center justify-between text-[#525252]">
            Muted
            <input type="checkbox" :checked="selectedClip.muted" @change="emit('updateClip', selectedClip.id, { muted: booleanValue($event) })" />
          </label>
          <label class="grid gap-2">
            <span class="text-[#8a8a8a]">{{ selectedClip.effects[0]?.label ?? "Effect" }}</span>
            <input class="range-control" type="range" min="0" max="100" :value="selectedClip.effects[0]?.intensity ?? 0" :style="{ '--fill': `${selectedClip.effects[0]?.intensity ?? 0}%` }" @input="updateEffectIntensity(numberValue($event))" />
          </label>
        </div>
      </details>
    </section>

    <section v-else class="grid place-items-center p-6 text-center text-[12px] leading-[1.6] text-[#8a8a8a]">
      选择时间线片段后，可以调整声音、速度、画面变换、字幕和效果参数。
    </section>
  </aside>
</template>
