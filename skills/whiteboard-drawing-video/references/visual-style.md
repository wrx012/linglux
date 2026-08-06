# Whiteboard visual identity

Use this identity for every generated illustration and HyperFrames composition.

## Style prompt

Create a gentle children's-story whiteboard illustration on warm aged cream paper. Use loose graphite-gray pencil contours with slightly uneven pressure, rounded simplified anatomy, minimal interior lines, and only a few short hatching strokes for rock creases or motion. Leave most forms unfilled so the cream paper shows through. Reserve color for tiny muted narrative accents such as a yellow banana or a small red or blue clothing detail. Prefer one clean silhouette and a simple curved ground line over detailed scenery. Keep every subject visually separated and preserve broad quiet areas of paper.

Match the supplied reference look, not polished digital cartoon art: no glossy eyes, detailed fur rendering, photorealistic anatomy, heavy colored fills, sharp vector outlines, or dense cross-hatching. Characters should resemble light pencil storybook sketches with friendly proportions and economical marks.

## Scene style lock

Create one scene anchor before any renderable asset. Include representative full-body recurring animals and humans in neutral poses. Use the keyed anchor as a local image reference for every beat in that scene.

Lock these traits across all subjects:

- One graphite contour weight; do not use thin detailed lines for people and thick cartoon lines for animals.
- Heads about 35-42% of full character height, rounded cheeks, small simple noses, dot-oval eyes with one highlight at most.
- Simplified three-finger-plus-thumb hands and rounded feet or minimally detailed shoes.
- Hair represented by a few grouped contour masses, not many realistic strands.
- Clothing described with 3-8 structural lines, not fashion-illustration folds or textures.
- No species-specific rendering upgrade: humans and monkeys share the same simplification, hatching density, and finish level.

When an asset drifts, regenerate it from the anchor. Do not normalize mismatched styles later with color grading, resizing, or line extraction.

## Palette

- Aged cream paper: `#f3ead2`
- Graphite-gray line: `#55524b`
- Soft construction line: `#777269`
- Warm red accent: `#e65443`
- Muted blue accent: `#4d82d5`
- Soft green accent: `#35ae7a`
- Chroma key used only during asset generation: `#00ff00`

## Composition

- Work inside `(160,130)` to `(1760,950)` on a 1920x1080 canvas.
- Use two or more focal points per scene while leaving at least 55% of the frame visually empty.
- Generate isolated transparent-ready elements, never a complete paper background.
- Keep line weight visually equivalent to 3-6 pixels at final resolution.
- Use muted fills on no more than 10% of an element; preserve dominant cream-paper interiors and graphite outlines.
- Require complete full-body figures by default. Show the entire head, face, hair or ears, torso, both hands, both feet, and tail when applicable; surround every extremity with at least 8% blank chroma padding.
- Keep each generated character upright and readable at small size. Avoid thin disconnected details that can disappear during chroma removal.
- Keep at least 80 pixels of breathing room between unrelated element rectangles.
- Use only a subtle deterministic paper grain; never add stains or texture strong enough to compete with the drawing.

## Motion

- Bind the marker tip to the active reveal path.
- Draw the long outer silhouette first, then major internal contours, facial features, hands and feet, texture marks, and finally small color accents. Within each group, work top-to-bottom and left-to-right.
- Complete every stroke and switch to the full permit mask within the first 80% of its beat. Hold the unchanged, fully drawn element for the final 20%; never leave eyes, hands, feet, props, or small contour fragments for the cue-boundary frame.
- Use linear motion along individual strokes. Lift the marker and move without revealing pixels between disconnected strokes.
- Reveal graphite and colored marks around their own contours; never sweep the opaque cream interior in horizontal bands.
- Hold completed drawings. Do not pulse, float, zoom, or animate them after completion.
- Use a left-to-right 0.25-second ivory paper wipe between semantic scenes.

## Do not

- Do not render captions, labels, logos, watermarks, UI panels, gradients, photorealistic backgrounds, or generic stock icons.
- Do not use dense shading, heavy fills, crowded scenery, thick black masses, glossy digital-cartoon rendering, realistic fur, overlapping characters, cropped anatomy, incomplete limbs, or low-contrast beige linework.
- Do not reveal an entire element with opacity, scale, or a rectangular wipe.
- Do not allow future elements to ghost through or later strokes to repaint protected regions.
- Do not generate the hand per scene; use `assets/hand-marker.png`.
