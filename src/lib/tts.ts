import type { TtsEmotion, TtsVoice } from "../types/editor";

export const TTS_MAX_TEXT_LENGTH = 1000;
export const DEFAULT_TTS_VOICE: TtsVoice = "zhFemale";
export const DEFAULT_TTS_EMOTION: TtsEmotion = "natural";

export const TTS_VOICE_OPTIONS: Array<{ value: TtsVoice; label: string }> = [
  { value: "zhFemale", label: "中文女声" },
  { value: "zhMale", label: "中文男声" },
];

export const TTS_EMOTION_OPTIONS: Array<{ value: TtsEmotion; label: string }> = [
  { value: "natural", label: "自然" },
  { value: "gentle", label: "温柔" },
  { value: "cheerful", label: "愉快" },
  { value: "serious", label: "严肃" },
];

export const ttsEmotionInstructions: Record<TtsEmotion, string> = {
  natural: "请用自然、清晰的语气表达。",
  gentle: "请用温柔、舒缓的语气表达。",
  cheerful: "请用愉快、有活力的语气表达。",
  serious: "请用严肃、沉稳的语气表达。",
};

export function isTtsVoice(value: unknown): value is TtsVoice {
  return TTS_VOICE_OPTIONS.some((option) => option.value === value);
}

export function isTtsEmotion(value: unknown): value is TtsEmotion {
  return TTS_EMOTION_OPTIONS.some((option) => option.value === value);
}

export function validateTtsText(text: string) {
  const trimmed = text.trim();
  if (!trimmed) return "请输入需要生成的台词。";
  if ([...trimmed].length > TTS_MAX_TEXT_LENGTH) return `台词不能超过 ${TTS_MAX_TEXT_LENGTH} 个字符。`;
  return "";
}

export function createSpeechAssetName(existingNames: Iterable<string>) {
  const base = "AI 配音";
  const names = new Set(existingNames);
  let candidate = `${base}.wav`;
  let suffix = 2;
  while (names.has(candidate)) {
    candidate = `${base} ${suffix}.wav`;
    suffix += 1;
  }
  return candidate;
}
