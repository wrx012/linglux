import type { TtsEmotion } from "../types/editor";

export const TTS_MAX_TEXT_LENGTH = 1000;

export const ttsEmotionInstructions: Record<TtsEmotion, string> = {
  natural: "请用自然、清晰的语气表达。",
  gentle: "请用温柔、舒缓的语气表达。",
  cheerful: "请用愉快、有活力的语气表达。",
  serious: "请用严肃、沉稳的语气表达。",
};

export function validateTtsText(text: string) {
  const trimmed = text.trim();
  if (!trimmed) return "请输入需要生成的台词。";
  if ([...trimmed].length > TTS_MAX_TEXT_LENGTH) return `台词不能超过 ${TTS_MAX_TEXT_LENGTH} 个字符。`;
  return "";
}

export function createSpeechAssetName(text: string, existingNames: Iterable<string>) {
  const base = [...text.trim().replace(/\s+/g, " ")].slice(0, 20).join("") || "AI 配音";
  const names = new Set(existingNames);
  let candidate = `${base}.wav`;
  let suffix = 2;
  while (names.has(candidate)) {
    candidate = `${base} ${suffix}.wav`;
    suffix += 1;
  }
  return candidate;
}
