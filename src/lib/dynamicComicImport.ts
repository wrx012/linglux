import type { DynamicComicShot, MediaAsset, TimelineClip } from "../types/editor";
import { createDefaultDynamicComicCameraMotion, createTimelineClip } from "./editorProject";

export type DynamicComicAspectRatio = "16:9" | "9:16" | "1:1";

export const DYNAMIC_COMIC_RESOLUTIONS: Record<DynamicComicAspectRatio, { width: number; height: number }> = {
  "16:9": { width: 1920, height: 1080 },
  "9:16": { width: 1080, height: 1920 },
  "1:1": { width: 1080, height: 1080 },
};

const naturalFilenameCollator = new Intl.Collator(undefined, {
  numeric: true,
  sensitivity: "base",
});

export function naturalSortImageAssets(assets: MediaAsset[]) {
  return assets
    .map((asset, selectionIndex) => ({ asset, selectionIndex }))
    .sort((left, right) => naturalFilenameCollator.compare(left.asset.name, right.asset.name) || left.selectionIndex - right.selectionIndex)
    .map(({ asset }) => asset);
}

export function reorderDynamicComicAssetIds(ids: string[], draggedId: string, targetId: string, after: boolean) {
  if (draggedId === targetId || !ids.includes(draggedId) || !ids.includes(targetId)) return ids;
  const next = ids.filter((id) => id !== draggedId);
  const targetIndex = next.indexOf(targetId);
  next.splice(targetIndex + (after ? 1 : 0), 0, draggedId);
  return next;
}

export function createDynamicComicShots(
  assets: MediaAsset[],
  trackId: string,
  duration: number,
  existingShotCount: number,
  start: number,
): { shots: DynamicComicShot[]; clips: TimelineClip[] } {
  let cursor = start;
  const clips: TimelineClip[] = [];
  const shots = assets.map((asset, index) => {
    const nonce = `${Date.now()}-${index}-${Math.random().toString(36).slice(2, 8)}`;
    const clip = createTimelineClip({
      id: `clip-dynamic-comic-${nonce}`,
      assetId: asset.id,
      trackId,
      name: asset.name,
      type: "video",
      start: cursor,
      duration,
    });
    cursor += duration;
    clips.push(clip);
    return {
      id: `shot-${nonce}`,
      order: existingShotCount + index,
      visualAssetId: asset.id,
      visualClipId: clip.id,
      duration,
      focus: { x: 0.5, y: 0.5 },
      dialogue: "",
      pauseBefore: 0,
      pauseAfter: 0,
      cameraMotion: createDefaultDynamicComicCameraMotion(),
      soundEffectAssetIds: [],
    } satisfies DynamicComicShot;
  });
  return { shots, clips };
}
