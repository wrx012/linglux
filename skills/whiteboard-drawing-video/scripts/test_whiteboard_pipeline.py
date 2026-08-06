# /// script
# requires-python = ">=3.11"
# dependencies = [
#   "numpy==2.3.2",
#   "opencv-python-headless==4.11.0.86",
#   "pillow==10.4.0",
# ]
# ///
import importlib.util
import json
import sys
import tempfile
import unittest
from pathlib import Path

from PIL import Image, ImageDraw


MODULE_PATH = Path(__file__).with_name("whiteboard_pipeline.py")
SPEC = importlib.util.spec_from_file_location("whiteboard_pipeline", MODULE_PATH)
pipeline = importlib.util.module_from_spec(SPEC)
assert SPEC.loader
sys.modules[SPEC.name] = pipeline
SPEC.loader.exec_module(pipeline)


class SubtitleTests(unittest.TestCase):
    def write(self, directory: Path, name: str, content: str) -> Path:
        path = directory / name
        path.write_text(content, encoding="utf-8")
        return path

    def test_valid_srt_quantizes_frames(self):
        with tempfile.TemporaryDirectory() as raw:
            path = self.write(Path(raw), "demo.srt", "1\n00:00:00,017 --> 00:00:01,001\n你好\n\n2\n00:00:01,200 --> 00:00:02,000\n世界\n")
            cues = pipeline.parse_srt(path)
            self.assertEqual((cues[0].start_frame, cues[0].end_frame), (1, 31))
            self.assertEqual(cues[-1].end_frame, 60)

    def test_indexless_srt_blocks_receive_consecutive_indices(self):
        with tempfile.TemporaryDirectory() as raw:
            path = self.write(
                Path(raw), "demo.srt",
                "00:00:00,000 --> 00:00:01,000\n第一句\n\n00:00:01,000 --> 00:00:02,000\n第二句\n",
            )
            cues = pipeline.parse_srt(path)
            self.assertEqual([cue.index for cue in cues], [1, 2])

    def test_wrong_extension_is_rejected(self):
        with tempfile.TemporaryDirectory() as raw:
            path = self.write(Path(raw), "demo.crt", "1\n00:00:00,000 --> 00:00:01,000\nx\n")
            with self.assertRaisesRegex(pipeline.PipelineError, "\\.srt"):
                pipeline.parse_srt(path)

    def test_generated_artifacts_share_one_export_directory(self):
        source = Path("/tmp/example.srt")
        parent = source.resolve().parent
        export = pipeline.export_dir_for(source)
        self.assertEqual(export, parent / "example-whiteboard")
        self.assertEqual(pipeline.work_dir_for(source), export / "work")
        self.assertEqual(pipeline.output_for(source), export / "output.mp4")

    def test_overlap_is_rejected(self):
        with tempfile.TemporaryDirectory() as raw:
            path = self.write(Path(raw), "demo.srt", "1\n00:00:00,000 --> 00:00:02,000\na\n\n2\n00:00:01,900 --> 00:00:03,000\nb\n")
            with self.assertRaisesRegex(pipeline.PipelineError, "overlap"):
                pipeline.parse_srt(path)

    def test_malformed_empty_and_nonconsecutive_cues_are_rejected(self):
        cases = {
            "empty": "",
            "malformed": "1\n00:00:bogus --> 00:00:01,000\nx\n",
            "reversed": "1\n00:00:02,000 --> 00:00:01,000\nx\n",
            "nonconsecutive": "2\n00:00:00,000 --> 00:00:01,000\nx\n",
        }
        with tempfile.TemporaryDirectory() as raw:
            for name, content in cases.items():
                with self.subTest(name=name):
                    path = self.write(Path(raw), f"{name}.srt", content)
                    with self.assertRaises(pipeline.PipelineError):
                        pipeline.parse_srt(path)

    def test_permit_mask_excludes_future_and_protected(self):
        import numpy as np

        current = np.zeros((8, 8), dtype=np.uint8)
        current[2:6, 2:6] = 255
        future = np.zeros_like(current)
        future[2:4, 2:4] = 255
        protected = np.zeros_like(current)
        protected[4:6, 4:6] = 255
        allowed = pipeline.compute_permit_mask(current, future, protected, 0)
        self.assertFalse(allowed[2:4, 2:4].any())
        self.assertFalse(allowed[4:6, 4:6].any())
        self.assertTrue(allowed[2:4, 4:6].all())

    def test_final_reveal_uses_complete_permit_mask(self):
        import numpy as np

        with tempfile.TemporaryDirectory() as raw:
            permit = np.zeros((pipeline.HEIGHT, pipeline.WIDTH), dtype=np.uint8)
            permit[20:80, 30:90] = 255
            permit_path = Path(raw) / "permit.png"
            Image.fromarray(permit, "L").save(permit_path)
            reveal, tip = pipeline.reveal_for_progress(
                {"permit_path": permit_path, "strokes": [[(30, 20), (89, 79)]], "brush_width": 8}, 1.0
            )
            self.assertTrue(np.array_equal(reveal, permit))
            self.assertEqual(tip, (89, 79))

    def test_drawing_finishes_early_and_holds_complete_result(self):
        complete_frame, hold_frames = pipeline.draw_timing(0, 100)
        self.assertEqual(complete_frame, 79)
        self.assertEqual(hold_frames, 20)
        self.assertEqual(pipeline.drawing_progress(39, 0, 100), 0.5)
        self.assertEqual(pipeline.drawing_progress(79, 0, 100), 1.0)
        self.assertEqual(pipeline.drawing_progress(99, 0, 100), 1.0)

        one_frame_complete, one_frame_hold = pipeline.draw_timing(10, 11)
        self.assertEqual((one_frame_complete, one_frame_hold), (10, 0))

    def test_disconnected_strokes_do_not_draw_pen_up_travel(self):
        import numpy as np

        with tempfile.TemporaryDirectory() as raw:
            permit = np.full((pipeline.HEIGHT, pipeline.WIDTH), 255, dtype=np.uint8)
            permit_path = Path(raw) / "permit.png"
            Image.fromarray(permit, "L").save(permit_path)
            beat = {
                "permit_path": permit_path,
                "strokes": [[(20, 20), (60, 20)], [(300, 200), (340, 200)]],
                "brush_width": 8,
            }
            reveal, _ = pipeline.reveal_for_progress(beat, 0.75)
            self.assertTrue(reveal[20, 20:61].any())
            self.assertTrue(reveal[200, 300].any())
            self.assertFalse(reveal[110, 180])

    def test_drawable_stroke_mask_excludes_light_interior(self):
        import numpy as np

        canvas = Image.new("RGBA", (pipeline.WIDTH, pipeline.HEIGHT), (0, 0, 0, 0))
        draw = ImageDraw.Draw(canvas)
        draw.rectangle((100, 100, 300, 300), fill=(247, 242, 225, 255), outline=(85, 82, 75, 255), width=6)
        permit = np.asarray(canvas.getchannel("A"))
        marks = pipeline.drawable_stroke_mask(canvas, permit)
        self.assertTrue(marks[100, 150])
        self.assertFalse(marks[200, 200])

    def test_alpha_canvas_rejects_edge_touching_and_preserves_aspect_ratio(self):
        with tempfile.TemporaryDirectory() as raw:
            work = Path(raw)
            (work / "assets").mkdir()
            touching = Image.new("RGBA", (200, 100), (0, 0, 0, 0))
            ImageDraw.Draw(touching).rectangle((0, 20, 120, 80), fill=(52, 49, 45, 255))
            touching.save(work / "assets" / "touching.png")
            beat = {"id": "beat-001", "asset": "assets/touching.png", "x": 100, "y": 100, "width": 400, "height": 200}
            with self.assertRaisesRegex(pipeline.PipelineError, "cropped"):
                pipeline.alpha_canvas(work, beat)

            padded = Image.new("RGBA", (200, 100), (0, 0, 0, 0))
            ImageDraw.Draw(padded).rectangle((20, 20, 180, 80), fill=(52, 49, 45, 255))
            padded.save(work / "assets" / "padded.png")
            beat.update({"asset": "assets/padded.png", "width": 400, "height": 300})
            canvas, alpha = pipeline.alpha_canvas(work, beat)
            self.assertEqual(canvas.size, (pipeline.WIDTH, pipeline.HEIGHT))
            left, top, right, bottom = pipeline.alpha_bounds(alpha)
            self.assertAlmostEqual((right - left) / (bottom - top), 161 / 61, delta=0.08)

    def test_build_creates_hyperframes_and_report(self):
        with tempfile.TemporaryDirectory() as raw:
            directory = Path(raw)
            path = self.write(
                directory,
                "demo.srt",
                "1\n00:00:00,000 --> 00:00:01,000\n第一幕\n\n"
                "2\n00:00:01,200 --> 00:00:02,000\n继续画\n",
            )
            pipeline.prepare(path)
            work = pipeline.work_dir_for(path)
            for number in (1, 2):
                image = Image.new("RGBA", (240, 180), (0, 0, 0, 0))
                draw = ImageDraw.Draw(image)
                draw.ellipse((20, 20, 220, 160), outline=(52, 49, 45, 255), width=8)
                image.save(work / "assets" / f"beat-{number:03d}.png")
            storyboard = {
                "version": 1,
                "scenes": [
                    {
                        "id": "scene-001",
                        "cue_indices": [1, 2],
                        "summary": "test",
                        "beats": [
                            {
                                "id": f"beat-{number:03d}",
                                "cue_indices": [number],
                                "prompt": "test shape",
                                "asset_chroma": f"assets/beat-{number:03d}-chroma.png",
                                "asset": f"assets/beat-{number:03d}.png",
                                "x": 180 + (number - 1) * 500,
                                "y": 220,
                                "width": 400,
                                "height": 300,
                            }
                            for number in (1, 2)
                        ],
                    }
                ],
            }
            (work / "storyboard.json").write_text(json.dumps(storyboard), encoding="utf-8")
            contact_sheet = pipeline.style_check(path)
            self.assertTrue(contact_sheet.is_file())
            self.assertTrue((work / "qa" / "style-metrics.json").is_file())
            hyperframes = pipeline.build(path)
            self.assertTrue((hyperframes / "index.html").is_file())
            self.assertTrue((hyperframes / "compositions" / "scene-001.html").is_file())
            report = json.loads((work / "render-report.json").read_text(encoding="utf-8"))
            self.assertEqual(report["total_frames"], 60)
            self.assertLessEqual(report["scenes"][0]["coverage"], 0.45)
            beats = report["scenes"][0]["beats"]
            self.assertEqual(beats[0]["draw_complete_frame"], 23)
            self.assertEqual(beats[0]["hold_frames"], 6)
            self.assertLess(beats[0]["draw_complete_frame"], beats[0]["end_frame"] - 1)


if __name__ == "__main__":
    unittest.main()
