import { afterEach, describe, expect, it, vi } from "vitest";
import { createEditorProjectPersistence } from "./editorProjectPersistence";

afterEach(() => {
  vi.useRealTimers();
});

describe("editor project persistence", () => {
  it("debounces automatic saves and persists only the latest snapshot", async () => {
    vi.useFakeTimers();
    let value = "initial";
    const saved: string[] = [];
    const persistence = createEditorProjectPersistence({
      debounceMs: 1_000,
      getSnapshot: () => value,
      save: async (snapshot) => { saved.push(snapshot); },
    });

    value = "first";
    persistence.markDirty();
    await vi.advanceTimersByTimeAsync(700);
    value = "latest";
    persistence.markDirty();
    await vi.advanceTimersByTimeAsync(999);

    expect(saved).toEqual([]);
    await vi.advanceTimersByTimeAsync(1);
    expect(saved).toEqual(["latest"]);
    expect(persistence.hasUnsavedChanges()).toBe(false);
  });

  it("serializes saves without letting an older snapshot cover newer edits", async () => {
    let value = "first";
    let releaseFirstSave!: () => void;
    const saved: string[] = [];
    const firstSave = new Promise<void>((resolve) => { releaseFirstSave = resolve; });
    const persistence = createEditorProjectPersistence({
      getSnapshot: () => value,
      save: async (snapshot) => {
        saved.push(snapshot);
        if (snapshot === "first") await firstSave;
      },
    });

    persistence.markDirty();
    const flushing = persistence.flush();
    await Promise.resolve();
    value = "second";
    persistence.markDirty();
    releaseFirstSave();

    await expect(flushing).resolves.toBe(true);
    expect(saved).toEqual(["first", "second"]);
    expect(persistence.hasUnsavedChanges()).toBe(false);
  });

  it("flushes immediately before navigation", async () => {
    vi.useFakeTimers();
    let value = "小猫";
    const saved: string[] = [];
    const persistence = createEditorProjectPersistence({
      debounceMs: 1_000,
      getSnapshot: () => value,
      save: async (snapshot) => { saved.push(snapshot); },
    });

    persistence.markDirty();

    await expect(persistence.flush()).resolves.toBe(true);
    expect(saved).toEqual(["小猫"]);
  });

  it("keeps changes dirty when saving fails and retries on the next flush", async () => {
    let attempt = 0;
    const states: string[] = [];
    const persistence = createEditorProjectPersistence({
      getSnapshot: () => "project",
      save: async () => {
        attempt += 1;
        if (attempt === 1) throw new Error("disk full");
      },
      onStateChange: (state) => states.push(state),
    });

    persistence.markDirty();

    await expect(persistence.flush()).resolves.toBe(false);
    expect(persistence.hasUnsavedChanges()).toBe(true);
    await expect(persistence.flush()).resolves.toBe(true);
    expect(states).toContain("保存失败");
    expect(states[states.length - 1]).toBe("已保存");
  });

  it("dispose cancels a pending automatic save", async () => {
    vi.useFakeTimers();
    const save = vi.fn(async () => undefined);
    const persistence = createEditorProjectPersistence({
      debounceMs: 1_000,
      getSnapshot: () => "project",
      save,
    });

    persistence.markDirty();
    persistence.dispose();
    await vi.runAllTimersAsync();

    expect(save).not.toHaveBeenCalled();
  });
});
