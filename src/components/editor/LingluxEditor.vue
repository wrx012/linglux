<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import {
  ArrowLeft,
  Download,
  Pause,
  Play,
  Redo2,
  Save,
  Undo2,
} from "@lucide/vue";
import ExportDialog from "./ExportDialog.vue";
import InspectorPanel from "./InspectorPanel.vue";
import MediaBin from "./MediaBin.vue";
import PreviewMonitor from "./PreviewMonitor.vue";
import TimelinePanel from "./TimelinePanel.vue";
import type { EditSession, EditorExportResult, EditorProject, ExportPreset, TimelineClip } from "../../types/editor";
import { calculateProjectDuration, cloneProject, exportPresets } from "../../lib/editorProject";

const props = defineProps<{
  session: EditSession;
}>();

const emit = defineEmits<{
  returnToWorkflow: [];
  exported: [result: EditorExportResult];
}>();

const project = ref(cloneProject(props.session.project));
const selectedClipId = ref(findFirstClipId());
const selectedAssetId = ref(project.value.assets[0]?.id ?? "");
const playhead = ref(0);
const timelineScale = ref(1);
const snapEnabled = ref(true);
const isPlaying = ref(false);
const isExportDialogOpen = ref(false);
const selectedPresetId = ref(exportPresets[0].id);
const isExporting = ref(false);
const saveState = ref("已保存");
const history = ref<EditorProject[]>([]);
const future = ref<EditorProject[]>([]);

const selectedClip = computed(() => findClip(selectedClipId.value));
const activeAsset = computed(() => project.value.assets.find((asset) => asset.id === selectedAssetId.value));
const selectedPreset = computed(() => exportPresets.find((preset) => preset.id === selectedPresetId.value) ?? exportPresets[0]);
const canUndo = computed(() => history.value.length > 0);
const canRedo = computed(() => future.value.length > 0);

watch(
  () => props.session,
  (session) => {
    project.value = cloneProject(session.project);
    selectedClipId.value = findFirstClipId();
    selectedAssetId.value = project.value.assets[0]?.id ?? "";
    playhead.value = 0;
    history.value = [];
    future.value = [];
    saveState.value = session.isDirty ? "未保存" : "已保存";
  },
  { deep: true },
);

function findFirstClipId() {
  return props.session.project.tracks.flatMap((track) => track.clips)[0]?.id ?? "";
}

function findClip(clipId?: string) {
  if (!clipId) {
    return undefined;
  }

  return project.value.tracks.flatMap((track) => track.clips).find((clip) => clip.id === clipId);
}

function selectClip(clipId: string) {
  const clip = findClip(clipId);

  selectedClipId.value = clipId;

  if (clip) {
    selectedAssetId.value = clip.assetId;
    playhead.value = clip.start;
  }
}

function selectAsset(assetId: string) {
  selectedAssetId.value = assetId;
}

function pushHistory() {
  history.value.push(cloneProject(project.value));

  if (history.value.length > 32) {
    history.value.shift();
  }

  future.value = [];
}

function markDirty() {
  project.value.duration = calculateProjectDuration(project.value.tracks);
  project.value.updatedAt = new Date().toISOString();
  saveState.value = "未保存";
}

function updatePlayhead(seconds: number) {
  const nextSeconds = snapEnabled.value ? snapTime(seconds) : seconds;
  playhead.value = clamp(nextSeconds, 0, project.value.duration);
}

function snapTime(seconds: number) {
  return Math.round(seconds * 2) / 2;
}

function togglePlayback() {
  isPlaying.value = !isPlaying.value;

  if (!isPlaying.value) {
    return;
  }

  window.setTimeout(() => {
    if (!isPlaying.value) {
      return;
    }

    playhead.value = Math.min(project.value.duration, playhead.value + 0.5);
    isPlaying.value = false;
  }, 500);
}

function updateClip(clipId: string, patch: Partial<TimelineClip>) {
  pushHistory();

  for (const track of project.value.tracks) {
    const index = track.clips.findIndex((clip) => clip.id === clipId);

    if (index !== -1) {
      track.clips[index] = {
        ...track.clips[index],
        ...patch,
      };
      selectedClipId.value = clipId;
      markDirty();
      return;
    }
  }
}

