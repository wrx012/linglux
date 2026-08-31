import { describe, expect, it } from "vitest";
import type { EditorProject } from "../types/editor";
import { cloneProject, createSeededEditSession, normalizeEditorProject } from "./editorProject";

describe("editor project normalization", () => {
  it("loads a legacy project as a normal timeline project", () => {
    const { mode: _mode, ...legacyProject } = createSeededEditSession().project;

    const normalized = normalizeEditorProject(legacyProject as EditorProject);

    expect(normalized.mode).toBe("timeline");
    expect(normalized.dynamicComic).toBeUndefined();
  });

  it("creates an explicitly marked dynamic-comic project", () => {
    const project = createSeededEditSession({ mode: "dynamicComic" }).project;

    expect(project.mode).toBe("dynamicComic");
    expect(project.dynamicComic).toEqual({ characterVoiceProfiles: [], shots: [] });
  });

  it("normalizes and clones durable dynamic-comic shot metadata", () => {
    const project = createSeededEditSession({ mode: "dynamicComic" }).project;
    project.dynamicComic!.characterVoiceProfiles = [{ id: "character-lin-xia", name: "林夏", color: "#2dd4bf", voice: "zhFemale", defaultEmotion: "natural", defaultSpeed: 1 }];
    project.dynamicComic!.shots = [
      {
        id: "shot-hero-arrives",
        order: 4,
        visualAssetId: "asset-panel-04",
        visualClipId: "clip-panel-04",
        duration: 4.2,
        focus: { x: 0.62, y: 0.31 },
        characterId: "character-lin-xia",
        speechAssetId: "asset-speech-shot-04",
        dialogue: "你为什么现在才回来？",
        emotion: "serious",
        speechSpeed: 1.2,
        pauseBefore: 0.25,
        pauseAfter: 0.4,
        cameraMotion: {
          preset: "pushIn",
          start: { x: 0.5, y: 0.5, scale: 1 },
          end: { x: 0.62, y: 0.31, scale: 1.25 },
          easing: "easeInOut",
        },
        transition: "hardCut",
        soundEffectAssetIds: ["asset-rain", "asset-door"],
      },
    ];

    const cloned = cloneProject(project);

    expect(cloned).not.toBe(project);
    expect(cloned.dynamicComic?.shots).toEqual([
      expect.objectContaining({
        id: "shot-hero-arrives",
        order: 4,
        visualAssetId: "asset-panel-04",
        visualClipId: "clip-panel-04",
        duration: 4.2,
        focus: { x: 0.62, y: 0.31 },
        characterId: "character-lin-xia",
        speechAssetId: "asset-speech-shot-04",
        dialogue: "你为什么现在才回来？",
        emotion: "serious",
        speechSpeed: 1.2,
        pauseBefore: 0.25,
        pauseAfter: 0.4,
        transition: "hardCut",
        soundEffectAssetIds: ["asset-rain", "asset-door"],
      }),
    ]);
    expect(cloned.dynamicComic?.shots[0]?.cameraMotion).toEqual({
      preset: "pushIn",
      start: { x: 0.5, y: 0.5, scale: 1 },
      end: { x: 0.62, y: 0.31, scale: 1.25 },
      easing: "easeInOut",
    });
  });

  it("fills safe defaults for incomplete dynamic-comic shot data", () => {
    const project = createSeededEditSession({ mode: "dynamicComic" }).project;
    project.dynamicComic = {
      characterVoiceProfiles: [],
      shots: [{ id: "shot-1" } as never],
    };

    const shot = normalizeEditorProject(project).dynamicComic?.shots[0];

    expect(shot).toEqual({
      id: "shot-1",
      order: 0,
      visualAssetId: undefined,
      visualClipId: undefined,
      duration: 0,
      focus: { x: 0.5, y: 0.5 },
      characterId: undefined,
      speechAssetId: undefined,
      dialogue: "",
      emotion: undefined,
      speechSpeed: undefined,
      pauseBefore: 0,
      pauseAfter: 0,
      cameraMotion: {
        preset: "static",
        start: { x: 0.5, y: 0.5, scale: 1 },
        end: { x: 0.5, y: 0.5, scale: 1 },
        easing: "easeInOut",
      },
      transition: undefined,
      soundEffectAssetIds: [],
    });
  });

  it("replaces missing and duplicate shot IDs without rewriting valid order values", () => {
    const project = createSeededEditSession({ mode: "dynamicComic" }).project;
    project.dynamicComic = {
      characterVoiceProfiles: [],
      shots: [
        { id: "shot-existing", order: 8 } as never,
        { id: "shot-existing", order: 3.8 } as never,
        { order: 5 } as never,
      ],
    };

    const shots = normalizeEditorProject(project).dynamicComic?.shots ?? [];

    expect(shots.map((shot) => shot.order)).toEqual([3, 5, 8]);
    expect(new Set(shots.map((shot) => shot.id)).size).toBe(3);
    expect(shots.find((shot) => shot.order === 8)?.id).toBe("shot-existing");
    expect(shots.filter((shot) => shot.id.startsWith("shot-")).length).toBe(3);
  });

  it("migrates legacy free-text character bindings into reusable voice profiles", () => {
    const project = createSeededEditSession({ mode: "dynamicComic" }).project;
    project.dynamicComic = {
      characterVoiceProfiles: [],
      shots: [{ id: "shot-1", characterId: "legacy-free-text-name" } as never],
    };

    const dynamicComic = normalizeEditorProject(project).dynamicComic;

    expect(dynamicComic?.shots[0]?.characterId).toBe("legacy-free-text-name");
    expect(dynamicComic?.characterVoiceProfiles).toEqual([{
      id: "legacy-free-text-name",
      name: "legacy-free-text-name",
      color: "#2dd4bf",
      voice: "zhFemale",
      defaultEmotion: "natural",
      defaultSpeed: 1,
    }]);
  });

  it("normalizes generated speech asset references as optional non-empty IDs", () => {
    const project = createSeededEditSession({ mode: "dynamicComic" }).project;
    project.dynamicComic = {
      characterVoiceProfiles: [],
      shots: [
        { id: "shot-with-speech", order: 0, speechAssetId: "  asset-speech-1  " } as never,
        { id: "shot-without-speech", order: 1, speechAssetId: "   " } as never,
      ],
    };

    const shots = normalizeEditorProject(project).dynamicComic?.shots ?? [];

    expect(shots[0]?.speechAssetId).toBe("asset-speech-1");
    expect(shots[1]?.speechAssetId).toBeUndefined();
  });
});
