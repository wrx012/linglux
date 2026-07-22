<script setup lang="ts">
import { computed } from "vue";
import { Droplet, Expand, Type, Volume2 } from "@lucide/vue";
import { createDefaultTextClipStyle } from "../../lib/editorProject";
import type { TextClipStyle, TimelineClip } from "../../types/editor";

const props = defineProps<{
  selectedClip?: TimelineClip;
}>();

const emit = defineEmits<{
  updateClip: [clipId: string, patch: Partial<TimelineClip>];
}>();

const fontOptions = ["Arial", "Inter", "Georgia", "Impact", "Courier New", "Noto Sans SC"];
const isTextClip = computed(() => props.selectedClip?.type === "caption");
const textStyle = computed<TextClipStyle>(() => ({
  ...createDefaultTextClipStyle(),
  ...props.selectedClip?.textStyle,
}));
const workstationInputUi = {
  root: "w-full",
  base: "h-10 w-full rounded-lg bg-[#0f131c] px-3 text-[13px] font-semibold text-[#d8deea] ring-[#2a3344]",
};
const workstationSelectUi = {
  base: "h-10 rounded-lg bg-[#0f131c] px-3 text-[13px] font-semibold text-[#d8deea] ring-[#2a3344]",
  content: "z-[80] bg-[#111620] ring-[#2a3344]",
};

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

function updateTextContent(value: string | number | null) {
  if (!props.selectedClip) {
    return;
  }

  emit("updateClip", props.selectedClip.id, { captionText: String(value ?? "") });
}

function updateTextStyle(key: keyof TextClipStyle, value: TextClipStyle[keyof TextClipStyle]) {
  if (!props.selectedClip) {
    return;
  }

  emit("updateClip", props.selectedClip.id, {
    textStyle: {
      ...textStyle.value,
      [key]: value,
    },
  });
}

function updateFontFamily(value: string | number) {
  updateTextStyle("fontFamily", String(value));
}

function updateTextStyleNumber(
  key: "fontSize" | "letterSpacing" | "lineHeight" | "backgroundWidth" | "backgroundHeight" | "backgroundXOffset" | "backgroundYOffset" | "backgroundCornerRadius",
  value: number | null | undefined,
) {
  if (value == null || !Number.isFinite(value)) {
    return;
  }

  updateTextStyle(key, value);
}

function updateTextBackgroundEnabled(value: boolean) {
  updateTextStyle("backgroundEnabled", value);
}

function updateTextStyleColor(key: "color" | "backgroundColor", event: Event) {
  const input = event.target as HTMLInputElement;
  const color = normalizeHexColor(input.value);

  if (!color) {
    return;
  }

  updateTextStyle(key, color);
}

function normalizeHexColor(value: string) {
  const trimmedValue = value.trim();
  const color = trimmedValue.startsWith("#") ? trimmedValue : `#${trimmedValue}`;

  if (!/^#[0-9a-f]{6}$/i.test(color)) {
    return undefined;
  }

  return color.toUpperCase();
}

function safeColorValue(color: string) {
  return normalizeHexColor(color) ?? "#FFFFFF";
}

