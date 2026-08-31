import json
import os
import sys
import wave

import numpy as np
import torch
from kokoro import KModel, KPipeline


def main(request):
    model_dir = request["modelDir"]
    model = KModel(
        repo_id="hexgrad/Kokoro-82M-v1.1-zh",
        config=os.path.join(model_dir, "config.json"),
        model=os.path.join(model_dir, "kokoro-v1_1-zh.pth"),
    ).to("cpu").eval()
    pipeline = KPipeline(
        lang_code="z",
        repo_id="hexgrad/Kokoro-82M-v1.1-zh",
        model=model,
        device="cpu",
    )
    voice_name = "zm_010" if request["voice"] == "zhMale" else "zf_001"
    voice = torch.load(
        os.path.join(model_dir, "voices", f"{voice_name}.pt"),
        map_location="cpu",
        weights_only=True,
    )
    emotion_speed = {
        "natural": 1.0,
        "gentle": 0.92,
        "cheerful": 1.06,
        "serious": 0.95,
    }[request["emotion"]]
    speed = max(0.75, min(1.5, float(request["speed"]) * emotion_speed))

    chunks = [
        result.audio.detach().cpu().numpy()
        for result in pipeline(request["text"], voice=voice, speed=speed)
        if result.audio is not None
    ]
    if not chunks:
        raise RuntimeError("语音模型没有生成音频")

    samples = np.clip(np.concatenate(chunks), -1.0, 1.0)
    with wave.open(request["outputPath"], "wb") as output:
        output.setnchannels(1)
        output.setsampwidth(2)
        output.setframerate(24000)
        output.writeframes((samples * 32767).astype(np.int16).tobytes())


if __name__ == "__main__":
    try:
        main(json.loads(sys.stdin.readline()))
        print(json.dumps({"ok": True}, ensure_ascii=False))
    except Exception as error:
        print(json.dumps({"ok": False, "error": str(error)}, ensure_ascii=False))
