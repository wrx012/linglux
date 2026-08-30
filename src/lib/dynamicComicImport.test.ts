import { describe, expect, it } from "vitest";
import type { MediaAsset } from "../types/editor";
import { createDynamicComicShots, naturalSortImageAssets, reorderDynamicComicAssetIds } from "./dynamicComicImport";

function image(name: string, id = name): MediaAsset {
  return { id, name, type: "image", url: id, duration: 4, createdAt: "2026-08-31T00:00:00.000Z" };
}

describe("dynamic comic image sequence import", () => {
  it("sorts numbered filenames naturally and keeps ties in selection order", () => {
    expect(naturalSortImageAssets([image("10.png"), image("2.png"), image("01.png", "first"), image("01.png", "second")]).map((item) => item.id))
      .toEqual(["first", "second", "2.png", "10.png"]);
  });

  it("creates linked shots and gap-free primary-track clips", () => {
    const result = createDynamicComicShots([image("1.png"), image("2.png")], "track-video", 3.5, 2, 7);
    expect(result.clips.map((clip) => [clip.start, clip.duration])).toEqual([[7, 3.5], [10.5, 3.5]]);
    expect(result.shots.map((shot) => shot.order)).toEqual([2, 3]);
    expect(result.shots[0]?.visualClipId).toBe(result.clips[0]?.id);
    expect(result.shots[0]?.visualAssetId).toBe("1.png");
  });

  it("moves an image before or after the pointed card", () => {
    expect(reorderDynamicComicAssetIds(["1", "2", "10"], "1", "10", false)).toEqual(["2", "1", "10"]);
    expect(reorderDynamicComicAssetIds(["1", "2", "10"], "1", "10", true)).toEqual(["2", "10", "1"]);
  });
});