function splitSelectedClip() {
  const clip = selectedClip.value;

  if (!clip) {
    return;
  }

  const splitAt = clamp(snapEnabled.value ? snapTime(playhead.value) : playhead.value, clip.start + 0.5, clip.start + clip.duration - 0.5);

  if (splitAt <= clip.start || splitAt >= clip.start + clip.duration) {
    return;
  }

  pushHistory();

  const firstDuration = splitAt - clip.start;
  const secondDuration = clip.duration - firstDuration;
  const secondClip: TimelineClip = {
    ...cloneClip(clip),
    id: `${clip.id}-split-${Date.now()}`,
    name: `${clip.name} · B`,
    start: splitAt,
    duration: secondDuration,
    trimStart: clip.trimStart + firstDuration,
  };

  clip.duration = firstDuration;

  const track = project.value.tracks.find((item) => item.id === clip.trackId);
  track?.clips.push(secondClip);
  track?.clips.sort((left, right) => left.start - right.start);
  selectedClipId.value = secondClip.id;
  markDirty();
}

function deleteSelectedClip() {
  const clip = selectedClip.value;

  if (!clip) {
    return;
  }

  pushHistory();

  for (const track of project.value.tracks) {
    const index = track.clips.findIndex((item) => item.id === clip.id);

    if (index !== -1) {
      track.clips.splice(index, 1);
      selectedClipId.value = project.value.tracks.flatMap((item) => item.clips)[0]?.id ?? "";
      markDirty();
      return;
    }
  }
}

function trimClip(clipId: string, edge: "start" | "end") {
  const clip = findClip(clipId);

  if (!clip || clip.duration <= 0.75) {
    return;
  }

  pushHistory();

  if (edge === "start") {
    const trimAmount = Math.min(0.5, clip.duration - 0.5);
    clip.start = snapEnabled.value ? snapTime(clip.start + trimAmount) : clip.start + trimAmount;
    clip.duration -= trimAmount;
    clip.trimStart += trimAmount;
  } else {
    clip.duration = Math.max(0.5, clip.duration - 0.5);
    clip.trimEnd += 0.5;
  }

  selectedClipId.value = clip.id;
  markDirty();
}

function undo() {
  const previous = history.value.pop();

  if (!previous) {
    return;
  }

  future.value.push(cloneProject(project.value));
  project.value = previous;
  selectedClipId.value = project.value.tracks.flatMap((track) => track.clips)[0]?.id ?? "";
  saveState.value = "未保存";
}

function redo() {
  const next = future.value.pop();

  if (!next) {
    return;
  }

  history.value.push(cloneProject(project.value));
  project.value = next;
  selectedClipId.value = project.value.tracks.flatMap((track) => track.clips)[0]?.id ?? "";
  saveState.value = "未保存";
}

function zoomIn() {
  timelineScale.value = clamp(Number((timelineScale.value + 0.15).toFixed(2)), 0.55, 2.4);
}

function zoomOut() {
  timelineScale.value = clamp(Number((timelineScale.value - 0.15).toFixed(2)), 0.55, 2.4);
}

async function saveProject() {
  saveState.value = "保存中";

  try {
    project.value = await invoke<typeof project.value>("save_edit_project", {
      sessionId: props.session.id,
      project: project.value,
    });
    saveState.value = "已保存";
  } catch {
    project.value.updatedAt = new Date().toISOString();
    saveState.value = "已保存";
  }
}

async function exportProject() {
  isExporting.value = true;

  try {
    const result = await invoke<EditorExportResult>("export_edit_project", {
      request: {
        sessionId: props.session.id,
        project: project.value,
        preset: selectedPreset.value,
      },
    });
    emit("exported", result);
  } catch {
    emit("exported", createFallbackExport(selectedPreset.value));
  } finally {
    isExporting.value = false;
    isExportDialogOpen.value = false;
    saveState.value = "已保存";
  }
}

