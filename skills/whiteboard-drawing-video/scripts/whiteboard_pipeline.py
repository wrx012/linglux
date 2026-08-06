# /// script
# requires-python = ">=3.11"
# dependencies = [
#   "numpy==2.3.2",
#   "opencv-python-headless==4.11.0.86",
#   "pillow==10.4.0",
# ]
# ///
"""Deterministic .srt -> permit masks -> HyperFrames whiteboard pipeline."""

from __future__ import annotations

import argparse
import json
import math
import re
import shutil
import subprocess
import sys
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Any, Iterable

import cv2
import numpy as np
from PIL import Image, ImageDraw


FPS = 30
WIDTH = 1920
HEIGHT = 1080
PAPER = "#f3ead2"
INK = "#55524b"
TRANSITION_SECONDS = 0.25
SAFE_RECT = (160, 130, 1760, 950)
MAX_SCENE_COVERAGE = 0.45
MIN_SOURCE_ALPHA_MARGIN_RATIO = 0.03
DRAW_COMPLETION_RATIO = 0.80
TIME_RE = re.compile(
    r"^(?P<sh>\d{2}):(?P<sm>\d{2}):(?P<ss>\d{2}),(?P<sms>\d{3})"
    r"\s+-->\s+"
    r"(?P<eh>\d{2}):(?P<em>\d{2}):(?P<es>\d{2}),(?P<ems>\d{3})$"
)


class PipelineError(ValueError):
    pass


@dataclass(frozen=True)
class Cue:
    index: int
    start_ms: int
    end_ms: int
    start_frame: int
    end_frame: int
    text: str


def nearest_frame(milliseconds: int) -> int:
    return math.floor(milliseconds * FPS / 1000 + 0.5)


def end_frame(milliseconds: int) -> int:
    return math.ceil(milliseconds * FPS / 1000)


def draw_timing(start: int, finish: int) -> tuple[int, int]:
    """Return the first fully drawn frame and the number of complete hold frames."""
    duration = max(1, finish - start)
    draw_frames = max(1, min(duration, math.ceil(duration * DRAW_COMPLETION_RATIO)))
    complete_frame = start + draw_frames - 1
    return complete_frame, duration - draw_frames


def drawing_progress(frame: int, start: int, finish: int) -> float:
    """Finish drawing early enough to show a stable complete result before the beat ends."""
    complete_frame, _ = draw_timing(start, finish)
    draw_frames = max(1, complete_frame - start + 1)
    return min(1.0, max(0.0, (frame - start + 1) / draw_frames))


def parse_timestamp(match: re.Match[str], prefix: str) -> int:
    hours = int(match.group(f"{prefix}h"))
    minutes = int(match.group(f"{prefix}m"))
    seconds = int(match.group(f"{prefix}s"))
    millis = int(match.group(f"{prefix}ms"))
    if minutes > 59 or seconds > 59:
        raise PipelineError("timestamp minutes and seconds must be between 00 and 59")
    return ((hours * 60 + minutes) * 60 + seconds) * 1000 + millis


def parse_srt(path: Path) -> list[Cue]:
    path = path.expanduser().resolve()
    if path.suffix.lower() != ".srt":
        raise PipelineError("input must have a .srt extension")
    if not path.is_file():
        raise PipelineError(f"input does not exist: {path}")
    try:
        raw = path.read_text(encoding="utf-8-sig")
    except UnicodeDecodeError as error:
        raise PipelineError("input must be valid UTF-8 or UTF-8 with BOM") from error
    if not raw.strip():
        raise PipelineError("subtitle file is empty")

    blocks = re.split(r"\r?\n\s*\r?\n", raw.strip())
    cues: list[Cue] = []
    for expected, block in enumerate(blocks, 1):
        lines = block.splitlines()
        if len(lines) < 2:
            raise PipelineError(f"cue {expected} must contain a time range and text")
        indexless = TIME_RE.fullmatch(lines[0].strip()) is not None
        if indexless:
            index = expected
            time_line = lines[0]
            text_lines = lines[1:]
        else:
            if len(lines) < 3:
                raise PipelineError(f"cue {expected} must contain index, time range, and text")
            try:
                index = int(lines[0].strip())
            except ValueError as error:
                raise PipelineError(f"cue {expected} has a non-integer index") from error
            if index != expected:
                raise PipelineError(f"cue indices must be consecutive from 1; expected {expected}, got {index}")
            time_line = lines[1]
            text_lines = lines[2:]
        match = TIME_RE.fullmatch(time_line.strip())
        if not match:
            raise PipelineError(f"cue {index} has an invalid SRT time range")
        start_ms = parse_timestamp(match, "s")
        finish_ms = parse_timestamp(match, "e")
        if finish_ms <= start_ms:
            raise PipelineError(f"cue {index} must have positive duration")
        text = "\n".join(line.strip() for line in text_lines).strip()
        if not text:
            raise PipelineError(f"cue {index} has empty text")
        cue = Cue(index, start_ms, finish_ms, nearest_frame(start_ms), end_frame(finish_ms), text)
        if cue.end_frame <= cue.start_frame:
            raise PipelineError(f"cue {index} is shorter than one output frame")
        if cues and start_ms < cues[-1].end_ms:
            raise PipelineError(f"cue {index} overlaps cue {cues[-1].index}")
        cues.append(cue)
    return cues