function hexLabel(color: string) {
  return safeColorValue(color).replace("#", "");
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

function updateTransformNumber(key: "x" | "y" | "scale" | "rotation" | "opacity", value: number | null | undefined) {
  if (value != null && Number.isFinite(value)) {
    updateTransform(key, value);
  }
}

function updateClipVolume(value: number | number[] | undefined) {
  if (!props.selectedClip || typeof value !== "number") {
    return;
  }

  emit("updateClip", props.selectedClip.id, { volume: value });
}

function updateClipMuted(value: boolean) {
  if (props.selectedClip) {
    emit("updateClip", props.selectedClip.id, { muted: value });
  }
}

function updateClipEffectIntensity(value: number | number[] | undefined) {
  if (typeof value === "number") {
    updateEffectIntensity(value);
  }
}
</script>

<template>
  <aside class="min-h-0 overflow-hidden bg-[#111620] text-[#d8deea]" aria-label="剪辑属性面板">
    <section v-if="selectedClip" class="grid h-full min-h-0">
      <div v-if="isTextClip" class="min-h-0 overflow-y-auto [scrollbar-color:#2a3344_#111620]">
        <section class="border-b border-[#262c38]">
          <header class="flex items-center gap-2 px-5 py-3 text-left text-[12px] font-black text-[#d8deea]">
            <Type :size="14" class="text-[#c084fc]" />
            <span>内容</span>
          </header>
          <div class="px-5 pb-4">
            <UTextarea
              class="w-full"
              :model-value="selectedClip.captionText ?? selectedClip.name"
              :rows="4"
              :ui="{ base: 'h-28 resize-none rounded-lg bg-[#0f131c] px-4 py-3 text-[14px] font-semibold leading-6 text-[#d8deea] ring-[#2a3344] placeholder:text-[#5f6b7d]' }"
              @update:model-value="updateTextContent"
            />
          </div>
        </section>

        <section class="border-b border-[#262c38]">
          <header class="flex items-center gap-2 px-5 py-3 text-left text-[12px] font-black text-[#d8deea]">
            <Type :size="14" class="text-[#60a5fa]" />
            <span>字体</span>
          </header>
          <div class="grid gap-4 px-5 pb-4">
            <label class="grid gap-2">
              <span class="text-[12px] font-bold text-[#8993a3]">字体</span>
              <USelect
                :model-value="textStyle.fontFamily"
                :items="fontOptions"
                class="w-full"
                :ui="workstationSelectUi"
                @update:model-value="updateFontFamily"
              >
                <template #leading>
                  <Type :size="14" class="text-[#8993a3]" />
                </template>
              </USelect>
            </label>

            <label class="grid gap-2">
              <span class="text-[12px] font-bold text-[#8993a3]">字号</span>
              <UInputNumber
                :model-value="textStyle.fontSize"
                :min="8"
                :max="180"
                :step="1"
                :increment="false"
                :decrement="false"
                :ui="workstationInputUi"
                @update:model-value="updateTextStyleNumber('fontSize', $event)"
              />
            </label>

            <label class="grid gap-2">
              <span class="text-[12px] font-bold text-[#8993a3]">颜色</span>
              <span class="grid h-10 grid-cols-[28px_minmax(0,1fr)] items-center rounded-xl border border-[#2a3344] bg-[#0f131c] px-3 text-[13px] font-semibold text-[#d8deea]">
                <input
                  class="size-5 rounded-md border border-white/20 bg-transparent p-0"
                  type="color"
                  :value="safeColorValue(textStyle.color)"
                  aria-label="文字颜色"
                  @input="updateTextStyleColor('color', $event)"
                />
                <input
                  class="min-w-0 bg-transparent font-mono text-[13px] font-semibold uppercase text-[#d8deea] outline-none"
                  type="text"
                  :value="hexLabel(textStyle.color)"
                  @change="updateTextStyleColor('color', $event)"
                />
              </span>
            </label>
          </div>
        </section>

        <section class="border-b border-[#262c38]">
          <header class="px-5 py-3 text-left text-[12px] font-black text-[#d8deea]">间距</header>
          <div class="grid grid-cols-2 gap-3 px-5 pb-4">
            <label class="grid gap-2">
              <span class="text-[12px] font-bold text-[#8993a3]">字距</span>
              <UInputNumber
                :model-value="textStyle.letterSpacing"
                :min="-20"
                :max="60"
                :step="0.5"
                :increment="false"
                :decrement="false"
                :ui="workstationInputUi"
                @update:model-value="updateTextStyleNumber('letterSpacing', $event)"
              />
            </label>
            <label class="grid gap-2">
              <span class="text-[12px] font-bold text-[#8993a3]">行高</span>
              <UInputNumber
                :model-value="textStyle.lineHeight"
                :min="0.8"
                :max="3"
                :step="0.05"
                :increment="false"
                :decrement="false"
                :ui="workstationInputUi"
                @update:model-value="updateTextStyleNumber('lineHeight', $event)"
              />
            </label>
          </div>
        </section>

        <section class="border-b border-[#262c38]">
          <header class="flex items-center justify-between gap-3 px-5 py-3 text-left text-[12px] font-black text-[#d8deea]">
            <span>背景</span>
            <USwitch
              :model-value="textStyle.backgroundEnabled"
              size="sm"
              :aria-label="textStyle.backgroundEnabled ? '隐藏文字背景' : '显示文字背景'"
              :title="textStyle.backgroundEnabled ? '隐藏文字背景' : '显示文字背景'"
              @update:model-value="updateTextBackgroundEnabled"
            />
          </header>
          <div class="grid gap-4 px-5 pb-4">
            <label class="grid gap-2">
              <span class="text-[12px] font-bold text-[#8993a3]">颜色</span>
              <span class="grid h-10 grid-cols-[28px_minmax(0,1fr)] items-center rounded-xl border border-[#2a3344] bg-[#0f131c] px-3 text-[13px] font-semibold text-[#d8deea]" :class="textStyle.backgroundEnabled ? '' : 'opacity-45'">
                <input
                  class="size-5 rounded-md border border-white/20 bg-transparent p-0"
                  type="color"
                  :value="safeColorValue(textStyle.backgroundColor)"
                  :disabled="!textStyle.backgroundEnabled"
                  aria-label="文字背景颜色"
                  @input="updateTextStyleColor('backgroundColor', $event)"
                />
                <input
                  class="min-w-0 bg-transparent font-mono text-[13px] font-semibold uppercase text-[#d8deea] outline-none disabled:cursor-not-allowed"
                  type="text"
                  :value="hexLabel(textStyle.backgroundColor)"
                  :disabled="!textStyle.backgroundEnabled"
                  @change="updateTextStyleColor('backgroundColor', $event)"
                />
              </span>
            </label>
            <div class="grid grid-cols-2 gap-3" :class="textStyle.backgroundEnabled ? '' : 'opacity-45'">
              <label class="grid gap-2">
                <span class="text-[12px] font-bold text-[#8993a3]">宽度</span>
                <UInputNumber
                  :model-value="textStyle.backgroundWidth"
                  :min="0"
                  :max="1200"
                  :step="1"
                  :increment="false"
                  :decrement="false"
                  :disabled="!textStyle.backgroundEnabled"
                  :ui="workstationInputUi"
                  @update:model-value="updateTextStyleNumber('backgroundWidth', $event)"
                />
              </label>
              <label class="grid gap-2">
                <span class="text-[12px] font-bold text-[#8993a3]">高度</span>
                <UInputNumber
                  :model-value="textStyle.backgroundHeight"
                  :min="0"
                  :max="800"
                  :step="1"
                  :increment="false"
                  :decrement="false"
                  :disabled="!textStyle.backgroundEnabled"
                  :ui="workstationInputUi"
                  @update:model-value="updateTextStyleNumber('backgroundHeight', $event)"
                />
              </label>
              <label class="grid gap-2">
                <span class="text-[12px] font-bold text-[#8993a3]">X 偏移</span>
                <UInputNumber
                  :model-value="textStyle.backgroundXOffset"
                  :min="-600"
                  :max="600"
                  :step="1"
                  :increment="false"
                  :decrement="false"
                  :disabled="!textStyle.backgroundEnabled"
                  :ui="workstationInputUi"
                  @update:model-value="updateTextStyleNumber('backgroundXOffset', $event)"
                />
              </label>
              <label class="grid gap-2">
                <span class="text-[12px] font-bold text-[#8993a3]">Y 偏移</span>
                <UInputNumber
                  :model-value="textStyle.backgroundYOffset"
                  :min="-600"
                  :max="600"
                  :step="1"
                  :increment="false"
                  :decrement="false"
                  :disabled="!textStyle.backgroundEnabled"
                  :ui="workstationInputUi"
                  @update:model-value="updateTextStyleNumber('backgroundYOffset', $event)"
                />
              </label>
            </div>
            <label class="grid gap-2" :class="textStyle.backgroundEnabled ? '' : 'opacity-45'">
              <span class="text-[12px] font-bold text-[#8993a3]">圆角</span>
              <UInputNumber
                :model-value="textStyle.backgroundCornerRadius"
                :min="0"
                :max="240"
                :step="1"
                :increment="false"
                :decrement="false"
                :disabled="!textStyle.backgroundEnabled"
                :ui="workstationInputUi"
                @update:model-value="updateTextStyleNumber('backgroundCornerRadius', $event)"
              />
            </label>
          </div>
        </section>
      </div>

      <div v-else class="min-h-0 space-y-4 overflow-y-auto px-5 py-5 [scrollbar-color:#2a3344_#111620]">
        <details class="rounded-2xl border border-[#262c38] bg-[#151a24]" open>
          <summary class="flex cursor-pointer list-none items-center justify-between px-4 py-4 text-[14px] font-black text-highlighted">
            <span class="inline-flex items-center gap-2">
              <Expand :size="16" class="text-[#34d399]" />
              画面变换
            </span>
            <span class="text-[#687386]">⌄</span>
          </summary>
          <div class="grid grid-cols-2 gap-3 border-t border-[#262c38] p-4">
            <label class="grid gap-2">
              <span class="text-[12px] font-bold text-[#8993a3]">X</span>
              <UInputNumber :model-value="selectedClip.transform.x" :increment="false" :decrement="false" :ui="workstationInputUi" @update:model-value="updateTransformNumber('x', $event)" />
            </label>
            <label class="grid gap-2">
              <span class="text-[12px] font-bold text-[#8993a3]">Y</span>
              <UInputNumber :model-value="selectedClip.transform.y" :increment="false" :decrement="false" :ui="workstationInputUi" @update:model-value="updateTransformNumber('y', $event)" />
            </label>
            <label class="grid gap-2">
              <span class="text-[12px] font-bold text-[#8993a3]">缩放</span>
              <UInputNumber :model-value="selectedClip.transform.scale" :min="0.1" :max="4" :step="0.05" :increment="false" :decrement="false" :ui="workstationInputUi" @update:model-value="updateTransformNumber('scale', $event)" />
            </label>
            <label class="grid gap-2">
              <span class="text-[12px] font-bold text-[#8993a3]">旋转</span>
              <UInputNumber :model-value="selectedClip.transform.rotation" :increment="false" :decrement="false" :ui="workstationInputUi" @update:model-value="updateTransformNumber('rotation', $event)" />
            </label>
          </div>
        </details>

        <details class="rounded-2xl border border-[#262c38] bg-[#151a24]" open>
          <summary class="flex cursor-pointer list-none items-center justify-between px-4 py-4 text-[14px] font-black text-highlighted">
            <span class="inline-flex items-center gap-2">
              <Volume2 :size="16" class="text-[#60a5fa]" />
              片段
            </span>
            <span class="text-[#687386]">⌄</span>
          </summary>
          <div class="grid gap-4 border-t border-[#262c38] p-4 text-[13px]">
            <label class="grid gap-2">
              <span class="font-bold text-[#8993a3]">透明度 {{ Math.round(selectedClip.transform.opacity * 100) }}%</span>
              <USlider :model-value="selectedClip.transform.opacity" :min="0" :max="1" :step="0.01" size="sm" @update:model-value="updateTransformNumber('opacity', $event)" />
            </label>
            <label class="grid gap-2">
              <span class="font-bold text-[#8993a3]">音量 {{ Math.round(selectedClip.volume * 100) }}%</span>
              <USlider :model-value="selectedClip.volume" :min="0" :max="1" :step="0.01" size="sm" @update:model-value="updateClipVolume" />
            </label>
            <div class="flex items-center justify-between rounded-lg border border-[#2a3344] bg-[#0f131c] px-3 py-3 font-bold text-[#8993a3]">
              静音
              <USwitch :model-value="selectedClip.muted" size="sm" aria-label="片段静音" @update:model-value="updateClipMuted" />
            </div>
            <label class="grid gap-2">
              <span class="inline-flex items-center gap-2 font-bold text-[#8993a3]">
                <Droplet :size="15" class="text-[#34d399]" />
                {{ selectedClip.effects[0]?.label ?? "Effect" }}
              </span>
              <USlider :model-value="selectedClip.effects[0]?.intensity ?? 0" :min="0" :max="100" :step="1" size="sm" @update:model-value="updateClipEffectIntensity" />
            </label>
          </div>
        </details>
      </div>
    </section>

    <section v-else class="grid h-full place-items-center p-8 text-center">
      <div class="max-w-[260px]">
        <span class="mx-auto grid size-14 place-items-center rounded-2xl border border-[#262c38] bg-[#151a24] text-[#687386]">
          <Expand :size="22" />
        </span>
        <h3 class="mt-4 text-[15px] font-black text-highlighted">未选择片段</h3>
        <p class="mt-2 text-[12px] font-semibold leading-6 text-[#687386]">
          选择时间线片段后，可以调整声音、速度、画面变换、字幕和效果参数。
        </p>
      </div>
    </section>
  </aside>
</template>
