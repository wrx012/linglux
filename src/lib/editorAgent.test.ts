import { describe, expect, it } from "vitest";
import { createSeededEditSession, createTimelineClip } from "./editorProject";
import {
  AgentPlanValidationError,
  createAgentProjectSnapshot,
  previewAgentEditPlan,
} from "./editorAgent";
import type { AgentEditPlan } from "../types/agent";
import type { EditorProject, MediaAsset } from "../types/editor";

function createProject(duration = 240): EditorProject {
  const project = createSeededEditSession().project;
  const asset: MediaAsset = {
    id: "asset-x",
    type: "video",
    name: "x.mp4",
    url: "linglux://test/x.mp4",
    duration,
    createdAt: "2026-01-01T00:00:00.000Z",
  };
  project.assets.push(asset);
  return project;
}

function plan(project: EditorProject, operation: AgentEditPlan["operations"][number]): AgentEditPlan {
  return {
    id: "plan-1",
    projectId: project.id,
    baseEditorVersion: 7,
    summary: "测试计划",
    operations: [operation],
    warnings: [],
  };
}

function addFullClip(project: EditorProject, speed = 1) {
  const track = project.tracks.find((item) => item.type === "video")!;
  const asset = project.assets[0];
  const clip = createTimelineClip({
    id: "clip-x",
    assetId: asset.id,
    trackId: track.id,
    name: asset.name,
    type: "video",
    start: 0,
    duration: asset.duration / speed,
  });
  clip.speed = speed;
  track.clips.push(clip);
  project.duration = clip.duration;
  return { track, clip };
}

describe("editor Agent plan executor", () => {
  it("adds an unplaced source range and keeps exactly 1:03–3:02", () => {
    const project = createProject();
    const videoTrack = project.tracks.find((track) => track.type === "video")!;
    const preview = previewAgentEditPlan(
      project,
      plan(project, {
        id: "add-range",
        type: "addAssetRange",
        assetId: "asset-x",
        trackId: videoTrack.id,
        sourceInMs: 63_000,
        sourceOutMs: 182_000,
      }),
      7,
    );
    const clip = preview.project.tracks.find((track) => track.id === videoTrack.id)!.clips[0];

    expect(clip.trimStart).toBe(63);
    expect(clip.trimEnd).toBe(58);
    expect(clip.duration).toBe(119);
    expect(preview.project.duration).toBe(119);
    expect(project.tracks.find((track) => track.id === videoTrack.id)!.clips).toHaveLength(0);
  });

  it("converts source duration through clip speed", () => {
    const project = createProject();
    const { clip } = addFullClip(project, 2);
    const preview = previewAgentEditPlan(
      project,
      plan(project, {
        id: "keep-fast",
        type: "keepClipSourceRange",
        clipId: clip.id,
        sourceInMs: 20_000,
        sourceOutMs: 80_000,
      }),
      7,
    );
    const result = preview.project.tracks.flatMap((track) => track.clips)[0];

    expect(result.trimStart).toBe(20);
    expect(result.trimEnd).toBe(160);
    expect(result.duration).toBe(30);
  });

  it("removes a middle source range and closes the magnetic gap", () => {
    const project = createProject(100);
    const { track, clip } = addFullClip(project);
    const preview = previewAgentEditPlan(
      project,
      plan(project, {
        id: "remove-middle",
        type: "removeClipSourceRange",
        clipId: clip.id,
        sourceInMs: 20_000,
        sourceOutMs: 40_000,
      }),
      7,
    );
    const clips = preview.project.tracks.find((item) => item.id === track.id)!.clips;

    expect(clips).toHaveLength(2);
    expect(clips[0]).toMatchObject({ start: 0, duration: 20, trimStart: 0, trimEnd: 80 });
    expect(clips[1]).toMatchObject({ start: 20, duration: 60, trimStart: 40, trimEnd: 0 });
    expect(preview.project.duration).toBe(80);
  });

  it("splits and moves clips across compatible visual tracks", () => {
    const project = createProject(100);
    const { track, clip } = addFullClip(project);
    const overlayTrack = project.tracks.find((item) => item.type === "overlay")!;
    const editPlan: AgentEditPlan = {
      id: "plan-split-move",
      projectId: project.id,
      baseEditorVersion: 7,
      summary: "分割并移动",
      warnings: [],
      operations: [
        {
          id: "split",
          type: "splitClipAtTimeline",
          clipId: clip.id,
          timelineTimeMs: 30_000,
        },
        {
          id: "move",
          type: "moveClip",
          clipId: clip.id,
          trackId: overlayTrack.id,
          timelineStartMs: 5_000,
        },
      ],
    };
    const preview = previewAgentEditPlan(project, editPlan, 7);
    const sourceClips = preview.project.tracks.find((item) => item.id === track.id)!.clips;
    const overlayClips = preview.project.tracks.find((item) => item.id === overlayTrack.id)!.clips;

    expect(sourceClips).toHaveLength(1);
    expect(sourceClips[0]).toMatchObject({ start: 0, duration: 70, trimStart: 30 });
    expect(overlayClips).toHaveLength(1);
    expect(overlayClips[0]).toMatchObject({ id: clip.id, start: 5, duration: 30 });
    expect(preview.affectedTrackIds).toEqual(expect.arrayContaining([track.id, overlayTrack.id]));
  });

  it("rejects edits on locked tracks", () => {
    const project = createProject();
    const { track, clip } = addFullClip(project);
    track.locked = true;

    expect(() =>
      previewAgentEditPlan(
        project,
        plan(project, {
          id: "delete",
          type: "deleteClip",
          clipId: clip.id,
        }),
        7,
      ),
    ).toThrow(/已锁定/);
  });

  it("rejects stale and out-of-range plans", () => {
    const project = createProject(120);
    const { clip } = addFullClip(project);
    const editPlan = plan(project, {
      id: "keep",
      type: "keepClipSourceRange",
      clipId: clip.id,
      sourceInMs: 63_000,
      sourceOutMs: 182_000,
    });

    expect(() => previewAgentEditPlan(project, editPlan, 8)).toThrow(/已经变化/);
    expect(() => previewAgentEditPlan(project, editPlan, 7)).toThrow(AgentPlanValidationError);
  });

  it("builds a sanitized snapshot without local media fields", () => {
    const project = createProject();
    project.assets[0].filePath = "/private/secret/x.mp4";
    project.assets[0].waveformPeaks = [0.2, 0.8];
    addFullClip(project);
    const snapshot = createAgentProjectSnapshot({
      project,
      editorVersion: 3,
      playhead: 4,
      selectedClipId: "clip-x",
      selectedAssetIds: [],
    });
    const serialized = JSON.stringify(snapshot);

    expect(serialized).not.toContain("/private/secret");
    expect(serialized).not.toContain("waveform");
    expect(snapshot.tracks[1].clips[0].sourceOutMs).toBe(240_000);
  });
});
