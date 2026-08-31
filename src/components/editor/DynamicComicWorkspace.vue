<script setup lang="ts">
import { computed, onUnmounted, ref } from "vue";
import { CheckCircle2, ChevronDown, ChevronUp, Copy, GripVertical, Mic2, Play, Plus, RotateCcw, Square, Trash2, Video, WandSparkles } from "@lucide/vue";
import type { CharacterVoiceProfile, DynamicComicShot, EditorProject, TtsEmotion, TtsStatus, TtsVoice } from "../../types/editor";
import { clampDynamicComicShotDuration, swapDynamicComicIds } from "../../lib/dynamicComicWorkspace";
import { createCharacterVoiceProfile, resolveShotSpeechSettings } from "../../lib/characterVoiceProfiles";
import { TTS_EMOTION_OPTIONS, TTS_VOICE_OPTIONS } from "../../lib/tts";

const props = defineProps<{
  project: EditorProject;
  selectedShotIds: string[];
  ttsStatus?: TtsStatus;
  ttsBusy?: boolean;
  ttsProgress?: number;
  ttsTaskStatus?: string;
  ttsError?: string;
  previewingSpeechShotId?: string;
}>();
const emit = defineEmits<{
  select: [id: string, additive: boolean];
  reorderSequence: [ids: string[]];
  duplicate: [id: string];
  delete: [ids: string[]];
  batchDuration: [ids: string[], duration: number];
  updateShot: [id: string, patch: Partial<DynamicComicShot>];
  createCharacter: [profile: CharacterVoiceProfile];
  updateCharacter: [id: string, patch: Partial<CharacterVoiceProfile>];
  deleteCharacter: [id: string];
  auditionShot: [id: string];
  toggleSpeechPreview: [id: string];
  setupTts: [];
  openTimeline: [shotId?: string];
}>();