def export_dir_for(input_path: Path) -> Path:
    resolved = input_path.resolve()
    return resolved.with_name(f"{resolved.stem}-whiteboard")


def work_dir_for(input_path: Path) -> Path:
    return export_dir_for(input_path) / "work"


def output_for(input_path: Path) -> Path:
    resolved = input_path.resolve()
    return export_dir_for(resolved) / "output.mp4"


def prepare(input_path: Path) -> Path:
    cues = parse_srt(input_path)
    work = work_dir_for(input_path)
    (work / "assets").mkdir(parents=True, exist_ok=True)
    request = {
        "version": 1,
        "source": str(input_path.resolve()),
        "fps": FPS,
        "width": WIDTH,
        "height": HEIGHT,
        "safe_rect": list(SAFE_RECT),
        "style": {
            "paper": PAPER,
            "ink": INK,
            "minimum_negative_space": 0.55,
            "maximum_muted_fill_ratio": 0.18,
        },
        "cues": [asdict(cue) for cue in cues],
        "instructions": "Semantically group consecutive cues and create storyboard.json using the bundled schema.",
    }
    target = work / "storyboard-request.json"
    target.write_text(json.dumps(request, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    return target


def safe_child(work: Path, relative: str, required_parent: str | None = None) -> Path:
    candidate = (work / relative).resolve()
    try:
        candidate.relative_to(work.resolve())
    except ValueError as error:
        raise PipelineError(f"path escapes work directory: {relative}") from error
    if required_parent and required_parent not in candidate.relative_to(work.resolve()).parts:
        raise PipelineError(f"path must be below {required_parent}/: {relative}")
    return candidate


def load_storyboard(work: Path, cues: list[Cue]) -> dict[str, Any]:
    path = work / "storyboard.json"
    if not path.is_file():
        raise PipelineError(f"missing storyboard: {path}")
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        raise PipelineError(f"invalid storyboard JSON: {error}") from error
    if data.get("version") != 1 or not isinstance(data.get("scenes"), list) or not data["scenes"]:
        raise PipelineError("storyboard must have version 1 and a non-empty scenes array")

    all_indices = [cue.index for cue in cues]
    scene_partition: list[int] = []
    beat_partition: list[int] = []
    scene_ids: set[str] = set()
    beat_ids: set[str] = set()
    for scene_no, scene in enumerate(data["scenes"], 1):
        expected_scene_id = f"scene-{scene_no:03d}"
        if scene.get("id") != expected_scene_id or scene["id"] in scene_ids:
            raise PipelineError(f"scene id must be unique and sequential: {expected_scene_id}")
        scene_ids.add(scene["id"])
        indices = require_indices(scene.get("cue_indices"), f"scene {scene['id']}")
        scene_partition.extend(indices)
        beats = scene.get("beats")
        if not isinstance(beats, list) or not beats:
            raise PipelineError(f"scene {scene['id']} must contain beats")
        local_partition: list[int] = []
        for beat in beats:
            beat_id = beat.get("id")
            if not isinstance(beat_id, str) or not re.fullmatch(r"beat-\d{3}", beat_id) or beat_id in beat_ids:
                raise PipelineError("beat ids must be unique and match beat-NNN")
            beat_ids.add(beat_id)
            beat_indices = require_indices(beat.get("cue_indices"), f"beat {beat_id}")
            local_partition.extend(beat_indices)
            beat_partition.extend(beat_indices)
            if not str(beat.get("prompt", "")).strip():
                raise PipelineError(f"beat {beat_id} needs a visual prompt")
            for key in ("x", "y", "width", "height"):
                if not isinstance(beat.get(key), int):
                    raise PipelineError(f"beat {beat_id} {key} must be an integer")
            x, y, width, height = (beat[k] for k in ("x", "y", "width", "height"))
            if width <= 0 or height <= 0 or x < 0 or y < 0 or x + width > WIDTH or y + height > HEIGHT:
                raise PipelineError(f"beat {beat_id} rectangle is outside the canvas")
            asset = safe_child(work, str(beat.get("asset", "")), "assets")
            safe_child(work, str(beat.get("asset_chroma", "")), "assets")
            if not asset.is_file():
                raise PipelineError(f"missing keyed asset for {beat_id}: {asset}")
        if local_partition != indices:
            raise PipelineError(f"beats in {scene['id']} must partition its cues exactly once and in order")
    if scene_partition != all_indices or beat_partition != all_indices:
        raise PipelineError("scenes and beats must partition every cue exactly once in source order")
    return data


def require_indices(value: Any, context: str) -> list[int]:
    if not isinstance(value, list) or not value or not all(isinstance(item, int) for item in value):
        raise PipelineError(f"{context} cue_indices must be a non-empty integer array")
    return value


def cue_range(indices: Iterable[int], cues: list[Cue]) -> tuple[int, int]:
    selected = [cues[index - 1] for index in indices]
    return selected[0].start_frame, selected[-1].end_frame


def alpha_bounds(alpha: np.ndarray) -> tuple[int, int, int, int]:
    ys, xs = np.nonzero(alpha > 0)
    if not len(xs):
        raise PipelineError("asset is fully transparent")
    return int(xs.min()), int(ys.min()), int(xs.max()) + 1, int(ys.max()) + 1


def alpha_canvas(work: Path, beat: dict[str, Any]) -> tuple[Image.Image, np.ndarray]:
    asset_path = safe_child(work, beat["asset"], "assets")
    try:
        source = Image.open(asset_path).convert("RGBA")
    except Exception as error:
        raise PipelineError(f"cannot read RGBA asset {asset_path}: {error}") from error
    alpha = np.asarray(source.getchannel("A"))
    if not alpha.any():
        raise PipelineError(f"asset is fully transparent: {asset_path}")
    left, top, right, bottom = alpha_bounds(alpha)
    margin_x = max(1, math.ceil(source.width * MIN_SOURCE_ALPHA_MARGIN_RATIO))
    margin_y = max(1, math.ceil(source.height * MIN_SOURCE_ALPHA_MARGIN_RATIO))
    if left < margin_x or top < margin_y or source.width - right < margin_x or source.height - bottom < margin_y:
        raise PipelineError(
            f"asset subject is cropped or too close to an edge: {asset_path}; regenerate with blank chroma padding"
        )
    subject = source.crop((left, top, right, bottom))
    scale = min(beat["width"] / subject.width, beat["height"] / subject.height)
    fitted_width = max(1, round(subject.width * scale))
    fitted_height = max(1, round(subject.height * scale))
    resized = subject.resize((fitted_width, fitted_height), Image.Resampling.LANCZOS)
    offset_x = beat["x"] + (beat["width"] - fitted_width) // 2
    offset_y = beat["y"] + (beat["height"] - fitted_height) // 2
    canvas = Image.new("RGBA", (WIDTH, HEIGHT), (0, 0, 0, 0))
    canvas.alpha_composite(resized, (offset_x, offset_y))
    return canvas, np.asarray(canvas.getchannel("A"))


def compute_permit_mask(current: np.ndarray, future: np.ndarray, protected: np.ndarray, brush_radius: int) -> np.ndarray:
    base = current > 0
    if brush_radius > 0:
        size = brush_radius * 2 + 1
        kernel = cv2.getStructuringElement(cv2.MORPH_ELLIPSE, (size, size))
        base = cv2.dilate(base.astype(np.uint8), kernel).astype(bool)
    allowed = base & ~(future > 0) & ~(protected > 0)
    return allowed.astype(np.uint8) * 255


def drawable_stroke_mask(canvas: Image.Image, permit: np.ndarray) -> np.ndarray:
    rgba = np.asarray(canvas)
    rgb = rgba[:, :, :3].astype(np.int32)
    luminance = (rgb[:, :, 0] * 299 + rgb[:, :, 1] * 587 + rgb[:, :, 2] * 114) // 1000
    saturation = rgb.max(axis=2) - rgb.min(axis=2)
    marks = (rgba[:, :, 3] > 0) & ((luminance < 218) | (saturation > 34)) & (permit > 0)
    kernel = cv2.getStructuringElement(cv2.MORPH_ELLIPSE, (3, 3))
    return cv2.morphologyEx(marks.astype(np.uint8), cv2.MORPH_CLOSE, kernel)


def ordered_reveal_strokes(mask: np.ndarray, max_points_per_stroke: int = 48) -> list[list[tuple[int, int]]]:
    binary = (mask > 0).astype(np.uint8)
    contours, _ = cv2.findContours(binary, cv2.RETR_LIST, cv2.CHAIN_APPROX_NONE)
    candidates = [contour[:, 0, :] for contour in contours if len(contour) >= 2]
    candidates.sort(
        key=lambda points: (
            int(points[:, 1].min()),
            int(points[:, 0].min()),
            -float(cv2.arcLength(points.reshape(-1, 1, 2), True)),
        )
    )
    strokes: list[list[tuple[int, int]]] = []
    previous_tip: tuple[int, int] | None = None
    for points in candidates:
        stride = max(1, math.ceil(len(points) / max_points_per_stroke))
        sampled = [(int(x), int(y)) for x, y in points[::stride]]
        if len(sampled) < 2:
            continue
        if previous_tip is not None and distance(previous_tip, sampled[-1]) < distance(previous_tip, sampled[0]):
            sampled.reverse()
        strokes.append(sampled)
        previous_tip = sampled[-1]
    if not strokes:
        ys, xs = np.nonzero(binary)
        if not len(xs):
            raise PipelineError("stroke mask contains no drawable pixels")
        strokes = [[(int(xs[0]), int(ys[0]))]]
    return strokes


def distance(a: tuple[int, int], b: tuple[int, int]) -> float:
    return math.hypot(b[0] - a[0], b[1] - a[1])


def svg_path(strokes: list[list[tuple[int, int]]]) -> tuple[str, float]:
    commands: list[str] = []
    length = 0.0
    for stroke in strokes:
        commands.append(f"M {stroke[0][0]} {stroke[0][1]}")
        for previous, point in zip(stroke, stroke[1:]):
            commands.append(f"L {point[0]} {point[1]}")
            length += distance(previous, point)
    return " ".join(commands), max(length, 1.0)


def scene_coverage(masks: list[np.ndarray]) -> float:
    union = np.zeros((HEIGHT, WIDTH), dtype=np.uint8)
    for mask in masks:
        union = np.maximum(union, mask)
    return float(np.count_nonzero(union)) / float(WIDTH * HEIGHT)


def build(input_path: Path) -> Path:
    cues = parse_srt(input_path)
    work = work_dir_for(input_path)
    storyboard = load_storyboard(work, cues)
    hyperframes = work / "hyperframes"
    compositions = hyperframes / "compositions"
    generated_assets = hyperframes / "assets"
    if hyperframes.exists():
        shutil.rmtree(hyperframes)
    compositions.mkdir(parents=True)
    generated_assets.mkdir(parents=True)
    shutil.copy2(Path(__file__).parents[1] / "assets" / "hand-marker.png", generated_assets / "hand-marker.png")

    report_scenes: list[dict[str, Any]] = []
    root_clips: list[str] = []
    for scene_no, scene in enumerate(storyboard["scenes"], 1):
        scene_start, scene_end = cue_range(scene["cue_indices"], cues)
        canvases: list[Image.Image] = []
        masks: list[np.ndarray] = []
        for beat in scene["beats"]:
            canvas, alpha = alpha_canvas(work, beat)
            canvases.append(canvas)
            masks.append(alpha)
        coverage = scene_coverage(masks)
        if coverage > MAX_SCENE_COVERAGE:
            raise PipelineError(
                f"{scene['id']} covers {coverage:.1%} of the frame; keep at least 55% warm-paper negative space"
            )

        protected = np.zeros((HEIGHT, WIDTH), dtype=np.uint8)
        rendered_beats: list[dict[str, Any]] = []
        for beat_no, (beat, canvas, current) in enumerate(zip(scene["beats"], canvases, masks)):
            future = np.zeros((HEIGHT, WIDTH), dtype=np.uint8)
            for later in masks[beat_no + 1 :]:
                future = np.maximum(future, later)
            radius = max(5, min(14, round(min(beat["width"], beat["height"]) / 75)))
            allowed = compute_permit_mask(current, future, protected, radius)
            allowed &= (current > 0).astype(np.uint8) * 255
            rgba = np.asarray(canvas).copy()
            rgba[:, :, 3] = np.minimum(rgba[:, :, 3], allowed)
            output_name = f"{scene['id']}-{beat['id']}.png"
            Image.fromarray(rgba, "RGBA").save(generated_assets / output_name)
            mask_name = f"{scene['id']}-{beat['id']}-permit.png"
            Image.fromarray(allowed, "L").save(generated_assets / mask_name)
            stroke_mask = drawable_stroke_mask(canvas, allowed)
            strokes = ordered_reveal_strokes(stroke_mask)
            path_data, path_length = svg_path(strokes)
            beat_start, beat_end = cue_range(beat["cue_indices"], cues)
            draw_complete_frame, hold_frames = draw_timing(beat_start, beat_end)
            rendered_beats.append(
                {
                    "id": beat["id"],
                    "asset": output_name,
                    "mask": mask_name,
                    "start": (beat_start - scene_start) / FPS,
                    "duration": max(1 / FPS, (beat_end - beat_start) / FPS),
                    "path": path_data,
                    "path_length": path_length,
                    "strokes": strokes,
                    "stroke_count": len(strokes),
                    "brush_width": radius * 2,
                    "start_frame": beat_start,
                    "end_frame": beat_end,
                    "draw_complete_frame": draw_complete_frame,
                    "hold_frames": hold_frames,
                }
            )
            protected = np.maximum(protected, current)

        scene_duration = (scene_end - scene_start) / FPS
        scene_video = f"{scene['id']}.mp4"
        render_scene_video(generated_assets, scene_video, rendered_beats, scene_duration)
        scene_file = compositions / f"{scene['id']}.html"
        scene_file.write_text(scene_html(scene, scene_video, scene_duration), encoding="utf-8")
        root_clips.append(
            f'<div id="clip-{scene["id"]}" data-composition-id="{scene["id"]}" '
            f'data-composition-src="compositions/{scene["id"]}.html" data-start="{scene_start / FPS:.6f}" '
            f'data-duration="{scene_duration:.6f}" data-track-index="{scene_no}"></div>'
        )
        report_scenes.append(
            {
                "id": scene["id"],
                "start_frame": scene_start,
                "end_frame": scene_end,
                "coverage": round(coverage, 6),
                "beats": [
                    {
                        key: beat[key]
                        for key in (
                            "id", "start_frame", "draw_complete_frame", "end_frame", "hold_frames", "mask"
                        )
                    }
                    for beat in rendered_beats
                ],
            }
        )

    total_frames = cues[-1].end_frame
    (hyperframes / "index.html").write_text(root_html(root_clips, total_frames / FPS), encoding="utf-8")
    (hyperframes / "DESIGN.md").write_text(
        "# Whiteboard Visual Identity\n\n"
        "## Colors\n\n- Aged cream paper `#f3ead2`\n- Graphite-gray line `#55524b`\n"
        "- Warm red `#e65443`\n- Muted blue `#4d82d5`\n- Soft green `#35ae7a`\n\n"
        "## Motion\n\nReveal only along ordered pen paths. Hold completed drawings. Use a 0.25-second paper wipe between scenes.\n\n"
        "## What NOT to Do\n\nNo captions, audio, dense fills, glossy cartoon rendering, cropped figures, crowded scenery, gradients, floating motion, or global rectangular drawing reveals.\n",
        encoding="utf-8",
    )
    report = {
        "version": 1,
        "source": str(input_path.resolve()),
        "output": str(output_for(input_path)),
        "video": {"width": WIDTH, "height": HEIGHT, "fps": FPS, "codec": "h264", "audio": False},
        "total_frames": total_frames,
        "duration_seconds": total_frames / FPS,
        "cue_frame_map": [asdict(cue) for cue in cues],
        "scenes": report_scenes,
        "status": "composition-built",
    }
    report_path = work / "render-report.json"
    report_path.write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    return hyperframes


def aged_paper_frame() -> Image.Image:
    base = np.empty((HEIGHT, WIDTH, 3), dtype=np.int16)
    base[:, :, :] = np.array((243, 234, 210), dtype=np.int16)
    rng = np.random.default_rng(20260806)
    grain = rng.integers(-2, 3, size=(HEIGHT, WIDTH, 1), dtype=np.int16)
    base = np.clip(base + grain, 0, 255).astype(np.uint8)
    return Image.fromarray(base, "RGB").convert("RGBA")


def composite_with_mask(base: Image.Image, layer_path: Path, reveal: np.ndarray) -> None:
    layer = Image.open(layer_path).convert("RGBA")
    rgba = np.asarray(layer).copy()
    rgba[:, :, 3] = np.minimum(rgba[:, :, 3], reveal)
    base.alpha_composite(Image.fromarray(rgba, "RGBA"))


def reveal_for_progress(beat: dict[str, Any], progress: float) -> tuple[np.ndarray, tuple[int, int]]:
    permit = np.asarray(Image.open(beat["permit_path"]).convert("L"))
    strokes = beat["strokes"]
    if progress >= 1.0:
        return permit, strokes[-1][-1]
    total_points = sum(len(stroke) for stroke in strokes)
    remaining = max(1, min(total_points, math.ceil(total_points * progress)))
    stroke = np.zeros((HEIGHT, WIDTH), dtype=np.uint8)
    active_tip = strokes[0][0]
    for points in strokes:
        if remaining <= 0:
            break
        active = points[:remaining]
        if len(active) == 1:
            cv2.circle(stroke, active[0], beat["brush_width"] // 2, 255, -1, cv2.LINE_AA)
        else:
            cv2.polylines(stroke, [np.asarray(active, dtype=np.int32)], False, 255, beat["brush_width"], cv2.LINE_AA)
        active_tip = active[-1]
        remaining -= len(active)
    return np.minimum(stroke, permit), active_tip


def render_scene_video(asset_dir: Path, output_name: str, beats: list[dict[str, Any]], duration: float) -> None:
    output = asset_dir / output_name
    total_frames = max(1, round(duration * FPS))
    hand = Image.open(Path(__file__).parents[1] / "assets" / "hand-marker.png").convert("RGBA")
    hand = hand.resize((540, 540), Image.Resampling.LANCZOS)
    for beat in beats:
        beat["permit_path"] = asset_dir / beat["mask"]
    command = [
        "ffmpeg", "-y", "-v", "error", "-f", "rawvideo", "-pix_fmt", "rgb24",
        "-s", f"{WIDTH}x{HEIGHT}", "-r", str(FPS), "-i", "-", "-an", "-c:v", "libx264",
        "-preset", "veryfast", "-crf", "18", "-pix_fmt", "yuv420p", "-movflags", "+faststart", str(output),
    ]
    try:
        process = subprocess.Popen(command, stdin=subprocess.PIPE, stderr=subprocess.PIPE)
    except FileNotFoundError as error:
        raise PipelineError("ffmpeg is required to encode deterministic scene videos") from error
    assert process.stdin is not None
    paper = aged_paper_frame()
    try:
        for local_frame in range(total_frames):
            frame = paper.copy()
            active_hand: tuple[int, int] | None = None
            for beat in beats:
                start = round(beat["start"] * FPS)
                finish = start + round(beat["duration"] * FPS)
                if local_frame < start:
                    continue
                if local_frame >= finish:
                    reveal = np.asarray(Image.open(beat["permit_path"]).convert("L"))
                else:
                    progress = drawing_progress(local_frame, start, finish)
                    reveal, active_hand = reveal_for_progress(beat, progress)
                composite_with_mask(frame, asset_dir / beat["asset"], reveal)
            if active_hand is not None:
                frame.alpha_composite(hand, (active_hand[0] - 112, active_hand[1] - 82))
            process.stdin.write(np.asarray(frame.convert("RGB"), dtype=np.uint8).tobytes())
        process.stdin.close()
        stderr = process.stderr.read().decode("utf-8", errors="replace") if process.stderr else ""
        if process.stderr:
            process.stderr.close()
        code = process.wait()
    except BrokenPipeError as error:
        stderr = process.stderr.read().decode("utf-8", errors="replace") if process.stderr else ""
        raise PipelineError(f"ffmpeg stopped while encoding {output_name}: {stderr.strip()}") from error
    if code:
        raise PipelineError(f"ffmpeg failed while encoding {output_name}: {stderr.strip()}")


def scene_html(scene: dict[str, Any], scene_video: str, duration: float) -> str:
    scene_id = scene["id"]
    wipe_duration = min(TRANSITION_SECONDS, max(1 / FPS, duration / 2))
    return f"""<!doctype html>
<html><head><meta charset="utf-8"><style>
html,body{{margin:0;width:100%;height:100%;overflow:hidden;background:{PAPER};}}
[data-composition-id="{scene_id}"]{{position:relative;width:1920px;height:1080px;overflow:hidden;background-color:{PAPER};}}
.paper{{position:absolute;inset:0;background-color:{PAPER};background-image:repeating-linear-gradient(8deg,rgba(52,49,45,.018) 0,rgba(52,49,45,.018) 1px,rgba(243,234,210,0) 1px,rgba(243,234,210,0) 7px);}}
.scene-video{{position:absolute;inset:0;width:1920px;height:1080px;object-fit:cover;}}
.clip{{visibility:hidden;}}
.paper-wipe{{position:absolute;inset:0;background:{PAPER};z-index:60;transform-origin:right center;}}
</style></head><body>
<template id="{scene_id}-template"><div data-composition-id="{scene_id}" data-width="1920" data-height="1080">
<div class="paper" data-layout-ignore></div>
<video id="video-{scene_id}" class="scene-video clip" src="assets/{scene_video}" data-start="0" data-duration="{duration:.6f}" data-track-index="0" muted playsinline></video>
<div id="paper-wipe" class="paper-wipe" data-layout-ignore></div>
<script src="https://cdn.jsdelivr.net/npm/gsap@3.14.2/dist/gsap.min.js"></script>
<script>
window.__timelines=window.__timelines||{{}};
const tl=gsap.timeline({{paused:true}});
gsap.set('#paper-wipe',{{scaleX:1}});
tl.to('#paper-wipe',{{scaleX:0,duration:{wipe_duration:.6f},ease:'power2.inOut'}},0.000001);
window.__timelines['{scene_id}']=tl;
</script></div></template></body></html>"""


def root_html(clips: list[str], duration: float) -> str:
    return f"""<!doctype html>
<html><head><meta charset="utf-8"><style>
html,body{{margin:0;width:100%;height:100%;overflow:hidden;background:{PAPER};}}
#whiteboard-root{{position:relative;width:1920px;height:1080px;overflow:hidden;background-color:{PAPER};}}
</style></head><body><div id="whiteboard-root" data-composition-id="whiteboard-root" data-start="0" data-duration="{duration:.6f}" data-width="1920" data-height="1080">
{''.join(clips)}
<script src="https://cdn.jsdelivr.net/npm/gsap@3.14.2/dist/gsap.min.js"></script>
<script>window.__timelines=window.__timelines||{{}};window.__timelines['whiteboard-root']=gsap.timeline({{paused:true}});</script>
</div></body></html>"""


def style_check(input_path: Path) -> Path:
    cues = parse_srt(input_path)
    work = work_dir_for(input_path)
    storyboard = load_storyboard(work, cues)
    beats = [beat for scene in storyboard["scenes"] for beat in scene["beats"]]
    columns = min(3, len(beats))
    rows = math.ceil(len(beats) / columns)
    cell_width, cell_height = 600, 420
    sheet = Image.new("RGB", (columns * cell_width, rows * cell_height), PAPER)
    draw = ImageDraw.Draw(sheet)
    metrics: list[dict[str, Any]] = []
    for position, beat in enumerate(beats):
        asset_path = safe_child(work, beat["asset"], "assets")
        image = Image.open(asset_path).convert("RGBA")
        alpha = np.asarray(image.getchannel("A"))
        left, top, right, bottom = alpha_bounds(alpha)
        subject = image.crop((left, top, right, bottom))
        scale = min((cell_width - 80) / subject.width, (cell_height - 70) / subject.height)
        fitted = subject.resize(
            (max(1, round(subject.width * scale)), max(1, round(subject.height * scale))),
            Image.Resampling.LANCZOS,
        )
        column, row = position % columns, position // columns
        x = column * cell_width + (cell_width - fitted.width) // 2
        y = row * cell_height + 38 + (cell_height - 58 - fitted.height) // 2
        sheet.paste(fitted, (x, y), fitted)
        draw.text((column * cell_width + 18, row * cell_height + 14), beat["id"], fill=INK)

        rgba = np.asarray(image)
        rgb = rgba[:, :, :3].astype(np.int32)
        visible = rgba[:, :, 3] > 16
        luminance = (rgb[:, :, 0] * 299 + rgb[:, :, 1] * 587 + rgb[:, :, 2] * 114) // 1000
        saturation = rgb.max(axis=2) - rgb.min(axis=2)
        visible_count = max(1, int(np.count_nonzero(visible)))
        dark = visible & (luminance < 218)
        accents = visible & (saturation > 34)
        metrics.append(
            {
                "id": beat["id"],
                "asset": beat["asset"],
                "dark_mark_ratio": round(float(np.count_nonzero(dark)) / visible_count, 4),
                "accent_ratio": round(float(np.count_nonzero(accents)) / visible_count, 4),
                "median_dark_luminance": int(np.median(luminance[dark])) if np.any(dark) else None,
            }
        )

    qa = work / "qa"
    qa.mkdir(parents=True, exist_ok=True)
    target = qa / "style-contact-sheet.png"
    sheet.save(target)
    (qa / "style-metrics.json").write_text(
        json.dumps({"version": 1, "assets": metrics}, ensure_ascii=False, indent=2) + "\n",
        encoding="utf-8",
    )
    return target


def ffprobe_json(output: Path) -> dict[str, Any]:
    command = [
        "ffprobe", "-v", "error", "-show_streams", "-show_format", "-of", "json", str(output)
    ]
    try:
        completed = subprocess.run(command, check=True, capture_output=True, text=True)
    except FileNotFoundError as error:
        raise PipelineError("ffprobe is required for verification") from error
    except subprocess.CalledProcessError as error:
        raise PipelineError(f"ffprobe failed: {error.stderr.strip()}") from error
    return json.loads(completed.stdout)


def verify(input_path: Path) -> Path:
    cues = parse_srt(input_path)
    work = work_dir_for(input_path)
    output = output_for(input_path)
    report_path = work / "render-report.json"
    if not output.is_file() or not report_path.is_file():
        raise PipelineError("rendered MP4 or render-report.json is missing")
    probe = ffprobe_json(output)
    video_streams = [stream for stream in probe.get("streams", []) if stream.get("codec_type") == "video"]
    audio_streams = [stream for stream in probe.get("streams", []) if stream.get("codec_type") == "audio"]
    if len(video_streams) != 1 or audio_streams:
        raise PipelineError("output must have exactly one video stream and no audio stream")
    video = video_streams[0]
    if video.get("codec_name") != "h264" or video.get("width") != WIDTH or video.get("height") != HEIGHT:
        raise PipelineError("output must be 1920x1080 H.264")
    rate = video.get("avg_frame_rate", "0/1").split("/")
    actual_fps = float(rate[0]) / float(rate[1])
    if abs(actual_fps - FPS) > 0.001:
        raise PipelineError(f"output frame rate is {actual_fps}, expected {FPS}")
    actual_duration = float(probe["format"]["duration"])
    expected_duration = cues[-1].end_frame / FPS
    if abs(actual_duration - expected_duration) > 1 / FPS + 0.001:
        raise PipelineError(f"output duration {actual_duration:.3f}s differs from {expected_duration:.3f}s")
    report = json.loads(report_path.read_text(encoding="utf-8"))
    report["status"] = "verified"
    report["probe"] = {
        "codec": video["codec_name"], "width": video["width"], "height": video["height"],
        "fps": actual_fps, "duration_seconds": actual_duration, "audio_streams": 0,
    }
    report_path.write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    return report_path


def parser() -> argparse.ArgumentParser:
    result = argparse.ArgumentParser(description=__doc__)
    result.add_argument("command", choices=("prepare", "style-check", "build", "verify"))
    result.add_argument("input", type=Path, help="the only user input; must be a standard timed .srt file")
    return result


def main() -> int:
    args = parser().parse_args()
    try:
        if args.command == "prepare":
            target = prepare(args.input)
        elif args.command == "style-check":
            target = style_check(args.input)
        elif args.command == "build":
            target = build(args.input)
        else:
            target = verify(args.input)
    except PipelineError as error:
        print(f"whiteboard pipeline error: {error}", file=sys.stderr)
        return 2
    print(target)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
