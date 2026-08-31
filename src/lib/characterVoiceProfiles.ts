import type { CharacterVoiceProfile, DynamicComicProject, DynamicComicShot, TtsEmotion, TtsVoice } from "../types/editor";
import { DEFAULT_TTS_EMOTION, DEFAULT_TTS_VOICE, isTtsEmotion } from "./tts";

export function createCharacterVoiceProfile(index: number): CharacterVoiceProfile {
  return {
    id: `character-${globalThis.crypto?.randomUUID?.() ?? `${Date.now()}-${Math.random().toString(36).slice(2)}`}`,
    name: `角色 ${index + 1}`,
    color: "#2dd4bf",
    voice: DEFAULT_TTS_VOICE,
    defaultEmotion: DEFAULT_TTS_EMOTION,
    defaultSpeed: 1,
  };
}

export function resolveShotSpeechSettings(project: DynamicComicProject, shot: DynamicComicShot): {
  voice: TtsVoice;
  emotion: TtsEmotion;
  speed: number;
} | undefined {
  const profile = project.characterVoiceProfiles.find((item) => item.id === shot.characterId);
  if (!profile) return undefined;
  return {
    voice: profile.voice,
    emotion: isTtsEmotion(shot.emotion) ? shot.emotion : profile.defaultEmotion,
    speed: shot.speechSpeed ?? profile.defaultSpeed,
  };
}

export function deleteCharacterVoiceProfile(project: DynamicComicProject, profileId: string) {
  project.characterVoiceProfiles = project.characterVoiceProfiles.filter((profile) => profile.id !== profileId);
  for (const shot of project.shots) {
    if (shot.characterId === profileId) shot.characterId = undefined;
  }
}