const draggedId = ref<string | null>(null);
const previewOrderedIds = ref<string[]>([]);
const dragTargetId = ref<string | null>(null);
const dragPointerX = ref(0);
const dragPointerY = ref(0);
let pointerDrag: {
  pointerId: number;
  originalIds: string[];
  startX: number;
  startY: number;
  active: boolean;
  slots: Array<{ id: string; bounds: DOMRect }>;
} | undefined;
let suppressCardClick = false;
const batchDuration = ref(4);
const feedbackToastOpen = ref(false);
const feedbackToastTitle = ref("");
const feedbackToastDescription = ref("");
const isCreateCharacterOpen = ref(false);
const characterNameDraft = ref("");
const pendingCharacterShotId = ref<string | null>(null);
let feedbackToastTimer: number | undefined;
const resetOrderIds = ref(
  [...(props.project.dynamicComic?.shots ?? [])]
    .sort((left, right) => left.order - right.order)
    .map((shot) => shot.id),
);
const sourceShots = computed(() => [...(props.project.dynamicComic?.shots ?? [])].sort((a, b) => a.order - b.order));
const shots = computed(() => {
  const orderedIds = previewOrderedIds.value;
  if (!orderedIds.length) return sourceShots.value;
  const byId = new Map(sourceShots.value.map((shot) => [shot.id, shot]));
  return orderedIds.flatMap((id) => {
    const shot = byId.get(id);
    return shot ? [shot] : [];
  });
});
const selectedShot = computed(() => shots.value.find((shot) => shot.id === props.selectedShotIds[props.selectedShotIds.length - 1]));
const selectedSet = computed(() => new Set(props.selectedShotIds));
const characterProfiles = computed(() => props.project.dynamicComic?.characterVoiceProfiles ?? []);
const selectedCharacter = computed(() => characterProfiles.value.find((profile) => profile.id === selectedShot.value?.characterId));
const selectedSpeechSettings = computed(() => {
  const project = props.project.dynamicComic;
  return project && selectedShot.value ? resolveShotSpeechSettings(project, selectedShot.value) : undefined;
});
const selectedSpeechAsset = computed(() => selectedShot.value ? speechAsset(selectedShot.value) : undefined);
const characterOptions = computed(() => [
  { value: "", label: "未指定角色" },
  ...characterProfiles.value.map((profile) => ({ value: profile.id, label: profile.name })),
  { value: "__create_character__", label: "＋ 新建角色声线…" },
]);
const emotionOverrideOptions = computed(() => [
  { value: "", label: `继承角色默认（${selectedCharacter.value?.defaultEmotion || "未设置"}）` },
  { value: "natural", label: "自然" },
  { value: "gentle", label: "温柔" },
  { value: "cheerful", label: "愉快" },
  { value: "serious", label: "严肃" },
]);
const voiceOptions = TTS_VOICE_OPTIONS;
const defaultEmotionOptions = TTS_EMOTION_OPTIONS;
const workstationSelectUi = { base: "h-9 rounded-md bg-[#0b1018] px-2 text-xs text-[#e5e7eb] ring-[#2b3545]" };
const profileColorInputUi = {
  base: "role-color-swatch size-9 min-h-9 cursor-pointer rounded-lg border-[#2b3545] bg-[#0f151f] p-1 [&::-webkit-color-swatch-wrapper]:p-0 [&::-webkit-color-swatch]:rounded-md [&::-webkit-color-swatch]:border-0",
};
const profileNameInputUi = { base: "h-9 min-h-9 rounded-lg border-[#2b3545] bg-[#0f151f] px-3 text-xs" };
const speechActionLabel = computed(() => {
  if (!props.ttsStatus?.supported) return "当前设备不支持本地 TTS";
  if (props.ttsBusy || props.ttsStatus.state === "installing") return "正在准备本地语音模型…";
  if (props.ttsStatus.state !== "ready") {
    if (props.ttsStatus.legacyBytes) return "清理旧版并安装轻量语音模型";
    return props.ttsStatus.state === "error" ? "重新安装本地语音模型" : "安装本地语音模型";
  }
  if (!selectedCharacter.value) return "请先绑定角色";
  if (!selectedShot.value?.dialogue.trim()) return "请先输入台词";
  return selectedSpeechAsset.value ? "重新生成单条试听" : "生成单条试听";
});
const speechActionDisabled = computed(() => {
  if (!props.ttsStatus?.supported || props.ttsBusy || props.ttsStatus.state === "installing") return true;
  if (props.ttsStatus.state !== "ready") return false;
  return !selectedSpeechSettings.value || !selectedShot.value?.dialogue.trim();
});
const assetById = computed(() => new Map(props.project.assets.map((asset) => [asset.id, asset])));
const draggedShot = computed(() => sourceShots.value.find((shot) => shot.id === draggedId.value));
const canResetOrder = computed(() => {
  const currentIds = sourceShots.value.map((shot) => shot.id);
  const originalIds = resetOrderIds.value.filter((id) => currentIds.includes(id));
  const currentOriginalIds = currentIds.filter((id) => resetOrderIds.value.includes(id));
  return originalIds.some((id, index) => id !== currentOriginalIds[index]);
});

function thumbnail(shot: DynamicComicShot) {
  const asset = shot.visualAssetId ? assetById.value.get(shot.visualAssetId) : undefined;
  return asset?.thumbnailUrl || (asset?.type === "image" ? asset.url : "");
}

function speechAsset(shot: DynamicComicShot) {
  const asset = shot.speechAssetId ? assetById.value.get(shot.speechAssetId) : undefined;
  return asset?.type === "audio" ? asset : undefined;
}

function speechStatusLabel(shot: DynamicComicShot) {
  if (!shot.dialogue.trim()) return "无需配音";
  return speechAsset(shot) ? "已配音" : "待配音";
}

function characterName(shot: DynamicComicShot) {
  return characterProfiles.value.find((profile) => profile.id === shot.characterId)?.name || "未指定角色";
}

function applyDuration() {
  if (!props.selectedShotIds.length) return;
  emit("batchDuration", props.selectedShotIds, clampDynamicComicShotDuration(Number(batchDuration.value)));
}

function addCharacter() {
  emit("createCharacter", createCharacterVoiceProfile(characterProfiles.value.length));
}

function selectShotCharacter(shot: DynamicComicShot, value: string | number) {
  const characterId = String(value);
  if (characterId === "__create_character__") {
    pendingCharacterShotId.value = shot.id;
    characterNameDraft.value = `角色 ${characterProfiles.value.length + 1}`;
    isCreateCharacterOpen.value = true;
    return;
  }
  emit("updateShot", shot.id, { characterId: characterId || undefined, emotion: undefined, speechSpeed: undefined });
}

