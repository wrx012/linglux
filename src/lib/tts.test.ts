import { describe, expect, it } from "vitest";
import { createSpeechAssetName, ttsEmotionInstructions, validateTtsText } from "./tts";

describe("local TTS helpers", () => {
  it("validates empty and oversized scripts", () => {
    expect(validateTtsText(" ")).toContain("请输入");
    expect(validateTtsText("好".repeat(1001))).toContain("1000");
    expect(validateTtsText("今天天气很好")).toBe("");
  });

  it("creates a bounded unique asset name", () => {
    const text = "这是一段超过二十个字符的中文配音台词用于验证名称截断行为";
    const first = createSpeechAssetName(text, []);
    expect([...first.replace(/\.wav$/, "")]).toHaveLength(20);
    expect(createSpeechAssetName(text, [first])).toMatch(/ 2\.wav$/);
  });

  it("keeps every public emotion mapped", () => {
    expect(Object.keys(ttsEmotionInstructions)).toEqual(["natural", "gentle", "cheerful", "serious"]);
  });
});
