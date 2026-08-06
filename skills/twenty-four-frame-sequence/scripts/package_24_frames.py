#!/usr/bin/env python3
"""Extract 24 frames from a safe 5x5 generation sheet and package them."""

from __future__ import annotations

import argparse
import shutil
from pathlib import Path

from PIL import Image


def parse_bounds(value: str) -> tuple[float, float, float, float]:
    try:
        bounds = tuple(float(part) for part in value.split(","))
    except ValueError as exc:
        raise argparse.ArgumentTypeError("bounds must contain four numbers") from exc
    if len(bounds) != 4:
        raise argparse.ArgumentTypeError("bounds must be left,top,right,bottom")
    left, top, right, bottom = bounds
    if not (0 <= left < right <= 1 and 0 <= top < bottom <= 1):
        raise argparse.ArgumentTypeError("bounds must be ordered values from 0 to 1")
    return left, top, right, bottom


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--input", required=True, type=Path)
    parser.add_argument("--output-dir", required=True, type=Path)
    parser.add_argument(
        "--bounds",
        type=parse_bounds,
        default=(0.044, 0.022, 0.951, 0.968),
        help="normalized outer bounds of the 5x5 grid",
    )
    parser.add_argument("--gutter", type=float, default=0.022, help="fraction trimmed inside each cell")
    parser.add_argument("--frame-width", type=int, default=880)
    parser.add_argument("--frame-height", type=int, default=920)
    args = parser.parse_args()

    if not args.input.is_file():
        parser.error(f"input does not exist: {args.input}")
    if not 0 <= args.gutter < 0.1:
        parser.error("gutter must be between 0 and 0.1")
    if args.frame_width < 1 or args.frame_height < 1:
        parser.error("frame dimensions must be positive")

    frames_dir = args.output_dir / "frames"
    frames_dir.mkdir(parents=True, exist_ok=True)
    for stale in frames_dir.glob("frame_*.png"):
        stale.unlink()

    sheet = Image.open(args.input).convert("RGB")
    left, top, right, bottom = args.bounds
    x0, y0 = round(left * sheet.width), round(top * sheet.height)
    x1, y1 = round(right * sheet.width), round(bottom * sheet.height)
    x_edges = [round(x0 + i * (x1 - x0) / 5) for i in range(6)]
    y_edges = [round(y0 + i * (y1 - y0) / 5) for i in range(6)]

    for index in range(24):
        row, col = divmod(index, 5)
        tile = sheet.crop((x_edges[col], y_edges[row], x_edges[col + 1], y_edges[row + 1]))
        trim_x = round(tile.width * args.gutter)
        trim_y = round(tile.height * args.gutter)
        tile = tile.crop((trim_x, trim_y, tile.width - trim_x, tile.height - trim_y))
        tile = tile.resize((args.frame_width, args.frame_height), Image.Resampling.LANCZOS)
        tile.save(frames_dir / f"frame_{index + 1:02d}.png", compress_level=3)

    thumb_w, thumb_h = args.frame_width // 4, args.frame_height // 4
    gap, margin = 6, 18
    delivery = Image.new(
        "RGB",
        (margin * 2 + thumb_w * 4 + gap * 3, margin * 2 + thumb_h * 6 + gap * 5),
        (49, 29, 17),
    )
    for index in range(24):
        frame = Image.open(frames_dir / f"frame_{index + 1:02d}.png")
        frame = frame.resize((thumb_w, thumb_h), Image.Resampling.LANCZOS)
        col, row = index % 4, index // 4
        delivery.paste(frame, (margin + col * (thumb_w + gap), margin + row * (thumb_h + gap)))

    args.output_dir.mkdir(parents=True, exist_ok=True)
    delivery_path = args.output_dir / "contact_sheet_24_complete.png"
    delivery.save(delivery_path, compress_level=3)
    zip_path = shutil.make_archive(str(args.output_dir / "frames_24"), "zip", frames_dir)

    exported = list(frames_dir.glob("frame_*.png"))
    if len(exported) != 24:
        raise RuntimeError(f"expected 24 frames, found {len(exported)}")
    print(f"frames: {frames_dir}")
    print(f"contact sheet: {delivery_path}")
    print(f"zip: {zip_path}")


if __name__ == "__main__":
    main()
