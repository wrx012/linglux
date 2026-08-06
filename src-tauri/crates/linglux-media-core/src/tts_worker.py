import json
import os
import re
import sys
import wave


def split_text(text, limit=120):
    parts = [part.strip() for part in re.split(r"(?<=[。！？；!?;])", text) if part.strip()]
    result = []
    for part in parts:
        while len(part) > limit:
            result.append(part[:limit])
            part = part[limit:]
        if part:
            result.append(part)
    return result


def instruction(emotion, speed):
    emotions = {
        "natural": "请用自然、清晰的语气表达。",
        "gentle": "请用温柔、舒缓的语气表达。",
        "cheerful": "请用愉快、有活力的语气表达。",
        "serious": "请用严肃、沉稳的语气表达。",
    }
    speed_text = "正常语速"
    if speed < 0.9:
        speed_text = "较慢语速"
    elif speed > 1.1:
        speed_text = "较快语速"
    return f"{emotions[emotion]}请使用{speed_text}。<|endofprompt|>"


def merge_wavs(paths, output_path):
    params = None
    frames = []
    for path in paths:
        with wave.open(path, "rb") as source:
            current = source.getparams()
            if params is None:
                params = current
            elif current[:4] != params[:4]:
                raise RuntimeError("生成的音频分段格式不一致")
            frames.append(source.readframes(source.getnframes()))
    with wave.open(output_path, "wb") as target:
        target.setparams(params)
        for frame in frames:
            target.writeframes(frame)


def main(request):
    source_dir = request["sourceDir"]
    sys.path.insert(0, source_dir)
    sys.path.insert(0, os.path.join(source_dir, "third_party", "Matcha-TTS"))
    from cosyvoice.cli.cosyvoice import AutoModel
    import torchaudio

    model = AutoModel(model_dir=request["modelDir"])
    speaker = "中文男" if request["voice"] == "zhMale" else "中文女"
    chunks = split_text(request["text"])
    temp_paths = []
    try:
        for index, text in enumerate(chunks):
            output = next(model.inference_instruct(text, speaker, instruction(request["emotion"], request["speed"]), stream=False))
            path = f'{request["outputPath"]}.part-{index}.wav'
            torchaudio.save(path, output["tts_speech"], model.sample_rate)
            temp_paths.append(path)
        merge_wavs(temp_paths, request["outputPath"])
    finally:
        for path in temp_paths:
            try:
                os.remove(path)
            except OSError:
                pass


if __name__ == "__main__":
    try:
        payload = json.loads(sys.stdin.readline())
        main(payload)
        print(json.dumps({"ok": True}, ensure_ascii=False))
    except Exception as error:
        print(json.dumps({"ok": False, "error": str(error)}, ensure_ascii=False))
