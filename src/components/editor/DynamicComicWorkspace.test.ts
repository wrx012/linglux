// @vitest-environment happy-dom

import { createApp, nextTick } from "vue";
import { afterEach, describe, expect, it, vi } from "vitest";
import { UiButton, UiInput, UiInputNumber, UiTextarea, UiToast } from "../ui";
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
    dynamicComic: { shots: [1, 2, 3, 4, 5].map(shot) },
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