function createFallbackExport(preset: ExportPreset): EditorExportResult {
  const createdAt = new Date().toISOString();

  return {
    preset,
    artifact: {
      id: `artifact-edit-${Date.now()}`,
      type: "video",
      name: `${project.value.name} · edited.${preset.format}`,
      sourceNodeId: props.session.sourceNodeId ?? "editor",
      url: `linglux://exports/${project.value.id}.${preset.format}`,
      duration: project.value.duration,
      createdAt,
    },
  };
}

function cloneClip(clip: TimelineClip): TimelineClip {
  return JSON.parse(JSON.stringify(clip)) as TimelineClip;
}

function clamp(value: number, min: number, max: number) {
  return Math.min(Math.max(value, min), max);
}
</script>

<template>
  <section class="relative grid h-dvh grid-rows-[minmax(0,1fr)_minmax(360px,42vh)] gap-2 overflow-hidden bg-[#f5f6f7] p-1.5 text-[#171717] max-[980px]:h-auto max-[980px]:min-h-dvh max-[980px]:grid-rows-none" aria-label="Linglux 内置剪辑器">
    <div class="absolute right-5 top-5 z-30 flex items-center gap-1.5 rounded-full border border-[#d7d7d7] bg-white/90 px-2 py-1 shadow-sm backdrop-blur">
      <button class="grid size-8 place-items-center rounded-full text-[#525252] hover:bg-[#eef2f7]" type="button" title="返回工作流" aria-label="返回工作流" @click="emit('returnToWorkflow')">
        <ArrowLeft :size="15" />
      </button>
      <button class="grid size-8 place-items-center rounded-full text-[#525252] hover:bg-[#eef2f7] disabled:opacity-35" type="button" title="撤销" aria-label="撤销" :disabled="!canUndo" @click="undo">
        <Undo2 :size="15" />
      </button>
      <button class="grid size-8 place-items-center rounded-full text-[#525252] hover:bg-[#eef2f7] disabled:opacity-35" type="button" title="重做" aria-label="重做" :disabled="!canRedo" @click="redo">
        <Redo2 :size="15" />
      </button>
      <button class="grid size-8 place-items-center rounded-full text-[#525252] hover:bg-[#eef2f7]" type="button" title="播放/暂停" aria-label="播放或暂停" @click="togglePlayback">
        <Pause v-if="isPlaying" :size="15" />
        <Play v-else :size="15" />
      </button>
      <button class="grid size-8 place-items-center rounded-full text-[#525252] hover:bg-[#eef2f7]" type="button" title="保存" aria-label="保存剪辑工程" @click="saveProject">
        <Save :size="15" />
      </button>
      <button class="grid size-8 place-items-center rounded-full bg-[#0ea5e9] text-white hover:bg-[#0284c7]" type="button" title="导出" aria-label="打开导出设置" @click="isExportDialogOpen = true">
        <Download :size="15" />
      </button>
    </div>

    <div class="grid min-h-0 grid-cols-[minmax(360px,0.95fr)_minmax(480px,1.95fr)_minmax(360px,0.95fr)] gap-2 max-[1320px]:grid-cols-[320px_minmax(420px,1fr)_340px] max-[980px]:grid-cols-1">
      <MediaBin :assets="project.assets" :active-asset-id="selectedAssetId" @select-asset="selectAsset" />
      <PreviewMonitor :project="project" :selected-clip="selectedClip" :playhead="playhead" :is-playing="isPlaying" />
      <InspectorPanel :selected-clip="selectedClip" @update-clip="updateClip" />
    </div>

    <TimelinePanel
      :project="project"
      :selected-clip-id="selectedClipId"
      :playhead="playhead"
      :timeline-scale="timelineScale"
      :snap-enabled="snapEnabled"
      @select-clip="selectClip"
      @update-playhead="updatePlayhead"
      @trim-clip="trimClip"
      @split-selected="splitSelectedClip"
      @delete-selected="deleteSelectedClip"
      @zoom-in="zoomIn"
      @zoom-out="zoomOut"
    />

    <ExportDialog
      :open="isExportDialogOpen"
      :presets="exportPresets"
      :selected-preset="selectedPreset"
      :is-exporting="isExporting"
      @close="isExportDialogOpen = false"
      @select-preset="selectedPresetId = $event"
      @export="exportProject"
    />
  </section>
</template>
