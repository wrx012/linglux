import { describe, expect, it } from "vitest";
import type { EditorProject, MediaAsset, TimelineClip, TimelineTrack, TimelineTrackType } from "../types/editor";
import { collectTimelineBeatMarkerTimes, planVideoBeatMarkerAlignment } from "./beatMarkerAlignment";

function asset(id: string, type: MediaAsset["type"], duration = 60): MediaAsset {
  return {
    id,
    type,
    name: `${id}.mov`,
    url: `linglux://test/${id}`,
    duration,
    createdAt: "2026-01-01T00:00:00.000Z",
  };
}

function clip(
  id: string,
  assetId: string,
  trackId: string,
  type: TimelineTrackType,
  start: number,
  duration: number,
): TimelineClip {
  return {
    id,
    assetId,
    trackId,
    name: id,
    type,
    start,
    duration,
    trimStart: 0,
    trimEnd: 60 - duration,
    volume: 1,
    muted: false,
    visible: true,
    speed: 1,
    transform: { x: 0, y: 0, scale: 1, rotation: 0, opacity: 1 },
    effects: [],
  };
}

function track(id: string, type: TimelineTrackType, clips: TimelineClip[]): TimelineTrack {
  return { id, type, clips, label: id, muted: false, visible: true, mediaEnabled: true, locked: false };
}

function project(videoClips: TimelineClip[], audioClips: TimelineClip[], assets: MediaAsset[]): EditorProject {
  return {
    id: "project",
    name: "Alignment",
    mode: "timeline",
    assets,
    tracks: [track("video", "video", videoClips), track("audio", "audio", audioClips)],
    mainTrackMagnetEnabled: true,
    duration: 60,
    fps: 30,
    resolution: { width: 1920, height: 1080 },
    createdAt: "2026-01-01T00:00:00.000Z",
    updatedAt: "2026-01-01T00:00:00.000Z",
  };
}

function beatClip(id: string, start: number, duration: number, markers: number[]) {
  const result = clip(id, "audio-asset", "audio", "audio", start, duration);
  result.beatMode = "auto";
  result.beatMarkers = markers.map((time) => ({ time, intensity: 0.8 }));
  return result;
}

describe("video beat-marker alignment", () => {
  it("converts clip-relative markers to sorted, deduplicated timeline times", () => {
    const selected = clip("selected", "video-asset", "video", "video", 2, 8);
    const firstAudio = beatClip("beat-a", 4, 10, [1, 3]);
    const secondAudio = beatClip("beat-b", 0, 10, [5, 9]);
    const value = project(
      [selected],
      [firstAudio, secondAudio],
      [asset("video-asset", "video"), asset("audio-asset", "audio")],
    );

    expect(collectTimelineBeatMarkerTimes(value)).toEqual([5, 7, 9]);
  });

  it("trims leading audio silence so the first beat and first primary video both begin at zero", () => {
    const selected = clip("selected", "video-asset", "video", "video", 2, 8);
    selected.trimStart = 2;
    selected.trimEnd = 50;
    const value = project(
      [selected],
      [beatClip("beats", 0, 20, [1.5, 2.25, 9.7, 10.4])],
      [asset("video-asset", "video"), asset("audio-asset", "audio")],
    );

    const patches = planVideoBeatMarkerAlignment(value, selected.id)?.patches ?? [];

    expect(patches).toHaveLength(2);
    expect(patches).toEqual(expect.arrayContaining([
      expect.objectContaining({ clipId: "selected", start: 0, duration: 8.9, trimStart: 0, trimEnd: 51.1 }),
      expect.objectContaining({
        clipId: "beats",
        start: 0,
        duration: 18.5,
        trimStart: 1.5,
        beatMarkers: [
          { time: 0, intensity: 0.8 },
          { time: 0.75, intensity: 0.8 },
          { time: 8.2, intensity: 0.8 },
          { time: 8.9, intensity: 0.8 },
        ],
      }),
    ]));

    for (const patch of patches) {
      const target = value.tracks.flatMap((item) => item.clips).find((item) => item.id === patch.clipId)!;
      Object.assign(target, patch);
    }

    expect(planVideoBeatMarkerAlignment(value, selected.id)).toBeUndefined();
  });

  it("moves a shared cut on both adjacent video clips", () => {
    const previous = clip("previous", "previous-asset", "video", "video", 0, 5);
    previous.trimEnd = 55;
    const selected = clip("selected", "video-asset", "video", "video", 5, 5);
    selected.trimStart = 5;
    selected.trimEnd = 50;
    const value = project(
      [previous, selected],
      [beatClip("beats", 0, 12, [5.2, 9.8])],
      [asset("previous-asset", "video"), asset("video-asset", "video"), asset("audio-asset", "audio")],
    );
    const patches = planVideoBeatMarkerAlignment(value, selected.id)?.patches ?? [];

    expect(patches).toEqual([
      expect.objectContaining({ clipId: "previous", start: 0, duration: 5.2, trimEnd: 54.8 }),
      expect.objectContaining({ clipId: "selected", start: 5.2, duration: 4.6, trimStart: 5.2, trimEnd: 50.2 }),
    ]);
  });

  it("uses source-time speed and skips a closer marker that exceeds source handles", () => {
    const earlier = clip("earlier", "earlier-asset", "video", "video", 0, 1);
    const selected = clip("selected", "video-asset", "video", "video", 5, 5);
    selected.speed = 2;
    selected.trimStart = 0.2;
    selected.trimEnd = 49.8;
    const value = project(
      [earlier, selected],
      [beatClip("beats", 0, 20, [4.5, 5.1, 9.8, 10.5])],
      [asset("earlier-asset", "video"), asset("video-asset", "video"), asset("audio-asset", "audio")],
    );
    const patch = planVideoBeatMarkerAlignment(value, selected.id)?.patches[0];

    expect(patch).toMatchObject({ start: 5.1, duration: 4.7, trimStart: 0.4, trimEnd: 50.2 });
  });

  it("keeps at least the minimum duration and prefers the earlier marker on a tie", () => {
    const earlier = clip("earlier", "earlier-asset", "video", "video", 0, 1);
    const selected = clip("selected", "video-asset", "video", "video", 5, 1);
    selected.trimStart = 5;
    selected.trimEnd = 54;
    const value = project(
      [earlier, selected],
      [beatClip("beats", 0, 10, [4.8, 5.2, 5.8, 6.2])],
      [asset("earlier-asset", "video"), asset("video-asset", "video"), asset("audio-asset", "audio")],
    );
    const patch = planVideoBeatMarkerAlignment(value, selected.id)?.patches[0];

    expect(patch).toMatchObject({ start: 4.8, duration: 1 });
  });

  it("returns no plan without a selected video or enabled markers", () => {
    const selected = clip("selected", "image-asset", "video", "video", 0, 4);
    const audio = beatClip("beats", 0, 10, [2, 4]);
    audio.beatMode = undefined;
    const value = project(
      [selected],
      [audio],
      [asset("image-asset", "image"), asset("audio-asset", "audio")],
    );

    expect(planVideoBeatMarkerAlignment(value, selected.id)).toBeUndefined();
    expect(planVideoBeatMarkerAlignment(value, "missing")).toBeUndefined();
  });
});
