"""Contracts of the pilot tools. Annotators here are made up: nothing measures language."""
import json
import os
import re
import tempfile
import unittest
from pathlib import Path
from unittest import mock

import pilot


def fake_case(i):
    return dict(id=f"pilot-{i:02d}", title="Game (1)", url="https://example.invalid", text=f"Case {i}: it works now after I set OPTION_{i}.")


def fake_row(i, name, overall="resolved", relation="single", role="first_hand"):
    return dict(id=f"pilot-{i:02d}", annotator=name, role=role, overall=overall, relation=relation, evidence=["it works now"],
                actions=[f"OPTION_{i}"], assistance="none")


class Alpha(unittest.TestCase):
    def test_published_example_two_observers(self):
        a = [0, 1, 0, 0, 0, 0, 0, 0, 1, 0]
        b = [1, 1, 1, 0, 0, 1, 0, 0, 0, 0]
        self.assertAlmostEqual(pilot.krippendorff_nominal(list(map(list, zip(a, b)))), 0.095, places=3)

    def test_published_example_with_missing_values(self):
        n = None
        a = [n, n, n, n, n, 3, 4, 1, 2, 1, 1, 3, 3, n, 3]
        b = [1, n, 2, 1, 3, 3, 4, 3, n, n, n, n, n, n, n]
        c = [n, n, 2, 1, 3, 4, 4, n, 2, 1, 1, 3, 3, n, 4]
        self.assertAlmostEqual(pilot.krippendorff_nominal(list(map(list, zip(a, b, c)))), 0.691, places=3)

    def test_perfect_and_degenerate(self):
        self.assertEqual(pilot.krippendorff_nominal([["x", "x"], ["y", "y"]]), 1.0)
        self.assertIsNone(pilot.krippendorff_nominal([["x", None]]))

    def test_thresholds(self):
        self.assertEqual(pilot.verdict(.8), "acceptable")
        self.assertEqual(pilot.verdict(.7), "tentative")
        self.assertTrue(pilot.verdict(.5).startswith("below"))

    def test_span_overlap(self):
        self.assertEqual(pilot.span_f1([(0, 4)], [(0, 4)]), 1.0)
        self.assertEqual(pilot.span_f1([(0, 4)], [(4, 8)]), 0.0)
        self.assertIsNone(pilot.span_f1([], []))


class Flow(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.root = Path(self.tmp.name)
        (self.root / "packet").mkdir()
        (self.root / "annotations").mkdir()
        pilot.write_rows(self.root / "packet/cases.jsonl", [fake_case(i) for i in range(4)])

    def tearDown(self):
        self.tmp.cleanup()

    def people(self, a=("human", True), b=("human", True)):
        (self.root / "annotators.json").write_text(json.dumps({
            "A": dict(id="p1", kind=a[0], independent_of_frx=a[1]), "B": dict(id="p2", kind=b[0], independent_of_frx=b[1])}))

    def files(self, second=None):
        pilot.write_rows(self.root / "annotations/A.jsonl", [fake_row(i, "A") for i in range(4)])
        pilot.write_rows(self.root / "annotations/B.jsonl", second or [fake_row(i, "B") for i in range(4)])

    def test_literal_evidence_is_enforced(self):
        bad = fake_row(0, "A")
        bad["evidence"] = ["not in the text"]
        pilot.write_rows(self.root / "annotations/A.jsonl", [bad])
        pilot.write_rows(self.root / "annotations/B.jsonl", [fake_row(0, "B")])
        with self.assertRaises(SystemExit):
            pilot.agree(self.root)

    def test_declared_assistance_is_rejected(self):
        row = fake_row(0, "A")
        row["assistance"] = "declared"
        pilot.write_rows(self.root / "annotations/A.jsonl", [row])
        pilot.write_rows(self.root / "annotations/B.jsonl", [fake_row(0, "B")])
        with self.assertRaises(SystemExit):
            pilot.agree(self.root)

    def test_claim_of_independence_needs_two_independent_people(self):
        self.files()
        self.people()
        self.assertEqual(pilot.agree(self.root)["claim"], "independent")
        self.people(b=("human", False))
        self.assertEqual(pilot.agree(self.root)["review_level"], "H1")
        self.people(("agent", True), ("agent", True))
        self.assertEqual(pilot.agree(self.root)["review_level"], "H0")

    def test_no_registry_is_never_independent(self):
        self.files()
        self.assertEqual(pilot.agree(self.root)["review_level"], "H0")

    def test_disagreement_blocks_gold_until_adjudicated(self):
        second = [fake_row(i, "B") for i in range(4)]
        second[1] = fake_row(1, "B", overall="not_resolved")
        self.files(second)
        with self.assertRaises(SystemExit):
            pilot.adjudicate(self.root)
        self.assertFalse((self.root / "gold/pilot-gold.jsonl").exists())
        decision = self.root / "decision.jsonl"
        pilot.write_rows(decision, [fake_row(1, "C")])
        pilot.adjudicate(self.root, decisions=decision)
        self.assertEqual(oct((self.root / "gold/pilot-gold.jsonl").stat().st_mode)[-3:], "600")
        with self.assertRaises(SystemExit):
            pilot.adjudicate(self.root, decisions=decision)

    def test_predictions_lock_only_before_gold(self):
        prediction = self.root / "p.jsonl"
        prediction.write_text("{}\n")
        pilot.lock_predictions(self.root, "rules", prediction)
        with self.assertRaises(SystemExit):
            pilot.lock_predictions(self.root, "rules", prediction)
        self.files()
        pilot.adjudicate(self.root)
        with self.assertRaises(SystemExit):
            pilot.lock_predictions(self.root, "other", prediction)

    def test_annotation_refuses_agent_session(self):
        with mock.patch.dict(os.environ, {"CLAUDECODE": "1"}), self.assertRaises(SystemExit):
            pilot.annotate_cli(self.root, "A")

    def test_packet_and_tool_stay_blind(self):
        source = Path(pilot.__file__).read_text()
        self.assertIsNone(re.search(r"(?m)^(?:from|import)\s+(?:sklearn|torch|onnxruntime|transformers|procedures|detector)\b", source))
        pilot_fields = set(fake_case(0))
        self.assertEqual(pilot_fields, {"id", "title", "url", "text"})


if __name__ == "__main__":
    unittest.main()
