---
name: twenty-four-frame-sequence
description: Generate a coherent 24-frame image sequence from a theme, action description, or visual reference, preserving character, camera, scene, lighting, and object identity across frames. Use when the user asks for 24帧图片、1秒24fps分镜、连续动作图、动画帧、逐帧图片、sprite sheet、故事板，或要求按固定24帧格式生图；deliver a complete 4×6 contact sheet and optionally 24 individual PNG files, without making a video unless explicitly requested.
---

# Generate a 24-frame image sequence

Create exactly 24 coherent frames. Prioritize complete cells, locked composition, and temporal continuity over dramatic variation.

## Workflow

1. Load the `imagegen` skill and follow its built-in image-generation workflow.
2. Inspect every reference image before generating. Treat it as a style/identity reference unless the user explicitly requests an edit.
3. Derive a simple motion arc with a clear start, middle, and end. Divide it into 24 tiny chronological changes.
4. Generate a temporary **5 columns × 5 rows** square sheet. Put frames 01–24 in reading order and leave cell 25 blank. Do not ask the model to generate a six-column sheet; its final column is prone to canvas overflow.
5. Require generous outer margins and complete borders around all five columns. Keep the camera, subjects, set, lighting, crop, lens, scale, and palette fixed.
6. Inspect the result. Confirm that cells 05, 10, 15, 20, and frame 24 are fully visible. Regenerate with larger margins if any cell touches or crosses the canvas edge.
7. Run `scripts/package_24_frames.py` on the accepted temporary sheet. This extracts frames 01–24 and creates a clean **4 columns × 6 rows** delivery sheet.
8. Inspect the final delivery sheet and at least frames 01, 12, 20, and 24. Confirm that there are exactly 24 files and no cropped right edges.
9. Report the saved delivery-sheet path and ZIP path. Do not create a GIF or video unless explicitly requested.

## Generation prompt contract

Include all of the following in the image-generation prompt, adapting only the subject and action:

```text
Use case: illustration-story
Asset type: 24-frame coherent animation image sequence

Create a SAFE 5 columns × 5 rows storyboard sheet. Frames 01–24 occupy the first 24 cells in reading order. Cell 25 is blank dark background. Do not create a sixth column.

Canvas safety: place the complete grid inside a solid outer border at least 5% of canvas width on both left and right and at least 3% on top and bottom. Every cell must have a full visible border. Nothing may touch or cross the canvas edge.

Continuity: use one locked camera. Preserve the exact same character identity, face, body proportions, clothing, scene geometry, props, lighting, exposure, lens, crop, and color palette. Change only the elements required by the action, by a tiny amount in each adjacent frame. No cuts, alternate angles, time jumps, duplicated frames, or unrelated poses.

Motion: <describe frames 01–24 as 4–6 contiguous phases>.

Labels: put small legible frame numbers 01–24 at the top-left inside each cell. Do not put a number or scene in cell 25. No other captions, logos, watermarks, or unrequested text.

Critical checks: exactly 24 numbered scenes; cells 05, 10, 15, and 20 must be complete at the right edge of their rows; frame 24 must be complete.
```

When the user provides a reference, explicitly identify which traits must remain invariant. Avoid asking for exact rendered words unless text is essential; generated text often drifts between frames.

## Package the result

Run:

```bash
python scripts/package_24_frames.py \
  --input <temporary-sheet.png> \
  --output-dir <delivery-directory>
```

The script uses normalized grid bounds tuned to the required safety prompt. If inspection shows different bounds, pass measured normalized coordinates:

```bash
python scripts/package_24_frames.py \
  --input <temporary-sheet.png> \
  --output-dir <delivery-directory> \
  --bounds left,top,right,bottom
```

Coordinates range from 0 to 1. Measure the outer edge of the 5×5 grid, not the canvas edge. The script removes the thin cell gutter, exports `frame_01.png` through `frame_24.png`, creates `contact_sheet_24_complete.png`, and packages the frames as a ZIP.

## Acceptance criteria

- Deliver exactly 24 individual PNG files.
- Deliver one 4×6 contact sheet containing exactly those 24 frames.
- Keep every frame fully inside its cell with visible left and right content.
- Keep the same subjects and environment throughout.
- Make neighboring frames differ by small, physically plausible changes.
- Preserve chronological reading order: left-to-right, top-to-bottom.
- Never substitute a video for the requested images.