function confirmCreateCharacter() {
  const name = characterNameDraft.value.trim();
  const shotId = pendingCharacterShotId.value;
  if (!name || !shotId) return;
  const profile = { ...createCharacterVoiceProfile(characterProfiles.value.length), name };
  emit("createCharacter", profile);
  emit("updateShot", shotId, { characterId: profile.id, emotion: undefined, speechSpeed: undefined });
  isCreateCharacterOpen.value = false;
  pendingCharacterShotId.value = null;
  showFeedbackToast("角色已创建", `${profile.name} 已绑定到当前镜头`);
}

function closeCreateCharacter() {
  isCreateCharacterOpen.value = false;
  pendingCharacterShotId.value = null;
}

function updateShotEmotion(shot: DynamicComicShot, value: string | number) {
  emit("updateShot", shot.id, { emotion: String(value) || undefined });
}

function updateProfileVoice(profile: CharacterVoiceProfile, value: string | number) {
  updateCharacter(profile, "voice", String(value) as TtsVoice);
}

function updateProfileEmotion(profile: CharacterVoiceProfile, value: string | number) {
  updateCharacter(profile, "defaultEmotion", String(value) as TtsEmotion);
}

function runSpeechAction() {
  if (!props.ttsStatus?.supported || props.ttsBusy) return;
  if (props.ttsStatus.state !== "ready") {
    emit("setupTts");
    return;
  }
  if (selectedShot.value && selectedSpeechSettings.value && selectedShot.value.dialogue.trim()) {
    emit("auditionShot", selectedShot.value.id);
  }
}

function removeCharacter(profile: CharacterVoiceProfile) {
  const referencedCount = sourceShots.value.filter((shot) => shot.characterId === profile.id).length;
  emit("deleteCharacter", profile.id);
  showFeedbackToast("角色已删除", referencedCount ? `${referencedCount} 个镜头已保留台词并标记为未指定角色` : "角色声线档案已移除");
}

function updateCharacter<K extends keyof CharacterVoiceProfile>(profile: CharacterVoiceProfile, key: K, value: CharacterVoiceProfile[K]) {
  emit("updateCharacter", profile.id, { [key]: value });
}

function beginPointerSort(shotId: string, event: PointerEvent) {
  if (event.button !== 0 || !event.isPrimary) return;
  if ((event.target as HTMLElement).closest("button, input, textarea, select, a, [data-no-shot-drag]")) return;
  const originalIds = sourceShots.value.map((shot) => shot.id);
  const slots = [...document.querySelectorAll<HTMLElement>("[data-shot-id]")].map((card) => ({
    id: card.dataset.shotId ?? "",
    bounds: card.getBoundingClientRect(),
  })).filter((slot) => slot.id);
  pointerDrag = { pointerId: event.pointerId, originalIds, startX: event.clientX, startY: event.clientY, active: false, slots };
  draggedId.value = null;
  dragPointerX.value = event.clientX;
  dragPointerY.value = event.clientY;
  (event.currentTarget as HTMLElement).dataset.pendingDragShotId = shotId;
  window.addEventListener("pointermove", updatePointerSort);
  window.addEventListener("pointerup", finishPointerSort);
  window.addEventListener("pointercancel", cancelPointerSort);
}

function moveShot(id: string, direction: -1 | 1) {
  const index = shots.value.findIndex((shot) => shot.id === id);
  const neighbor = shots.value[index + direction];
  if (!neighbor) return;
  const next = swapDynamicComicIds(sourceShots.value.map((shot) => shot.id), id, neighbor.id);
  emit("reorderSequence", next);
}

function resetShotOrder() {
  if (!canResetOrder.value) return;
  const currentIds = sourceShots.value.map((shot) => shot.id);
  const currentSet = new Set(currentIds);
  const restoredIds = resetOrderIds.value.filter((id) => currentSet.has(id));
  const addedIds = currentIds.filter((id) => !resetOrderIds.value.includes(id));
  emit("reorderSequence", [...restoredIds, ...addedIds]);
}

function deleteShots(ids: string[]) {
  const existingIds = ids.filter((id) => sourceShots.value.some((shot) => shot.id === id));
  if (!existingIds.length) return;
  emit("delete", existingIds);
  showFeedbackToast(
    "镜头已删除",
    existingIds.length === 1 ? "镜头及关联主轨片段已移除" : `${existingIds.length} 个镜头及关联主轨片段已移除`,
  );
}

