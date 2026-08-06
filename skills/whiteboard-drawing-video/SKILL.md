---
name: whiteboard-drawing-video
description: Turn one standard timed .srt subtitle file into a silent, frame-synchronized 1920x1080 whiteboard hand-drawing MP4 using semantic storyboards, Codex ImageGen assets, Python permit masks and stroke paths, and HyperFrames. Use when the user asks for 白板手绘动画, whiteboard drawing animation, 字幕转分镜手绘视频, or a hand-drawn explainer video driven strictly by SRT timing.
---

# Whiteboard Drawing Video

Create the whole video from exactly one `.srt` input. Do not accept another user-provided media input. Require UTF-8 SRT time ranges with millisecond precision. Accept standard numbered cues and indexless SRT-style blocks; assign consecutive indices in memory without rewriting the source.

## Fixed contract

- Render 1920x1080 at 30 fps as silent H.264 MP4.
- Do not display or mux subtitles.
- Create exactly one sibling export directory named `<input-stem>-whiteboard/`. Put the final video at `<input-stem>-whiteboard/output.mp4` and every generated or diagnostic artifact under `<input-stem>-whiteboard/work/`. Do not scatter generated files beside the input.
- Keep each visual beat inside its cue interval. Complete every stroke and the full permit reveal within the first 80% of the beat, then hold the identical complete drawing for the final 20%. Only a one-frame beat may omit the hold.
- Quantize cue starts to the nearest frame and cue ends upward. Allow at most one frame of timing error.
- Hold the completed board through subtitle gaps. Start each semantic scene on a fresh warm-white board.
- Reject wrong extensions, invalid UTF-8, empty files, malformed or overlapping cues, reversed ranges, and non-increasing indices before generating images.

## Required workflow

1. Read the full `.srt` file and no other user input. Run:

   ```bash
   uv run --script scripts/whiteboard_pipeline.py prepare /absolute/input.srt
   ```

   Use the absolute script path when running outside this skill directory. The command creates `<stem>-whiteboard/work/storyboard-request.json`.

2. Read [references/storyboard-schema.md](references/storyboard-schema.md). Semantically group consecutive cues. Create `storyboard.json` in the work directory. Preserve every cue exactly once and in order. Prefer 2-5 related cues per scene; keep a single cue alone when its topic or setting changes.

3. Read [references/visual-style.md](references/visual-style.md). Before generating beats, call the built-in ImageGen tool once per semantic scene to create `assets/<scene-id>-style-anchor-chroma.png`: a compact model sheet containing every recurring character type in that scene, all drawn with one line weight, face construction, eye style, hand and foot design, body proportion, hatching density, and accent palette. Remove the anchor's chroma key and save `assets/<scene-id>-style-anchor.png`. Do not render the anchor into the video.

   For every beat, call the built-in ImageGen tool once and pass the local keyed anchor through `referenced_image_paths`. When a recurring character already exists, also pass the closest prior keyed beat asset as a second reference. Never rely on prompt text or recent-conversation image memory alone when a local reference exists. Generate an isolated opaque illustration element on a perfectly flat `#00ff00` background, with no text, logo, watermark, paper, hand, marker, shadow, or scenery outside the requested element.

   Start every beat prompt with this invariant: `Match the attached scene style anchor exactly: same graphite line weight, rounded head-to-body ratio, dot-oval eyes, three-finger hand construction, rounded bare feet or simple shoes, minimal hatching, cream-white interiors, and muted accent saturation. Change only the requested pose and props.` Do not let humans, animals, props, or later beats drift into a more realistic, detailed, thinner, thicker, or more polished style.

   For every person or animal, explicitly require the complete figure: head, hair or ears, torso, both arms and hands, both legs and feet, and tail when applicable. Keep every extremity visible with at least 8% blank chroma padding on all four sides. Never crop a character unless the storyboard explicitly requests a close-up.

4. Copy each generated source into the path declared by `asset_chroma`, then remove its chroma key:

   ```bash
   python3 "${CODEX_HOME:-$HOME/.codex}/skills/.system/imagegen/scripts/remove_chroma_key.py" \
     --input <asset_chroma> --out <asset> --auto-key border --soft-matte \
     --transparent-threshold 12 --opaque-threshold 220 --despill
   ```

   Inspect each alpha result. Require transparent corners, a clean opaque subject, and an alpha bounding box separated from every source edge by at least 3% of that image dimension. Inspect characters for complete heads, faces, hands, feet, limbs, clothing, and tails. Regenerate cropped or incomplete figures; do not repair missing anatomy by scaling. Retry once with `--edge-contract 1` only when a green fringe remains. Do not switch to a model-native transparency path without user approval.

