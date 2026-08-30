<script setup lang="ts">
import { computed, onUnmounted, ref, watch } from "vue";
import { isTauri } from "@tauri-apps/api/core";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { Check, ChevronDown, GripVertical, Image as ImageIcon, Plus, Trash2, X } from "@lucide/vue";
import type { MediaAsset } from "../../types/editor";
import type { DynamicComicAspectRatio } from "../../lib/dynamicComicImport";
import { reorderDynamicComicAssetIds } from "../../lib/dynamicComicImport";

const props = defineProps<{ open: boolean; assets: MediaAsset[]; error?: string }>();
const emit = defineEmits<{
  close: [];
  confirm: [request: { assetIds: string[]; aspectRatio: DynamicComicAspectRatio; duration: number }];
  addFiles: [files: File[]];
  addPaths: [paths: string[]];
}>();

const orderedIds = ref<string[]>([]);
const aspectRatio = ref<DynamicComicAspectRatio>("16:9");
const isAspectMenuOpen = ref(false);
const duration = ref(4);
const draggedId = ref("");
const dragTargetId = ref("");
const dragTargetAfter = ref(false);
const sequencePanel = ref<HTMLElement>();
const addImageInput = ref<HTMLInputElement>();
const dragPointerX = ref(0);
const dragPointerY = ref(0);
let pointerDrag: { pointerId: number; startX: number; startY: number } | undefined;
let knownAssetIds = new Set<string>();

watch(() => props.open, (open) => {
  if (open) {
    orderedIds.value = props.assets.map((asset) => asset.id);
    knownAssetIds = new Set(orderedIds.value);
  } else {
    knownAssetIds.clear();
  }
}, { immediate: true });

watch(() => props.assets, (assets) => {
  if (!props.open) return;
  const addedIds = assets.map((asset) => asset.id).filter((id) => !knownAssetIds.has(id));
  if (addedIds.length) orderedIds.value = [...orderedIds.value, ...addedIds];
  knownAssetIds = new Set(assets.map((asset) => asset.id));
});

const orderedAssets = computed(() => orderedIds.value.flatMap((id) => {
  const asset = props.assets.find((item) => item.id === id);
  return asset ? [asset] : [];
}));
const totalDuration = computed(() => orderedAssets.value.length * duration.value);
const draggedAsset = computed(() => props.assets.find((asset) => asset.id === draggedId.value));
const aspectRatioOptions: Array<{ value: DynamicComicAspectRatio; label: string }> = [
  { value: "16:9", label: "16:9 横屏" },
  { value: "9:16", label: "9:16 竖屏" },
  { value: "1:1", label: "1:1 方形" },
];
const selectedAspectLabel = computed(() => aspectRatioOptions.find((option) => option.value === aspectRatio.value)?.label ?? "16:9 横屏");
const dragGhostStyle = computed(() => {
  const panel = sequencePanel.value;
  const rect = panel?.getBoundingClientRect();
  const ghostWidth = 136;
  const ghostHeight = 112;
  const inset = 8;
  const preferredLeft = dragPointerX.value + 14;
  const preferredTop = dragPointerY.value + 14;
  return {
    left: `${rect ? Math.min(Math.max(preferredLeft, rect.left + inset), rect.right - ghostWidth - inset) : preferredLeft}px`,
    top: `${rect ? Math.min(Math.max(preferredTop, rect.top + inset), rect.bottom - ghostHeight - inset) : preferredTop}px`,
  };
});

function beginPointerSort(assetId: string, event: PointerEvent) {
  if (event.button !== 0 || !event.isPrimary) return;
  event.preventDefault();
  pointerDrag = { pointerId: event.pointerId, startX: event.clientX, startY: event.clientY };
  dragPointerX.value = event.clientX;
  dragPointerY.value = event.clientY;
  draggedId.value = assetId;
  window.addEventListener("pointermove", updatePointerSort);
  window.addEventListener("pointerup", finishPointerSort);
  window.addEventListener("pointercancel", finishPointerSort);
}