function duplicateShot(id: string) {
  const shot = sourceShots.value.find((item) => item.id === id);
  if (!shot) return;
  emit("duplicate", id);
  showFeedbackToast("镜头已复制", `镜头 ${shot.order + 1} 的副本已插入其后`);
}

function showFeedbackToast(title: string, description: string) {
  feedbackToastTitle.value = title;
  feedbackToastDescription.value = description;
  feedbackToastOpen.value = true;
  if (feedbackToastTimer !== undefined) window.clearTimeout(feedbackToastTimer);
  feedbackToastTimer = window.setTimeout(() => {
    feedbackToastOpen.value = false;
    feedbackToastTimer = undefined;
  }, 2400);
}

function updatePointerSort(event: PointerEvent) {
  if (!pointerDrag || event.pointerId !== pointerDrag.pointerId) return;
  if (!pointerDrag.active) {
    if (Math.hypot(event.clientX - pointerDrag.startX, event.clientY - pointerDrag.startY) < 5) return;
    const pendingCard = document.querySelector<HTMLElement>("[data-pending-drag-shot-id]");
    const shotId = pendingCard?.dataset.pendingDragShotId;
    if (!shotId) return;
    pointerDrag.active = true;
    draggedId.value = shotId;
    previewOrderedIds.value = [...pointerDrag.originalIds];
    pendingCard.removeAttribute("data-pending-drag-shot-id");
  }
  if (!draggedId.value) return;
  event.preventDefault();
  dragPointerX.value = event.clientX;
  dragPointerY.value = event.clientY;
  const target = closestDragSlot(event.clientX, event.clientY, pointerDrag.slots);
  if (!target || target.id === draggedId.value) {
    dragTargetId.value = null;
    return;
  }
  dragTargetId.value = target.id;
  previewOrderedIds.value = swapDynamicComicIds(pointerDrag.originalIds, draggedId.value, target.id);
}

function finishPointerSort(event?: PointerEvent) {
  if (event && pointerDrag && event.pointerId !== pointerDrag.pointerId) return;
  const wasActive = Boolean(pointerDrag?.active);
  const changed = wasActive && previewOrderedIds.value.some((id, index) => id !== pointerDrag!.originalIds[index]);
  if (changed) emit("reorderSequence", [...previewOrderedIds.value]);
  if (wasActive) {
    suppressCardClick = true;
    window.setTimeout(() => { suppressCardClick = false; }, 0);
  }
  cleanupPointerSort();
}

function cancelPointerSort(event?: PointerEvent) {
  if (event && pointerDrag && event.pointerId !== pointerDrag.pointerId) return;
  cleanupPointerSort();
}

function cleanupPointerSort() {
  document.querySelector<HTMLElement>("[data-pending-drag-shot-id]")?.removeAttribute("data-pending-drag-shot-id");
  pointerDrag = undefined;
  draggedId.value = null;
  dragTargetId.value = null;
  previewOrderedIds.value = [];
  window.removeEventListener("pointermove", updatePointerSort);
  window.removeEventListener("pointerup", finishPointerSort);
  window.removeEventListener("pointercancel", cancelPointerSort);
}

function selectShotFromCard(shotId: string, event: MouseEvent) {
  if (suppressCardClick) {
    event.preventDefault();
    return;
  }
  emit("select", shotId, event.metaKey || event.ctrlKey);
}

function closestDragSlot(x: number, y: number, slots: Array<{ id: string; bounds: DOMRect }>) {
  return slots.reduce<{ id: string; bounds: DOMRect; distance: number } | undefined>((closest, slot) => {
    const dx = x < slot.bounds.left ? slot.bounds.left - x : x > slot.bounds.right ? x - slot.bounds.right : 0;
    const dy = y < slot.bounds.top ? slot.bounds.top - y : y > slot.bounds.bottom ? y - slot.bounds.bottom : 0;
    const distance = Math.hypot(dx, dy);
    return !closest || distance < closest.distance ? { ...slot, distance } : closest;
  }, undefined);
}

onUnmounted(() => {
  cleanupPointerSort();
  if (feedbackToastTimer !== undefined) window.clearTimeout(feedbackToastTimer);
});
</script>

