import { describe, expect, it } from "vitest";
import type { DynamicComicProject, DynamicComicShot } from "../types/editor";
import { createDefaultDynamicComicCameraMotion } from "./editorProject";
import { deleteCharacterVoiceProfile, resolveShotSpeechSettings } from "./characterVoiceProfiles";

function shot(overrides: Partial<DynamicComicShot> = {}): DynamicComicShot {
  return {
    id: "shot-1", order: 0, duration: 4, focus: { x: 0.5, y: 0.5 }, dialogue: "你好",
    pauseBefore: 0, pauseAfter: 0, cameraMotion: createDefaultDynamicComicCameraMotion(), soundEffectAssetIds: [],
    ...overrides,
  };
}

function project(shots: DynamicComicShot[]): DynamicComicProject {
  return {
    characterVoiceProfiles: [{ id: "hero", name: "主角", color: "#2dd4bf", voice: "zhMale", defaultEmotion: "serious", defaultSpeed: 0.9 }],
    shots,
  };
}

describe("character voice profiles", () => {
  it("inherits profile defaults while keeping per-shot overrides isolated", () => {
    const first = shot({ characterId: "hero", emotion: "cheerful", speechSpeed: 1.2 });
    const second = shot({ id: "shot-2", characterId: "hero" });
    const comic = project([first, second]);

    expect(resolveShotSpeechSettings(comic, first)).toEqual({ voice: "zhMale", emotion: "cheerful", speed: 1.2 });
    expect(resolveShotSpeechSettings(comic, second)).toEqual({ voice: "zhMale", emotion: "serious", speed: 0.9 });
  });

  it("deletes a referenced profile without losing dialogue or overrides", () => {
    const linked = shot({ characterId: "hero", emotion: "gentle", speechSpeed: 1.1 });
    const comic = project([linked]);

    deleteCharacterVoiceProfile(comic, "hero");

    expect(comic.characterVoiceProfiles).toEqual([]);
    expect(linked).toEqual(expect.objectContaining({ characterId: undefined, dialogue: "你好", emotion: "gentle", speechSpeed: 1.1 }));
  });
});