function updatePointerSort(event: PointerEvent) {
  if (!pointerDrag || event.pointerId !== pointerDrag.pointerId) return;
  event.preventDefault();
  dragPointerX.value = event.clientX;
  dragPointerY.value = event.clientY;
  const card = document.elementFromPoint(event.clientX, event.clientY)?.closest<HTMLElement>("[data-sequence-asset-id]");
  const targetId = card?.dataset.sequenceAssetId ?? "";
  if (!card || !targetId || targetId === draggedId.value) {
    dragTargetId.value = "";
    return;
  }
  const rect = card.getBoundingClientRect();
  const after = Math.abs(event.clientY - (rect.top + rect.height / 2)) > rect.height * 0.35
    ? event.clientY > rect.top + rect.height / 2
    : event.clientX > rect.left + rect.width / 2;
  dragTargetId.value = targetId;
  dragTargetAfter.value = after;
  orderedIds.value = reorderDynamicComicAssetIds(orderedIds.value, draggedId.value, targetId, after);
}

function finishPointerSort(event?: PointerEvent) {
  if (event && pointerDrag && event.pointerId !== pointerDrag.pointerId) return;
  pointerDrag = undefined;
  draggedId.value = "";
  dragTargetId.value = "";
  window.removeEventListener("pointermove", updatePointerSort);
  window.removeEventListener("pointerup", finishPointerSort);
  window.removeEventListener("pointercancel", finishPointerSort);
}

onUnmounted(finishPointerSort);

function confirm() {
  if (!orderedIds.value.length || !Number.isFinite(duration.value) || duration.value < 0.5) return;
  emit("confirm", { assetIds: [...orderedIds.value], aspectRatio: aspectRatio.value, duration: duration.value });
}

function selectAspectRatio(value: DynamicComicAspectRatio) {
  aspectRatio.value = value;
  isAspectMenuOpen.value = false;
}

async function addImages() {
  if (isTauri()) {
    const selected = await openDialog({
      title: "新增动态漫图片",
      multiple: true,
      directory: false,
      filters: [{ name: "图片", extensions: ["jpg", "jpeg", "png", "webp", "gif"] }],
    });
    const paths = Array.isArray(selected) ? selected : selected ? [selected] : [];
    if (paths.length) emit("addPaths", paths);
    return;
  }
  addImageInput.value?.click();
}

function handleAddImageInput(event: Event) {
  const input = event.target as HTMLInputElement;
  const files = Array.from(input.files ?? []).filter((file) => file.type.startsWith("image/") || /\.(jpe?g|png|webp|gif)$/i.test(file.name));
  if (files.length) emit("addFiles", files);
  input.value = "";
}
</script>

