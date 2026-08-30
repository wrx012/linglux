import type { DynamicComicShot, TimelineClip, TimelineTrack } from "../types/editor";

export const DYNAMIC_COMIC_MIN_SHOT_DURATION = 0.5;
export const DYNAMIC_COMIC_MAX_SHOT_DURATION = 60;

export function clampDynamicComicShotDuration(value: number) {
  if (!Number.isFinite(value)) return DYNAMIC_COMIC_MIN_SHOT_DURATION;
  return Math.min(DYNAMIC_COMIC_MAX_SHOT_DURATION, Math.max(DYNAMIC_COMIC_MIN_SHOT_DURATION, value));
}

export function swapDynamicComicIds(ids: string[], draggedId: string, targetId: string) {
  if (draggedId === targetId) return ids;
  const draggedIndex = ids.indexOf(draggedId);
  const targetIndex = ids.indexOf(targetId);
  if (draggedIndex < 0 || targetIndex < 0) return ids;
  const next = [...ids];
  next[draggedIndex] = targetId;
  next[targetIndex] = draggedId;
  return next;
}

export function reorderDynamicComicShots(shots: DynamicComicShot[], draggedId: string, targetId: string, after = false) {
  if (draggedId === targetId) return shots;
  const dragged = shots.find((shot) => shot.id === draggedId);
  const targetIndex = shots.findIndex((shot) => shot.id === targetId);
  if (!dragged || targetIndex < 0) return shots;
  const next = shots.filter((shot) => shot.id !== draggedId);
  const insertionIndex = next.findIndex((shot) => shot.id === targetId);
  next.splice(insertionIndex + (after ? 1 : 0), 0, dragged);
  return next.map((shot, order) => ({ ...shot, order }));
}

export function synchronizeDynamicComicTimeline(shots: DynamicComicShot[], primaryTrack: TimelineTrack | undefined) {
  if (!primaryTrack) return;
  const clipsById = new Map(primaryTrack.clips.map((clip) => [clip.id, clip] as const));
  let cursor = 0;
  for (const shot of shots) {
    shot.duration = clampDynamicComicShotDuration(shot.duration);
    const clip = shot.visualClipId ? clipsById.get(shot.visualClipId) : undefined;
    if (clip) {
      clip.start = cursor;
      clip.duration = shot.duration;
      clip.trimEnd = Math.max(clip.trimStart + shot.duration, clip.trimEnd);
      clip.transition = shot.transition;
      cursor += shot.duration;
    }
  }
  primaryTrack.clips.sort(compareTimelineClips);
}

export function duplicateDynamicComicShot(shot: DynamicComicShot, clip: TimelineClip | undefined) {
  const nonce = `${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
  const duplicatedClip = clip ? { ...cloneSerializableValue(clip), id: `clip-dynamic-comic-${nonce}` } : undefined;
  return {
    shot: {
      ...cloneSerializableValue(shot),
      id: `shot-${nonce}`,
      visualClipId: duplicatedClip?.id,
    },
    clip: duplicatedClip,
  };
}

function cloneSerializableValue<T>(value: T): T {
  return JSON.parse(JSON.stringify(value)) as T;
}

function compareTimelineClips(left: TimelineClip, right: TimelineClip) {
  return left.start - right.start || left.id.localeCompare(right.id);
}
