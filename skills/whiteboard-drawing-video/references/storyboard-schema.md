# Storyboard schema

Write `storyboard.json` beside `storyboard-request.json` using this shape:

```json
{
  "version": 1,
  "scenes": [
    {
      "id": "scene-001",
      "cue_indices": [1, 2],
      "summary": "One concise semantic setting",
      "beats": [
        {
          "id": "beat-001",
          "cue_indices": [1],
          "prompt": "An isolated drawable subject or action",
          "asset_chroma": "assets/beat-001-chroma.png",
          "asset": "assets/beat-001.png",
          "x": 180,
          "y": 210,
          "width": 620,
          "height": 520
        }
      ]
    }
  ]
}
```

## Invariants

- Use version `1`.
- Partition all cue indices exactly once across scenes, in source order.
- Partition each scene's cue indices exactly once across its beats, in order.
- Keep scene and beat IDs unique and match `scene-NNN` / `beat-NNN`.
- Resolve asset paths relative to the work directory and keep them below its `assets/` directory.
- Use integer coordinates and dimensions. Keep each element wholly inside the 1920x1080 canvas and preferably within the safe drawing area.
- Treat `width` and `height` as a placement box. Python trims transparent margins, preserves the subject aspect ratio, and centers it inside this box without stretching.
- Give full-body people and animals enough room to remain legible and keep their final alpha bounds at least 40 pixels from the canvas edge.
- Do not overlap beat rectangles unless the story requires contact between the objects. Python still prevents future pixels from leaking into current beats.
- Put only visual content in `prompt`; never include subtitle text as visible writing.

Python derives all start/end frames from the referenced cues. Do not add editable timing fields.