5. Before building, create the deterministic style QA sheet:

   ```bash
   uv run --script scripts/whiteboard_pipeline.py style-check /absolute/input.srt
   ```

   Inspect `work/qa/style-contact-sheet.png` at original resolution. Compare every asset simultaneously for line thickness, graphite darkness, eye and face construction, hand and foot design, head-to-body ratio, hatching density, and accent saturation. Regenerate every outlier with the scene anchor and nearest matching asset as references. Repeat `style-check` until no asset looks like a different illustrator. Do not proceed merely because each asset looks acceptable in isolation.

6. Build the permit masks and human-ordered stroke paths. Extract graphite and muted-color marks from the RGBA asset instead of scanning its opaque cream interior. Split disconnected contours into separate pen strokes; order them top-to-bottom and left-to-right, preferring the longer outer contour before nearby details. Lift the marker between disconnected strokes and draw only while following a stroke. Let Python stream deterministic frames with the fixed hand sprite into one silent H.264 clip per semantic scene; let HyperFrames own scene placement, paper transitions, the root timeline, and final assembly:

   ```bash
   uv run --script scripts/whiteboard_pipeline.py build /absolute/input.srt
   ```

   The deterministic permit mask is `dilate(Self, brushRadius) - Future - Protected`, intersected with the canvas. Never replace it with a simple global wipe.

   Never use horizontal, vertical, serpentine, raster-scan, or rectangular fill paths. Reach the complete permit mask at `draw_complete_frame`, 80% through the beat, so facial features, hands, feet, props, and contour fragments cannot pop into existence at the cue or scene boundary. Keep that exact complete mask unchanged through `end_frame`; do not defer cleanup to the final frame.

7. Read the installed `hyperframes`, `hyperframes-cli`, and `gsap` skills. From the generated HyperFrames directory run, in order:

   ```bash
   npx hyperframes lint
   npx hyperframes validate
   npx hyperframes inspect --samples 15
   npx hyperframes render --output /absolute/<stem>-whiteboard/output.mp4 --fps 30 --quality high --strict
   ```

   Also run the HyperFrames animation-map script when available. Fix every lint or layout error before rendering. Do not add audio.

8. Verify the output:

   ```bash
   uv run --script scripts/whiteboard_pipeline.py verify /absolute/input.srt
   ```

   Inspect the first frame, every cue boundary, every scene transition, the middle of every long beat, every `draw_complete_frame`, the frame immediately before every `end_frame`, and the final frame. Confirm the two completion/hold samples are visually identical for that beat, the hand tip follows the active stroke, the marker lifts between disconnected strokes, no regular horizontal or vertical gaps appear, future elements remain invisible, and protected pixels are not repainted.

## Storyboard rules

- Translate meaning into drawable nouns and actions, not decorative text.
- Use one to three elements per cue. Favor a clear narrative sequence over dense diagrams.
- Assign non-overlapping placements inside the 1600x820 safe drawing area at `(160, 130)`.
- Keep recurring characters visually consistent within a scene.
- Keep unrelated character species in the same illustrator's visual grammar. Children must not use finer lines, more realistic hair, extra fingers, smaller eyes, or more detailed clothes than the monkeys.
- Preserve each asset's aspect ratio when assigning its storyboard rectangle. Never stretch a person or animal to fill a box.
- Keep complete people at least 260 pixels tall in the final frame and leave at least 40 pixels between their alpha bounds and the canvas edge.
- Begin each scene with a 0.25-second paper wipe. Draw during the cue interval; shorten the wipe for very short scenes.
- Let Python own masks, paths, frame math, HTML generation, and report generation. Do not hand-edit generated masks or composition timing.
- Keep the source `.srt` in place. Never copy it or create additional top-level generated files outside the single export directory.

## Failure behavior

- Stop before ImageGen when subtitle validation fails.
- Stop before mask construction when the style contact sheet has not been inspected or contains a visible outlier. Stop before HyperFrames when a declared asset is missing, not RGBA, empty after key removal, touches its source edge, falls outside the canvas, or is mapped to an invalid cue.
- Preserve the work directory on failure and report the failing stage and path.
- Never silently create placeholder illustrations for a final render.
