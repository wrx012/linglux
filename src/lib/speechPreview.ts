export interface SpeechPreviewCallbacks {
  ended?: () => void;
  error?: (error: Error) => void;
}

type AudioFactory = (url: string) => HTMLAudioElement;
const SILENT_WAV_DATA_URL = "data:audio/wav;base64,UklGRiYAAABXQVZFZm10IBAAAAABAAEAQB8AAIA+AAACABAAZGF0YQIAAAAAAA==";

export class SpeechPreviewController {
  private audio?: HTMLAudioElement;
  private detachListeners?: () => void;

  constructor(private readonly createAudio: AudioFactory = (url) => new Audio(url)) {}

  get isPlaying() {
    return this.audio !== undefined;
  }

  prime() {
    this.stop();
    const audio = this.createAudio(SILENT_WAV_DATA_URL);
    this.audio = audio;
    audio.preload = "auto";
    audio.volume = 0;
    void audio.play().catch(() => undefined);
  }

  async play(url: string, callbacks: SpeechPreviewCallbacks = {}) {
    const existingAudio = this.audio;
    const audio = existingAudio ?? this.createAudio(url);
    this.detachListeners?.();
    this.detachListeners = undefined;
    if (existingAudio) audio.pause();
    audio.src = url;
    this.audio = audio;
    audio.preload = "auto";
    audio.volume = 1;

    const finish = () => {
      if (this.audio !== audio) return;
      this.stop();
      callbacks.ended?.();
    };
    const fail = () => {
      if (this.audio !== audio) return;
      const error = new Error("试听音频加载失败。");
      this.stop();
      callbacks.error?.(error);
    };
    audio.addEventListener("ended", finish);
    audio.addEventListener("error", fail);
    this.detachListeners = () => {
      audio.removeEventListener("ended", finish);
      audio.removeEventListener("error", fail);
    };

    try {
      await audio.play();
    } catch (error) {
      if (this.audio === audio) this.stop();
      throw error;
    }
  }

  stop() {
    const audio = this.audio;
    this.audio = undefined;
    this.detachListeners?.();
    this.detachListeners = undefined;
    if (!audio) return;
    audio.pause();
    audio.removeAttribute("src");
    audio.load();
  }
}
