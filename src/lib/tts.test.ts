import { describe, expect, it } from "vitest";
import { createSpeechAssetName, ttsEmotionInstructions, validateTtsText } from "./tts";

describe("local TTS helpers", () => {
  it("validates empty and oversized scripts", () => {
    expect(validateTtsText(" ")).toContain("请输入");
    expect(validateTtsText("好".repeat(1001))).toContain("1000");
    expect(validateTtsText("今天天气很好")).toBe("");
  });

  it("creates a unique asset name without copying private dialogue", () => {
    const privateDialogue = "这句私密台词不能进入素材名或 Agent 快照";
    const first = createSpeechAssetName([]);
    expect(first).toBe("AI 配音.wav");
    expect(first).not.toContain(privateDialogue);
    expect(createSpeechAssetName([first])).toBe("AI 配音 2.wav");
  });

  it("keeps every public emotion mapped", () => {
    expect(Object.keys(ttsEmotionInstructions)).toEqual(["natural", "gentle", "cheerful", "serious"]);
  });
});
