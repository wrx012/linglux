export type EditorProjectSaveState = "未保存" | "保存中" | "已保存" | "保存失败";

interface EditorProjectPersistenceOptions<TSnapshot> {
  getSnapshot: () => TSnapshot;
  save: (snapshot: TSnapshot) => Promise<void>;
  debounceMs?: number;
  onStateChange?: (state: EditorProjectSaveState, error?: unknown) => void;
}

export interface EditorProjectPersistence {
  markDirty: () => void;
  scheduleSave: () => void;
  flush: () => Promise<boolean>;
  dispose: () => void;
  hasUnsavedChanges: () => boolean;
}

const DEFAULT_AUTOSAVE_DEBOUNCE_MS = 1_000;

export function createEditorProjectPersistence<TSnapshot>(
  options: EditorProjectPersistenceOptions<TSnapshot>,
): EditorProjectPersistence {
  const debounceMs = options.debounceMs ?? DEFAULT_AUTOSAVE_DEBOUNCE_MS;
  let revision = 0;
  let savedRevision = 0;
  let timer: ReturnType<typeof setTimeout> | undefined;
  let activeSave: Promise<boolean> | undefined;
  let disposed = false;

  function clearScheduledSave() {
    if (timer !== undefined) {
      clearTimeout(timer);
      timer = undefined;
    }
  }

  function scheduleSave() {
    if (disposed) return;

    clearScheduledSave();
    timer = setTimeout(() => {
      timer = undefined;
      void flush();
    }, debounceMs);
  }

  function markDirty() {
    if (disposed) return;

    revision += 1;
    options.onStateChange?.("未保存");
    scheduleSave();
  }

  async function saveCurrentRevision() {
    const savingRevision = revision;
    const snapshot = options.getSnapshot();
    options.onStateChange?.("保存中");

    try {
      await options.save(snapshot);
      savedRevision = Math.max(savedRevision, savingRevision);
      options.onStateChange?.(savedRevision === revision ? "已保存" : "未保存");
      return true;
    } catch (error) {
      options.onStateChange?.("保存失败", error);
      return false;
    }
  }

  async function flush() {
    if (disposed) return revision === savedRevision;

    clearScheduledSave();

    while (savedRevision < revision) {
      activeSave ??= saveCurrentRevision().finally(() => {
        activeSave = undefined;
      });

      const succeeded = await activeSave;
      if (!succeeded) return false;
    }

    return true;
  }

  function dispose() {
    disposed = true;
    clearScheduledSave();
  }

  return {
    markDirty,
    scheduleSave,
    flush,
    dispose,
    hasUnsavedChanges: () => savedRevision < revision,
  };
}
