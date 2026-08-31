import { describe, expect, it, vi } from "vitest";
import { SpeechPreviewController } from "./speechPreview";

class FakeAudio extends EventTarget {
  preload = "";
  volume = 0;
  src: string;
  play = vi.fn(async () => undefined);
  pause = vi.fn();
  load = vi.fn();
  removeAttribute = vi.fn((name: string) => {
    if (name === "src") this.src = "";
  });

  constructor(src: string) {
    super();
    this.src = src;
  }
}

describe("SpeechPreviewController", () => {
  it("plays one generated speech file and cleans it up when playback ends", async () => {
    const audio = new FakeAudio("asset://speech.wav");
    const controller = new SpeechPreviewController(() => audio as unknown as HTMLAudioElement);
    const ended = vi.fn();

    await controller.play("asset://speech.wav", { ended });
    expect(audio.preload).toBe("auto");
    expect(audio.volume).toBe(1);
    expect(audio.play).toHaveBeenCalledOnce();
    expect(controller.isPlaying).toBe(true);

    audio.dispatchEvent(new Event("ended"));
    expect(ended).toHaveBeenCalledOnce();
    expect(audio.pause).toHaveBeenCalledOnce();
    expect(audio.removeAttribute).toHaveBeenCalledWith("src");
    expect(audio.load).toHaveBeenCalledOnce();
    expect(controller.isPlaying).toBe(false);
  });

  it("stops the previous speech before starting another one", async () => {
    const audio = new FakeAudio("first");
    const factory = vi.fn(() => audio as unknown as HTMLAudioElement);
    const controller = new SpeechPreviewController(factory);

    await controller.play("first");
    await controller.play("second");

    expect(factory).toHaveBeenCalledOnce();
    expect(audio.pause).toHaveBeenCalledOnce();
    expect(audio.play).toHaveBeenCalledTimes(2);
    expect(audio.src).toBe("second");
  });

  it("primes and reuses one audio element so delayed synthesis can still autoplay", async () => {
    const audio = new FakeAudio("silent");
    const factory = vi.fn(() => audio as unknown as HTMLAudioElement);
    const controller = new SpeechPreviewController(factory);

    controller.prime();
    await Promise.resolve();
    expect(audio.volume).toBe(0);
    expect(audio.play).toHaveBeenCalledOnce();

    await controller.play("asset://generated-speech.wav");
    expect(factory).toHaveBeenCalledOnce();
    expect(audio.src).toBe("asset://generated-speech.wav");
    expect(audio.volume).toBe(1);
    expect(audio.play).toHaveBeenCalledTimes(2);
  });

  it("cleans up and surfaces autoplay rejection", async () => {
    const audio = new FakeAudio("blocked");
    audio.play.mockRejectedValueOnce(new Error("autoplay blocked"));
    const controller = new SpeechPreviewController(() => audio as unknown as HTMLAudioElement);

    await expect(controller.play("blocked")).rejects.toThrow("autoplay blocked");
    expect(audio.pause).toHaveBeenCalledOnce();
    expect(controller.isPlaying).toBe(false);
  });
});
