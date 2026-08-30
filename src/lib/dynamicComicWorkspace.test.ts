import { describe, expect, it } from "vitest";
import { reactive } from "vue";
import type { DynamicComicShot, TimelineTrack } from "../types/editor";
import { createDefaultDynamicComicCameraMotion, createTimelineClip } from "./editorProject";
import { duplicateDynamicComicShot, reorderDynamicComicShots, swapDynamicComicIds, synchronizeDynamicComicTimeline } from "./dynamicComicWorkspace";

function shot(id: string, clipId: string, duration: number, order: number): DynamicComicShot {
  return { id, order, visualClipId: clipId, duration, focus: { x: 0.5, y: 0.5 }, dialogue: "", pauseBefore: 0, pauseAfter: 0, cameraMotion: createDefaultDynamicComicCameraMotion(), soundEffectAssetIds: [] };
}

function track(): TimelineTrack {
  return { id: "video", type: "video", label: "主轨", muted: false, visible: true, mediaEnabled: true, locked: false, clips: [
    createTimelineClip({ id: "c1", assetId: "a1", trackId: "video", name: "1", type: "video", start: 0, duration: 2 }),
    createTimelineClip({ id: "c2", assetId: "a2", trackId: "video", name: "2", type: "video", start: 2, duration: 3 }),
  ] };
}

describe("dynamic comic workspace", () => {
  it("swaps the dragged and target cards without shifting cards in between", () => {
    expect(swapDynamicComicIds(["1", "2", "3", "4", "5"], "1", "5")).toEqual(["5", "2", "3", "4", "1"]);
    expect(swapDynamicComicIds(["1", "2", "3", "4", "5"], "5", "1")).toEqual(["5", "2", "3", "4", "1"]);
  });

  it("duplicates Vue-reactive shot and clip values", () => {
    const sourceShot = reactive(shot("s1", "c1", 4, 0));
    const sourceClip = reactive(track().clips[0]!);
    const duplicated = duplicateDynamicComicShot(sourceShot, sourceClip);
    expect(duplicated.shot.id).not.toBe(sourceShot.id);
    expect(duplicated.clip?.id).not.toBe(sourceClip.id);
    expect(duplicated.shot.visualClipId).toBe(duplicated.clip?.id);
  });

  it("reorders shots and normalizes their visible order", () => {
    expect(reorderDynamicComicShots([shot("s1", "c1", 2, 0), shot("s2", "c2", 3, 1)], "s2", "s1").map((item) => [item.id, item.order])).toEqual([["s2", 0], ["s1", 1]]);
  });

  it("keeps linked clips continuous after order and duration changes", () => {
    const video = track();
    const shots = [shot("s2", "c2", 4, 0), shot("s1", "c1", 1.5, 1)];
    synchronizeDynamicComicTimeline(shots, video);
    expect(video.clips.map((clip) => [clip.id, clip.start, clip.duration])).toEqual([["c2", 0, 4], ["c1", 4, 1.5]]);
  });

  it("supports downward drops and ignores unlinked shots when closing primary-track gaps", () => {
    const shots = [shot("s1", "c1", 2, 0), shot("s2", "missing", 8, 1), shot("s3", "c2", 3, 2)];
    expect(reorderDynamicComicShots(shots, "s1", "s2", true).map((item) => item.id)).toEqual(["s2", "s1", "s3"]);
    const video = track();
    synchronizeDynamicComicTimeline(shots, video);
    expect(video.clips.map((clip) => [clip.id, clip.start])).toEqual([["c1", 0], ["c2", 2]]);
  });
});