<template>
  <section class="grid min-h-0 grid-cols-[minmax(0,1fr)_320px] overflow-hidden bg-[#090c12] max-[820px]:grid-cols-1 max-[820px]:grid-rows-[minmax(440px,1fr)_360px]" aria-label="镜头编排工作区">
    <div class="grid min-h-0 grid-rows-[auto_minmax(0,1fr)] overflow-hidden border-r border-[#202835] max-[820px]:border-b max-[820px]:border-r-0">
      <header class="flex min-h-14 flex-wrap items-center gap-2 border-b border-[#202835] bg-[#0e131c] px-4 py-2">
        <div class="mr-auto min-w-0">
          <p class="text-[10px] font-black uppercase tracking-[0.18em] text-[#2dd4bf]">Shot board</p>
          <h2 class="text-sm font-black text-[#e5e7eb]">镜头编排 · {{ shots.length }} 镜</h2>
        </div>
        <label class="flex items-center gap-2 text-[10px] font-bold text-[#94a3b8]">默认时长
          <UiInputNumber v-model="batchDuration" class="h-8 w-20 px-2 text-right text-xs" :min="0.5" :max="60" :step="0.5" :increment="false" :decrement="false" />
        </label>
        <UiButton color="secondary" variant="soft" size="xs" type="button" :disabled="!selectedShotIds.length" @click="applyDuration">应用到所选</UiButton>
        <UiButton color="neutral" variant="ghost" size="xs" type="button" :disabled="!canResetOrder" title="恢复进入镜头编排时的顺序" @click="resetShotOrder"><RotateCcw :size="13" />重置排序</UiButton>
        <UiButton color="neutral" variant="outline" size="xs" type="button" @click="emit('openTimeline', selectedShot?.id)"><Video :size="13" />专业时间线</UiButton>
      </header>

      <div class="overflow-y-auto p-3" data-testid="dynamic-comic-shot-list">
        <div v-if="shots.length" class="grid grid-cols-[repeat(auto-fill,minmax(210px,1fr))] content-start gap-3" role="listbox" aria-label="动态漫镜头" aria-multiselectable="true">
          <article
            v-for="(shot, shotIndex) in shots"
            :key="shot.id"
            role="option"
            tabindex="0"
            :aria-selected="selectedSet.has(shot.id)"
            :aria-label="`镜头 ${shotIndex + 1}，${shot.characterId || '未指定角色'}，${shot.duration.toFixed(1)} 秒`"
            :data-shot-id="shot.id"
            class="group grid h-[260px] cursor-grab touch-pan-y select-none grid-rows-[126px_minmax(0,1fr)] overflow-hidden rounded-xl border bg-[#111722] transition active:cursor-grabbing"
            :class="[
              selectedSet.has(shot.id) ? 'border-[#2dd4bf] shadow-[0_0_0_1px_rgb(45_212_191/0.25)]' : 'border-[#273142] hover:border-[#426078]',
              draggedId === shot.id ? 'border-2 border-dashed border-[#2dd4bf] bg-[#0a1718] shadow-none [&>*]:invisible' : '',
              dragTargetId === shot.id ? 'ring-2 ring-[#2dd4bf] ring-offset-2 ring-offset-[#090c12]' : '',
            ]"
            @pointerdown="beginPointerSort(shot.id, $event)"
            @click="selectShotFromCard(shot.id, $event)"
            @keydown.enter.prevent="emit('select', shot.id, $event.metaKey || $event.ctrlKey)"
            @keydown.space.prevent="emit('select', shot.id, $event.metaKey || $event.ctrlKey)"
          >
            <div class="relative overflow-hidden bg-[#080b11]">
              <img v-if="thumbnail(shot)" :src="thumbnail(shot)" alt="" class="size-full object-cover" loading="lazy" draggable="false" />
              <div v-else class="grid size-full place-items-center text-[#465469]"><WandSparkles :size="28" /></div>
              <span class="absolute left-2 top-2 rounded-md bg-black/75 px-2 py-1 font-mono text-[10px] font-black text-white">#{{ shotIndex + 1 }}</span>
              <span class="pointer-events-none absolute right-2 top-2 flex items-center gap-1 rounded-md bg-black/75 px-2 py-1 text-[9px] font-bold text-[#dbeafe]"><GripVertical :size="11" />拖动排序</span>
            </div>
            <div class="grid min-h-0 grid-rows-[auto_auto_1fr_auto] gap-1.5 p-3">
              <div class="flex items-center justify-between gap-2"><strong class="flex min-w-0 items-center gap-1.5 truncate text-xs text-[#e5e7eb]"><span v-if="characterProfiles.find((item) => item.id === shot.characterId)" class="size-2 shrink-0 rounded-full" :style="{ backgroundColor: characterProfiles.find((item) => item.id === shot.characterId)?.color }" />{{ characterProfiles.find((item) => item.id === shot.characterId)?.name || '未指定角色' }}</strong><span class="font-mono text-[10px] text-[#67e8f9]">{{ shot.duration.toFixed(1) }}s</span></div>
              <p class="line-clamp-2 min-h-8 text-[10px] leading-4 text-[#94a3b8]">{{ shot.dialogue || '暂无台词' }}</p>
              <div class="flex flex-wrap content-start gap-1 text-[9px] font-bold">
                <span
                  class="flex items-center gap-1 rounded px-1.5 py-1"
                  :class="speechAsset(shot) ? 'bg-[#12312b] text-[#6ee7b7]' : 'bg-[#172033] text-[#93c5fd]'"
                ><Mic2 :size="10" />{{ speechStatusLabel(shot) }}</span>
                <span class="flex items-center gap-1 rounded bg-[#142b27] px-1.5 py-1 text-[#6ee7b7]"><CheckCircle2 :size="10" />{{ shot.cameraMotion.preset === 'static' ? '静态' : '已设运镜' }}</span>
                <span class="rounded bg-[#2a2035] px-1.5 py-1 text-[#d8b4fe]">{{ shot.transition || '无转场' }}</span>
              </div>
              <div class="flex justify-end gap-1 opacity-70 group-hover:opacity-100">
                <UiButton color="neutral" variant="ghost" square size="xs" type="button" aria-label="镜头上移" :disabled="shotIndex === 0" @click.stop="moveShot(shot.id, -1)"><ChevronUp :size="12" /></UiButton>
                <UiButton color="neutral" variant="ghost" square size="xs" type="button" aria-label="镜头下移" :disabled="shotIndex === shots.length - 1" @click.stop="moveShot(shot.id, 1)"><ChevronDown :size="12" /></UiButton>
                <UiButton color="neutral" variant="ghost" square size="xs" type="button" aria-label="复制镜头" @click.stop="duplicateShot(shot.id)"><Copy :size="12" /></UiButton>
                <UiButton color="error" variant="ghost" square size="xs" type="button" aria-label="删除镜头" @click.stop="deleteShots([shot.id])"><Trash2 :size="12" /></UiButton>
              </div>
            </div>
          </article>
        </div>
        <div v-else class="grid h-full place-items-center text-center text-sm text-[#64748b]">导入图片序列后即可开始镜头编排</div>
      </div>
    </div>

    <aside class="min-h-0 overflow-y-auto bg-[#10151e] p-4" aria-label="镜头详情">
      <template v-if="selectedShot">
        <div class="mb-4 flex items-start justify-between gap-2"><div><p class="text-[9px] font-black uppercase tracking-[0.16em] text-[#64748b]">Shot inspector</p><h3 class="text-base font-black text-white">镜头 {{ selectedShot.order + 1 }}</h3></div><UiButton color="error" variant="soft" size="xs" type="button" @click="deleteShots(selectedShotIds)"><Trash2 :size="12" />删除所选</UiButton></div>
        <div class="grid gap-4 text-[10px] font-bold text-[#94a3b8]">
          <label class="grid gap-1">角色<UiSelect aria-label="镜头角色" :model-value="selectedShot.characterId || ''" :items="characterOptions" :ui="workstationSelectUi" @update:model-value="selectShotCharacter(selectedShot, $event)" /></label>
          <label class="grid gap-1">台词<UiTextarea :model-value="selectedShot.dialogue" class="min-h-28" placeholder="输入该镜台词" @update:model-value="emit('updateShot', selectedShot.id, { dialogue: String($event) })" /></label>
          <label class="grid gap-1">情绪覆盖<UiSelect :model-value="selectedShot.emotion || ''" :items="emotionOverrideOptions" :disabled="!selectedCharacter" :ui="workstationSelectUi" @update:model-value="updateShotEmotion(selectedShot, $event)" /></label>
          <label class="grid gap-1">语速覆盖 {{ (selectedSpeechSettings?.speed ?? 1).toFixed(2) }}×<UiSlider :model-value="selectedSpeechSettings?.speed ?? 1" :min="0.75" :max="1.5" :step="0.05" :disabled="!selectedCharacter" @update:model-value="emit('updateShot', selectedShot.id, { speechSpeed: Number($event) })" /><UiButton v-if="selectedShot.speechSpeed !== undefined" color="neutral" variant="ghost" size="xs" type="button" class="justify-self-start" @click="emit('updateShot', selectedShot.id, { speechSpeed: undefined })">恢复角色默认</UiButton></label>
          <div class="grid gap-1.5">
            <UiButton color="secondary" variant="soft" size="sm" type="button" :disabled="speechActionDisabled" @click="runSpeechAction"><Mic2 :size="13" />{{ speechActionLabel }}</UiButton>
            <UiButton v-if="selectedSpeechAsset" color="neutral" variant="soft" size="sm" type="button" @click="emit('toggleSpeechPreview', selectedShot.id)">
              <Square v-if="previewingSpeechShotId === selectedShot.id" :size="12" />
              <Play v-else :size="13" />
              {{ previewingSpeechShotId === selectedShot.id ? '停止试听' : '播放试听' }}
            </UiButton>
            <p v-if="ttsStatus?.state === 'notInstalled'" class="text-[9px] leading-4 text-[#7890a8]">轻量 Kokoro 中文模型：安装峰值约 2.7 GB，完成后约占 1.5 GB。</p>
            <div v-if="ttsBusy || ttsStatus?.state === 'installing'" class="grid gap-1"><UiProgress :model-value="ttsProgress || 0" size="xs" color="secondary" aria-label="本地语音模型安装进度" /><div class="flex items-center justify-between gap-2 font-mono text-[9px] text-[#7890a8]"><span class="truncate">{{ ttsTaskStatus || '正在准备本地语音模型' }}</span><span>{{ Math.round(ttsProgress || 0) }}%</span></div></div>
            <p v-if="ttsError || (ttsStatus?.state === 'error' && ttsStatus.error)" class="text-[9px] leading-4 text-[#fca5a5]">{{ ttsError || ttsStatus?.error }}</p>
          </div>
          <label class="grid gap-1">时长（秒）<UiInputNumber :model-value="selectedShot.duration" :min="0.5" :max="60" :step="0.5" @change="emit('updateShot', selectedShot.id, { duration: clampDynamicComicShotDuration(Number(($event.target as HTMLInputElement).value)) })" /></label>
          <label class="grid gap-1">转场<UiInput :model-value="selectedShot.transition || ''" placeholder="例如：淡入淡出" @update:model-value="emit('updateShot', selectedShot.id, { transition: String($event) || undefined })" /></label>
        </div>
      </template>
      <div v-else class="grid min-h-32 place-items-center text-center text-xs leading-5 text-[#64748b]">选择一张镜头卡<br />在这里编辑详细参数</div>
      <div class="mt-6 border-t border-[#263142] pt-4">
        <div class="mb-3 flex items-center justify-between"><div><p class="text-[9px] font-black uppercase tracking-[0.16em] text-[#64748b]">Voice profiles</p><h4 class="text-sm font-black text-white">角色声线</h4></div><UiButton color="secondary" variant="soft" square size="xs" type="button" aria-label="新建角色声线" @click="addCharacter"><Plus :size="13" /></UiButton></div>
        <div class="grid gap-3">
          <article v-for="profile in characterProfiles" :key="profile.id" class="grid min-w-0 gap-3 rounded-xl border border-[#263142] bg-[#0b1018] p-3 text-[10px] font-bold text-[#94a3b8]">
            <div class="grid min-w-0 grid-cols-[36px_minmax(0,1fr)_28px] items-center gap-2">
              <UiInput type="color" :model-value="profile.color" class="size-9 shrink-0" :ui="profileColorInputUi" aria-label="角色显示色" @update:model-value="updateCharacter(profile, 'color', $event)" />
              <UiInput class="min-w-0" :model-value="profile.name" :ui="profileNameInputUi" aria-label="角色名称" @update:model-value="updateCharacter(profile, 'name', String($event))" />
              <UiButton color="error" variant="ghost" square size="xs" type="button" class="self-center" :aria-label="`删除角色 ${profile.name}`" @click="removeCharacter(profile)"><Trash2 :size="12" /></UiButton>
            </div>
            <div class="grid min-w-0 grid-cols-2 gap-2">
              <label class="grid min-w-0 gap-1.5">声音<UiSelect :model-value="profile.voice" :items="voiceOptions" size="sm" :ui="workstationSelectUi" @update:model-value="updateProfileVoice(profile, $event)" /></label>
              <label class="grid min-w-0 gap-1.5">默认情绪<UiSelect :model-value="profile.defaultEmotion" :items="defaultEmotionOptions" size="sm" :ui="workstationSelectUi" @update:model-value="updateProfileEmotion(profile, $event)" /></label>
            </div>
            <label class="grid gap-1.5 border-t border-[#1e2938] pt-2">默认语速 {{ profile.defaultSpeed.toFixed(2) }}×<UiSlider :model-value="profile.defaultSpeed" :min="0.75" :max="1.5" :step="0.05" @update:model-value="updateCharacter(profile, 'defaultSpeed', Number($event))" /></label>
          </article>
          <p v-if="!characterProfiles.length" class="rounded-lg border border-dashed border-[#334155] p-3 text-center text-[10px] text-[#64748b]">新建角色后，可让多个镜头共享同一套声线参数。</p>
        </div>
      </div>
    </aside>

    <div v-if="draggedShot" class="pointer-events-none fixed z-[90] grid h-[220px] w-[230px] -translate-x-1/2 -translate-y-1/2 grid-rows-[120px_minmax(0,1fr)] overflow-hidden rounded-xl border-2 border-[#2dd4bf] bg-[#111722]/95 shadow-[0_18px_48px_rgb(0_0_0/0.55)] backdrop-blur" :style="{ left: `${dragPointerX}px`, top: `${dragPointerY}px` }" aria-hidden="true">
      <div class="relative overflow-hidden bg-[#080b11]">
        <img v-if="thumbnail(draggedShot)" :src="thumbnail(draggedShot)" alt="" class="size-full object-cover" draggable="false" />
        <div v-else class="grid size-full place-items-center text-[#465469]"><WandSparkles :size="28" /></div>
      </div>
      <div class="p-3">
        <p class="truncate text-xs font-black text-white">{{ characterName(draggedShot) }}</p>
        <p class="mt-1 line-clamp-2 text-[10px] text-[#94a3b8]">{{ draggedShot.dialogue || '暂无台词' }}</p>
        <p class="mt-2 text-[9px] font-bold text-[#5eead4]">松开后移动到虚线位置</p>
      </div>
    </div>
    <UiModal :open="isCreateCharacterOpen" title="新建角色声线" class="w-[min(420px,calc(100vw-2rem))] rounded-xl border border-[#2b3545] bg-[#111722] p-5 shadow-2xl" :ui="{ overlay: 'z-[100] bg-black/65 backdrop-blur-sm', content: 'z-[101]' }" @update:open="!$event && closeCreateCharacter()">
      <div class="grid gap-4">
        <div><p class="text-[10px] font-black uppercase tracking-[0.16em] text-[#2dd4bf]">Voice profile</p><h3 class="mt-1 text-base font-black text-white">新建角色声线</h3><p class="mt-1 text-[11px] leading-5 text-[#94a3b8]">创建后会立即绑定到当前镜头，声线参数可继续在右侧档案区调整。</p></div>
        <label class="grid gap-1.5 text-[11px] font-bold text-[#aab5c5]">角色名称<UiInput v-model="characterNameDraft" aria-label="新角色名称" placeholder="例如：林夏、旁白" autofocus /></label>
        <div class="flex justify-end gap-2"><UiButton color="neutral" variant="ghost" size="sm" type="button" @click="closeCreateCharacter">取消</UiButton><UiButton color="secondary" variant="solid" size="sm" type="button" :disabled="!characterNameDraft.trim()" @click="confirmCreateCharacter">创建并绑定</UiButton></div>
      </div>
    </UiModal>
    <UiToast :open="feedbackToastOpen" :title="feedbackToastTitle" :description="feedbackToastDescription">
      <template #icon><CheckCircle2 :size="17" /></template>
    </UiToast>
  </section>
</template>
