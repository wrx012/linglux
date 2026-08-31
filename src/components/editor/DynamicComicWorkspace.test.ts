// @vitest-environment happy-dom

import { createApp, nextTick, reactive } from "vue";
import { afterEach, describe, expect, it, vi } from "vitest";
import { UiButton, UiInput, UiInputNumber, UiModal, UiProgress, UiSelect, UiSlider, UiTextarea, UiToast } from "../ui";
import { createDefaultDynamicComicCameraMotion } from "../../lib/editorProject";
import type { DynamicComicShot, EditorProject } from "../../types/editor";
import DynamicComicWorkspace from "./DynamicComicWorkspace.vue";

const mountedApps: Array<ReturnType<typeof createApp>> = [];

function shot(index: number): DynamicComicShot {
  return {
    id: `shot-${index}`,
    order: index - 1,
    duration: 4,
    focus: { x: 0.5, y: 0.5 },
    dialogue: "",
    pauseBefore: 0,
    pauseAfter: 0,
    cameraMotion: createDefaultDynamicComicCameraMotion(),
    soundEffectAssetIds: [],
  };
}

function project(): EditorProject {
  return {
    id: "project",
    name: "Dynamic comic",
    mode: "dynamicComic",
    dynamicComic: { characterVoiceProfiles: [], shots: [1, 2, 3, 4, 5].map(shot) },
    assets: [],
    tracks: [],
    mainTrackMagnetEnabled: true,
    duration: 20,
    fps: 24,
    resolution: { width: 1920, height: 1080 },
    createdAt: "2026-08-31T00:00:00.000Z",
    updatedAt: "2026-08-31T00:00:00.000Z",
  };
}

afterEach(() => {
  while (mountedApps.length) mountedApps.pop()?.unmount();
  document.body.replaceChildren();
  vi.restoreAllMocks();
});