<template>
  <UiModal :open="open" title="导入图片序列" class="w-auto max-w-none bg-transparent p-0 ring-0 shadow-none" :ui="{ overlay: 'z-[75] bg-black/65 backdrop-blur-sm', content: 'z-[75] max-h-none' }" @update:open="!$event && emit('close')">
    <template #content>
      <section class="flex max-h-[calc(100dvh_-_40px)] w-[min(820px,calc(100vw_-_28px))] flex-col overflow-hidden rounded-xl border border-[#263041] bg-[#0d1119] shadow-[0_28px_90px_rgb(0_0_0/0.58)]">
        <header class="flex items-center justify-between gap-4 border-b border-[#263041] px-5 py-4">
          <div>
            <p class="text-[9px] font-black uppercase tracking-[0.2em] text-[#22d3ee]">Dynamic comic</p>
            <h2 class="mt-1 text-[16px] font-black text-[#f3f4f6]">校对图片镜头序列</h2>
            <p class="mt-1 text-[10px] font-semibold text-[#748198]">已按文件名自然排序；拖动卡片可在创建前调整。</p>
          </div>
          <UiButton color="neutral" variant="outline" square size="sm" type="button" aria-label="取消导入图片序列" @click="emit('close')"><X :size="15" /></UiButton>
        </header>

        <div class="grid min-h-0 grid-cols-[minmax(0,1fr)_220px] max-[700px]:grid-cols-1">
          <div ref="sequencePanel" class="editor-scrollbar min-h-[260px] overflow-y-auto p-4">
            <div v-if="orderedAssets.length" class="grid grid-cols-4 gap-2 max-[700px]:grid-cols-3 max-[480px]:grid-cols-2">
              <article v-for="(asset, index) in orderedAssets" :key="asset.id" :data-sequence-asset-id="asset.id" class="group relative overflow-hidden rounded-lg border bg-[#121925] transition-[transform,border-color,box-shadow,background-color]" :class="[draggedId === asset.id ? 'border-dashed border-[#22d3ee] bg-[#0b1d28] shadow-[inset_0_0_0_1px_rgb(34_211_238/0.18)]' : 'border-[#293347] hover:border-[#22d3ee]/70', dragTargetId === asset.id ? (dragTargetAfter ? 'translate-x-1' : '-translate-x-1') : '']">
                <div class="relative aspect-video overflow-hidden bg-[#081018]" :class="draggedId === asset.id ? 'invisible' : ''">
                  <img :src="asset.thumbnailUrl || asset.url" :alt="asset.name" class="size-full object-cover" />
                  <span class="absolute left-1.5 top-1.5 rounded bg-black/75 px-1.5 py-0.5 font-mono text-[9px] font-black text-white">{{ index + 1 }}</span>
                  <button type="button" class="absolute right-1.5 top-1.5 grid size-7 touch-none cursor-grab place-items-center rounded bg-black/75 text-[#cbd5e1] active:cursor-grabbing" :aria-label="`拖动调整 ${asset.name} 的顺序`" @pointerdown="beginPointerSort(asset.id, $event)"><GripVertical :size="14" /></button>
                </div>
                <div class="flex items-center gap-1.5 px-2 py-2" :class="draggedId === asset.id ? 'invisible' : ''">
                  <ImageIcon :size="12" class="shrink-0 text-[#22d3ee]" />
                  <span class="min-w-0 flex-1 truncate text-[10px] font-bold text-[#cbd5e1]" :title="asset.name">{{ asset.name }}</span>
                  <button type="button" class="text-[#68758a] hover:text-[#fb7185]" :aria-label="`移除 ${asset.name}`" @click="orderedIds = orderedIds.filter((id) => id !== asset.id)"><Trash2 :size="12" /></button>
                </div>
              </article>
              <button type="button" class="grid min-h-[108px] place-items-center rounded-lg border border-dashed border-[#334155] bg-[#0d1520] text-[#718096] transition hover:border-[#22d3ee] hover:bg-[#0b1d28] hover:text-[#67e8f9]" aria-label="新增图片" @click="addImages">
                <span class="grid justify-items-center gap-1.5"><span class="grid size-8 place-items-center rounded-full border border-current"><Plus :size="16" /></span><strong class="text-[10px]">新增图片</strong></span>
              </button>
            </div>
            <button v-else type="button" class="grid min-h-[240px] w-full place-items-center rounded-lg border border-dashed border-[#334155] bg-[#0d1520] text-[#718096] transition hover:border-[#22d3ee] hover:bg-[#0b1d28] hover:text-[#67e8f9]" aria-label="图片已全部移除，新增图片" @click="addImages">
              <span class="grid justify-items-center gap-2"><span class="grid size-10 place-items-center rounded-full border border-current"><Plus :size="18" /></span><strong class="text-[11px]">图片已全部移除 · 新增图片</strong></span>
            </button>
            <input ref="addImageInput" class="sr-only" type="file" multiple accept="image/*,.jpg,.jpeg,.png,.webp,.gif" @change="handleAddImageInput" />
          </div>

          <aside class="grid content-start gap-5 border-l border-[#263041] bg-[#101620] p-4 max-[700px]:border-l-0 max-[700px]:border-t">
            <div class="grid gap-2 text-[10px] font-black text-[#9aa6b8]">
              <span id="dynamic-comic-aspect-label">工程画幅</span>
              <UiPopover
                :open="isAspectMenuOpen"
                :content="{ side: 'bottom', align: 'start', sideOffset: 6, collisionPadding: 12 }"
                :ui="{ content: 'z-[90] w-[196px] rounded-lg border border-[#303b50] bg-[#111827] p-1.5 shadow-[0_18px_50px_rgb(0_0_0/0.5)]' }"
                @update:open="isAspectMenuOpen = $event"
              >
                <UiButton color="neutral" variant="outline" size="sm" class="h-9 w-full justify-between rounded-md border-[#303b50] bg-[#0a1018] px-3 text-[11px] font-bold text-[#e5e7eb] hover:bg-[#121b28]" type="button" aria-haspopup="listbox" aria-labelledby="dynamic-comic-aspect-label" :aria-expanded="isAspectMenuOpen">
                  {{ selectedAspectLabel }}
                  <ChevronDown :size="13" class="text-[#718096] transition-transform" :class="isAspectMenuOpen ? 'rotate-180' : ''" />
                </UiButton>
                <template #content>
                  <div role="listbox" aria-labelledby="dynamic-comic-aspect-label" class="grid gap-1">
                    <UiButton v-for="option in aspectRatioOptions" :key="option.value" color="neutral" :variant="aspectRatio === option.value ? 'soft' : 'ghost'" size="sm" class="w-full justify-between px-2.5 text-left text-[11px]" :class="aspectRatio === option.value ? 'bg-[#123047] text-[#67e8f9]' : 'text-[#cbd5e1] hover:bg-[#1a2433]'" type="button" role="option" :aria-selected="aspectRatio === option.value" @click="selectAspectRatio(option.value)">
                      {{ option.label }}
                      <Check v-if="aspectRatio === option.value" :size="13" />
                    </UiButton>
                  </div>
                </template>
              </UiPopover>
            </div>
            <label class="grid gap-2 text-[10px] font-black text-[#9aa6b8]">默认镜头时长
              <span class="flex items-center gap-2"><input v-model.number="duration" type="number" min="0.5" max="60" step="0.5" class="h-9 min-w-0 flex-1 rounded-md border border-[#303b50] bg-[#0a1018] px-3 text-[11px] font-bold text-[#e5e7eb]" /><span class="text-[10px] text-[#68758a]">秒</span></span>
            </label>
            <div class="rounded-lg border border-[#26364b] bg-[#0b1824] p-3 text-[10px] text-[#7f8da3]">
              <strong class="block text-[12px] text-[#dbeafe]">{{ orderedAssets.length }} 个镜头</strong>
              <span class="mt-1 block">预计总时长 {{ totalDuration.toFixed(1) }} 秒</span>
            </div>
            <p v-if="error" class="rounded-md border border-[#7f1d1d] bg-[#2a1116] p-2 text-[10px] font-semibold text-[#fca5a5]">{{ error }}</p>
          </aside>
        </div>

        <footer class="flex items-center justify-between gap-3 border-t border-[#263041] px-5 py-4">
          <span class="text-[10px] font-semibold text-[#68758a]">取消只保留已托管素材，不会创建镜头或片段。</span>
          <div class="flex gap-2"><UiButton color="neutral" variant="outline" size="sm" type="button" @click="emit('close')">取消</UiButton><UiButton color="secondary" variant="solid" size="sm" type="button" :disabled="!orderedAssets.length || duration < 0.5" @click="confirm">创建镜头序列</UiButton></div>
        </footer>
      </section>
      <Teleport to="body">
        <div v-if="draggedAsset" class="pointer-events-none fixed z-[95] w-[136px] -translate-y-3 overflow-hidden rounded-lg border border-[#67e8f9] bg-[#111827]/95 shadow-[0_18px_48px_rgb(0_0_0/0.55),0_0_24px_rgb(34_211_238/0.2)] backdrop-blur" :style="dragGhostStyle" aria-hidden="true">
          <div class="aspect-video overflow-hidden bg-[#081018]"><img :src="draggedAsset.thumbnailUrl || draggedAsset.url" alt="" class="size-full object-cover" /></div>
          <div class="flex items-center gap-1.5 px-2 py-2"><GripVertical :size="12" class="text-[#67e8f9]" /><span class="truncate text-[10px] font-bold text-[#e5e7eb]">{{ draggedAsset.name }}</span></div>
        </div>
      </Teleport>
    </template>
  </UiModal>
</template>