describe("DynamicComicWorkspace", () => {
  it("shows a generated shot as voiced and exposes a replay control", async () => {
    const host = document.createElement("div");
    document.body.append(host);
    const comicProject = project();
    comicProject.assets.push({
      id: "speech-1",
      type: "audio",
      name: "小猫试听.wav",
      url: "asset://speech-1.wav",
      duration: 2,
      createdAt: "2026-08-31T00:00:00.000Z",
    });
    comicProject.dynamicComic!.shots[0].dialogue = "你好，我是小猫";
    comicProject.dynamicComic!.shots[0].speechAssetId = "speech-1";
    const played: string[] = [];
    const app = createApp(DynamicComicWorkspace, {
      project: comicProject,
      selectedShotIds: ["shot-1"],
      ttsStatus: { supported: true, state: "ready", runtimeInstalled: true, modelInstalled: true, requiredBytes: 1_000, voices: [] },
      onToggleSpeechPreview: (id: string) => played.push(id),
    });
    app.component("UiButton", UiButton);
    app.component("UiInput", UiInput);
    app.component("UiInputNumber", UiInputNumber);
    app.component("UiModal", UiModal);
    app.component("UiProgress", UiProgress);
    app.component("UiSelect", UiSelect);
    app.component("UiSlider", UiSlider);
    app.component("UiTextarea", UiTextarea);
    app.component("UiToast", UiToast);
    mountedApps.push(app);
    app.mount(host);

    expect(host.textContent).toContain("已配音");
    const replay = [...host.querySelectorAll<HTMLButtonElement>("button")].find((button) => button.textContent?.includes("播放试听"));
    expect(replay).toBeDefined();
    replay?.click();
    await nextTick();
    expect(played).toEqual(["shot-1"]);
  });

  it("treats whitespace-only dialogue as not needing speech", () => {
    const host = document.createElement("div");
    document.body.append(host);
    const comicProject = project();
    comicProject.dynamicComic!.shots[0].dialogue = "   ";
    const app = createApp(DynamicComicWorkspace, { project: comicProject, selectedShotIds: ["shot-1"] });
    app.component("UiButton", UiButton);
    app.component("UiInput", UiInput);
    app.component("UiInputNumber", UiInputNumber);
    app.component("UiModal", UiModal);
    app.component("UiProgress", UiProgress);
    app.component("UiSelect", UiSelect);
    app.component("UiSlider", UiSlider);
    app.component("UiTextarea", UiTextarea);
    app.component("UiToast", UiToast);
    mountedApps.push(app);
    app.mount(host);

    expect(host.textContent).toContain("无需配音");
  });

  it("keeps dangling and non-audio speech references in the pending state", () => {
    const host = document.createElement("div");
    document.body.append(host);
    const comicProject = project();
    comicProject.assets.push({
      id: "not-speech",
      type: "image",
      name: "not-speech.png",
      url: "asset://not-speech.png",
      duration: 4,
      createdAt: "2026-08-31T00:00:00.000Z",
    });
    comicProject.dynamicComic!.shots[0].dialogue = "需要生成配音";
    comicProject.dynamicComic!.shots[0].speechAssetId = "not-speech";
    const app = createApp(DynamicComicWorkspace, { project: comicProject, selectedShotIds: ["shot-1"] });
    app.component("UiButton", UiButton);
    app.component("UiInput", UiInput);
    app.component("UiInputNumber", UiInputNumber);
    app.component("UiModal", UiModal);
    app.component("UiProgress", UiProgress);
    app.component("UiSelect", UiSelect);
    app.component("UiSlider", UiSlider);
    app.component("UiTextarea", UiTextarea);
    app.component("UiToast", UiToast);
    mountedApps.push(app);
    app.mount(host);

    expect(host.querySelector('[data-shot-id="shot-1"]')?.textContent).toContain("待配音");
    expect(host.textContent).not.toContain("播放试听");
  });

  it("shows real TTS installation progress and task status", () => {
    const host = document.createElement("div");
    document.body.append(host);
    const app = createApp(DynamicComicWorkspace, {
      project: project(),
      selectedShotIds: ["shot-1"],
      ttsStatus: { supported: true, state: "installing", runtimeInstalled: false, modelInstalled: false, requiredBytes: 1_000, voices: [] },
      ttsBusy: true,
      ttsProgress: 37,
      ttsTaskStatus: "正在下载语音模型",
    });
    app.component("UiButton", UiButton);
    app.component("UiInput", UiInput);
    app.component("UiInputNumber", UiInputNumber);
    app.component("UiModal", UiModal);
    app.component("UiProgress", UiProgress);
    app.component("UiSelect", UiSelect);
    app.component("UiSlider", UiSlider);
    app.component("UiTextarea", UiTextarea);
    app.component("UiToast", UiToast);
    mountedApps.push(app);
    app.mount(host);

    expect(host.querySelector('[aria-label="本地语音模型安装进度"]')?.getAttribute("aria-valuenow")).toBe("37");
    expect(host.textContent).toContain("正在下载语音模型");
    expect(host.textContent).toContain("37%");
  });

  it("offers TTS setup from the shot inspector when the model is not installed", async () => {
    const host = document.createElement("div");
    document.body.append(host);
    const comicProject = project();
    comicProject.dynamicComic!.characterVoiceProfiles.push({ id: "cat", name: "小猫", color: "#2dd4bf", voice: "zhFemale", defaultEmotion: "natural", defaultSpeed: 1 });
    comicProject.dynamicComic!.shots[0].characterId = "cat";
    comicProject.dynamicComic!.shots[0].dialogue = "你好";
    let setupCount = 0;
    const app = createApp(DynamicComicWorkspace, {
      project: comicProject,
      selectedShotIds: ["shot-1"],
      ttsStatus: { supported: true, state: "notInstalled", runtimeInstalled: false, modelInstalled: false, requiredBytes: 1_000, voices: [] },
      onSetupTts: () => { setupCount += 1; },
    });
    app.component("UiButton", UiButton);
    app.component("UiInput", UiInput);
    app.component("UiInputNumber", UiInputNumber);
    app.component("UiModal", UiModal);
    app.component("UiProgress", UiProgress);
    app.component("UiSelect", UiSelect);
    app.component("UiSlider", UiSlider);
    app.component("UiTextarea", UiTextarea);
    app.component("UiToast", UiToast);
    mountedApps.push(app);
    app.mount(host);

    const setupButton = [...host.querySelectorAll<HTMLButtonElement>("button")].find((button) => button.textContent?.includes("安装本地语音模型"));
    expect(setupButton?.disabled).toBe(false);
    setupButton?.click();
    await nextTick();

    expect(setupCount).toBe(1);
  });

  it("creates and binds a voice profile from the shot character selector", async () => {
    const host = document.createElement("div");
    document.body.append(host);
    const comicProject = reactive(project());
    const created: Array<{ id: string }> = [];
    const updates: Array<[string, Partial<DynamicComicShot>]> = [];
    const app = createApp(DynamicComicWorkspace, {
      project: comicProject,
      selectedShotIds: ["shot-1"],
      onCreateCharacter: (profile: { id: string; name: string }) => {
        created.push(profile);
        comicProject.dynamicComic!.characterVoiceProfiles.push(profile as never);
      },
      onUpdateShot: (id: string, patch: Partial<DynamicComicShot>) => {
        updates.push([id, patch]);
        Object.assign(comicProject.dynamicComic!.shots.find((shot) => shot.id === id)!, patch);
      },
    });
    app.component("UiButton", UiButton);
    app.component("UiInput", UiInput);
    app.component("UiInputNumber", UiInputNumber);
    app.component("UiModal", UiModal);
    app.component("UiProgress", UiProgress);
    app.component("UiSelect", UiSelect);
    app.component("UiSlider", UiSlider);
    app.component("UiTextarea", UiTextarea);
    app.component("UiToast", UiToast);
    mountedApps.push(app);
    app.mount(host);

    const selector = host.querySelector<HTMLElement>('[aria-label="镜头角色"]')!;
    selector.dispatchEvent(new PointerEvent("pointerdown", { bubbles: true, button: 0, pointerId: 1 }));
    await nextTick();
    await new Promise((resolve) => window.setTimeout(resolve, 0));
    const createOption = [...document.body.querySelectorAll<HTMLElement>("[role=option]")].find((option) => option.textContent?.includes("新建角色声线"));
    expect(createOption).toBeDefined();
    createOption?.dispatchEvent(new PointerEvent("pointerup", { bubbles: true, button: 0, pointerId: 1 }));
    await nextTick();
    await new Promise((resolve) => window.setTimeout(resolve, 0));
    await nextTick();

    const nameInput = document.body.querySelector<HTMLInputElement>('[aria-label="新角色名称"]')!;
    nameInput.value = "林夏";
    nameInput.dispatchEvent(new Event("input", { bubbles: true }));
    await nextTick();
    const confirmButton = [...document.body.querySelectorAll<HTMLButtonElement>("button")].find((button) => button.textContent?.includes("创建并绑定"));
    expect(confirmButton?.disabled).toBe(false);
    confirmButton?.click();
    await nextTick();

    expect(created).toHaveLength(1);
    expect(created[0]).toEqual(expect.objectContaining({ name: "林夏" }));
    expect(updates).toEqual([["shot-1", { characterId: created[0].id, emotion: undefined, speechSpeed: undefined }]]);
    expect(selector.textContent).toContain("林夏");
    const colorInput = host.querySelector<HTMLInputElement>('[aria-label="角色显示色"]');
    expect(colorInput?.value).toBe("#2dd4bf");
    expect(colorInput?.className).toContain("role-color-swatch");
  });

  it("allows creating a voice profile before any shots exist", async () => {
    const host = document.createElement("div");
    document.body.append(host);
    const emptyProject = project();
    emptyProject.dynamicComic!.shots = [];
    const created: unknown[] = [];
    const app = createApp(DynamicComicWorkspace, {
      project: emptyProject,
      selectedShotIds: [],
      onCreateCharacter: (profile: unknown) => created.push(profile),
    });
    app.component("UiButton", UiButton);
    app.component("UiInput", UiInput);
    app.component("UiInputNumber", UiInputNumber);
    app.component("UiModal", UiModal);
    app.component("UiProgress", UiProgress);
    app.component("UiSelect", UiSelect);
    app.component("UiSlider", UiSlider);
    app.component("UiTextarea", UiTextarea);
    app.component("UiToast", UiToast);
    mountedApps.push(app);
    app.mount(host);

    host.querySelector<HTMLButtonElement>('[aria-label="新建角色声线"]')?.click();
    await nextTick();

    expect(created).toHaveLength(1);
  });

  it("swaps shot 1 and shot 5 during a cross-row pointer drag", async () => {
    const host = document.createElement("div");
    document.body.append(host);
    const reordered: string[][] = [];
    const deleted: string[][] = [];
    const duplicated: string[] = [];
    const app = createApp(DynamicComicWorkspace, {
      project: project(),
      selectedShotIds: [],
      onReorderSequence: (ids: string[]) => reordered.push(ids),
      onDelete: (ids: string[]) => deleted.push(ids),
      onDuplicate: (id: string) => duplicated.push(id),
    });
    app.component("UiButton", UiButton);
    app.component("UiInput", UiInput);
    app.component("UiInputNumber", UiInputNumber);
    app.component("UiModal", UiModal);
    app.component("UiProgress", UiProgress);
    app.component("UiSelect", UiSelect);
    app.component("UiSlider", UiSlider);
    app.component("UiTextarea", UiTextarea);
    app.component("UiToast", UiToast);
    mountedApps.push(app);
    app.mount(host);

    const first = host.querySelector<HTMLElement>('[data-shot-id="shot-1"]')!;
    const fifth = host.querySelector<HTMLElement>('[data-shot-id="shot-5"]')!;
    vi.spyOn(fifth, "getBoundingClientRect").mockReturnValue({ left: 0, top: 300, width: 260, height: 260, right: 260, bottom: 560, x: 0, y: 300, toJSON: () => ({}) });

    first.dispatchEvent(new PointerEvent("pointerdown", { bubbles: true, button: 0, pointerId: 1, isPrimary: true, clientX: 30, clientY: 30 }));
    window.dispatchEvent(new PointerEvent("pointermove", { bubbles: true, pointerId: 1, isPrimary: true, clientX: 30, clientY: 480 }));
    await nextTick();

    expect([...host.querySelectorAll<HTMLElement>("[data-shot-id]")].map((card) => card.dataset.shotId))
      .toEqual(["shot-5", "shot-2", "shot-3", "shot-4", "shot-1"]);
    expect(host.querySelector<HTMLElement>('[data-shot-id="shot-1"]')?.className).toContain("border-dashed");
    expect(host.querySelector<HTMLElement>('[data-shot-id="shot-5"]')?.className).toContain("ring-2");
    window.dispatchEvent(new PointerEvent("pointerup", { bubbles: true, pointerId: 1, isPrimary: true, clientX: 30, clientY: 480 }));
    expect(reordered).toEqual([["shot-5", "shot-2", "shot-3", "shot-4", "shot-1"]]);

    host.querySelector<HTMLButtonElement>('[data-shot-id="shot-5"] [aria-label="删除镜头"]')?.click();
    await nextTick();
    expect(deleted).toEqual([["shot-5"]]);
    expect(document.body.textContent).toContain("镜头已删除");
    expect(document.body.textContent).toContain("镜头及关联主轨片段已移除");

    host.querySelector<HTMLButtonElement>('[data-shot-id="shot-3"] [aria-label="复制镜头"]')?.click();
    await nextTick();
    expect(duplicated).toEqual(["shot-3"]);
    expect(document.body.textContent).toContain("镜头已复制");
    expect(document.body.textContent).toContain("镜头 3 的副本已插入其后");
  });
});
