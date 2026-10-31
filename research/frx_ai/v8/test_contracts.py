"""Contracts of the v8 tools. They guard behaviour; they do not measure language."""
import json
import re
import subprocess
import sys
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import detector  # noqa: E402
import investigator as inv  # noqa: E402
import joint  # noqa: E402
import logs  # noqa: E402
import score  # noqa: E402
import simulate  # noqa: E402

CODE = [p for p in HERE.glob("*.py") if p.name != "test_contracts.py"]


class Scope(unittest.TestCase):
    def test_nothing_is_trained(self):
        for p in CODE:
            source = p.read_text()
            self.assertIsNone(re.search(r"\.fit\(|\bfit_transform\(|\.backward\(|optimizer|from sklearn|import sklearn|import torch", source), p.name)

    def test_no_shell_or_network_from_decision_code(self):
        for name in ("detector.py", "joint.py", "investigator.py", "logs.py", "trajectory.py", "frx_investigate.py"):
            source = (HERE / name).read_text()
            self.assertIsNone(re.search(r"(?m)^\s*(?:import|from)\s+(?:subprocess|socket|urllib|requests|http)\b|os\.system", source), name)

    def test_no_app_steam_or_prefix_writes(self):
        for p in CODE:
            source = p.read_text()
            self.assertNotIn("src-tauri", source, p.name)
            self.assertIsNone(re.search(r"(?i)steamapps[^\n]*(?:write|open\([^)]*['\"]w)", source), p.name)


class Outcome(unittest.TestCase):
    def overall(self, text):
        return detector.detect(text)["overall"]

    def test_state_is_not_an_outcome(self):
        self.assertEqual(self.overall("The game crashes on launch."), "no_claim")
        self.assertEqual(self.overall("Works out of the box."), "no_claim")

    def test_classes(self):
        self.assertEqual(self.overall("Setting OPTION_X=1 fixed the crash."), "resolved")
        self.assertEqual(self.overall("I tried Proton 9 and got the same black screen."), "not_resolved")
        self.assertEqual(self.overall("Downgrading helped a bit."), "improved")
        self.assertEqual(self.overall("The update made it much worse."), "worsened")

    def test_negation_beats_the_positive_word(self):
        self.assertEqual(self.overall("Enabling it didn't help at all."), "not_resolved")
        self.assertEqual(self.overall("Tried that, it did not fix anything."), "not_resolved")

    def test_hedges_questions_and_advice_are_not_claims(self):
        self.assertEqual(self.overall("This might fix it."), "no_claim")
        self.assertEqual(self.overall("Did that fix it for you?"), "no_claim")
        self.assertEqual(self.overall("Try adding OPTION_X=1 to your launch options."), "no_claim")

    def test_quotes_code_and_template_are_not_the_author(self):
        self.assertEqual(self.overall("> Setting this fixed it\nThanks for the info."), "no_claim")
        self.assertEqual(self.overall("```\nfixed\n```"), "no_claim")

    def test_evidence_is_literal(self):
        text = "Downgrading the driver fixed the crash, and the game runs now."
        for claim in detector.detect(text)["claims"]:
            self.assertEqual(text[slice(*claim["span"])], claim["quote"])

    def test_no_causality(self):
        self.assertFalse(detector.detect("It fixed it.")["causality_confirmed"])


class Procedures(unittest.TestCase):
    def test_launch_line_is_a_group(self):
        text = "`DXVK_ASYNC=1 PROTON_NO_FSYNC=1 %command%` works now."
        relation, items = joint.unit_relation(text)
        self.assertEqual(relation, "joint")
        self.assertEqual(items[0]["kind"], "launch-line")

    def test_alternatives(self):
        relation, items = joint.unit_relation("I tried Proton 8, 9 and Experimental, none of them worked.")
        self.assertEqual(relation, "alternatives")
        self.assertGreaterEqual(len(items), 2)

    def test_sequence(self):
        relation, _ = joint.unit_relation("First I verified the files, then I deleted the prefix and it works now.")
        self.assertEqual(relation, "sequence")

    def test_group_never_credits_one_member(self):
        for proc in joint.procedures("`A_VAR=1 B_VAR=1 %command%` fixed it."):
            self.assertEqual(proc["individual_cause"], "unknown")
            self.assertNotEqual(proc["attribution"], "single-reported")


class Logs(unittest.TestCase):
    def test_private_paths_are_refused(self):
        for path in ("/home/example/.steam/steam/logs/x.log", "/etc/passwd", str(logs.FIXTURES / ".." / ".." / "PROTOCOL.md")):
            with self.assertRaises(PermissionError):
                logs.safe_path(path)

    def test_cli_refuses_outside_fixtures(self):
        out = subprocess.run([sys.executable, "-B", str(HERE / "frx_investigate.py"), "--problem", "x crashes", "--fixture", "../../../../../../.steam/x.log"], capture_output=True, text=True)
        self.assertNotEqual(out.returncode, 0)
        self.assertIn("recusado", out.stderr)

    def test_redaction(self):
        text = logs.redact(logs.read_fixture("private-data.log"))
        for secret in ("alice", "192.168.0.42", "76561198000000000", "example.com"):
            self.assertNotIn(secret, text)

    def test_quotes_are_redacted(self):
        reading = logs.read_log(logs.read_fixture("private-data.log"))
        self.assertNotIn("alice", json.dumps(reading))

    def test_expected_signals_and_no_cause(self):
        expected = json.loads((logs.FIXTURES / "expected.json").read_text())
        for name, want in expected.items():
            reading = logs.read_log(logs.read_fixture(name))
            self.assertEqual(sorted({s["kind"] for s in reading["signals"]}), sorted(want), name)
            self.assertIsNone(reading["confirmed_cause"])

    def test_noise_alone_does_not_conclude(self):
        reading = logs.read_log(logs.read_fixture("noise-only.log"))
        self.assertEqual(reading["signals"], [])
        self.assertTrue(any("não significa" in x for x in reading["limits"]))


class Investigation(unittest.TestCase):
    def case(self, *turns):
        return dict(game="G", problem=turns[0][1], turns=[dict(role=r, text=t) for r, t in turns], context={})

    def test_attached_log_is_read_before_asking_again(self):
        state = inv.state_of(self.case(("reporter", "It crashes. [steam-10.log](https://x.invalid/steam-10.log)")))
        self.assertEqual(inv.discriminate(state)[0]["id"], "read-attached-log")

    def test_missing_log_is_requested(self):
        state = inv.state_of(self.case(("reporter", "The game crashes on start.")))
        self.assertEqual(inv.discriminate(state)[0]["id"], "collect-proton-log")

    def test_done_is_not_requested_again(self):
        state = inv.state_of(self.case(("reporter", "It crashes. I tried Proton 9 and Experimental, I deleted the prefix and verified the files.")))
        ids = [c["id"] for c in inv.discriminate(state)]
        for check in ("test-other-proton", "reset-prefix-with-backup", "verify-game-files"):
            self.assertNotIn(check, ids)

    def test_a_group_result_does_not_support_one_member(self):
        state = inv.state_of(self.case(("reporter", "`PROTON_NO_FSYNC=1 DXVK_ASYNC=1 %command%` fixed it.")))
        self.assertTrue(all(r["attribution"] != "single-reported" for r in state["results"]))
        self.assertEqual(state["confirmed_causes"], [])

    def test_maintainer_request_is_not_a_result(self):
        state = inv.state_of(self.case(("reporter", "It crashes."), ("maintainer", "Setting X fixed it for others, please try it.")))
        self.assertEqual(state["results"], [])

    def test_reports_origin_and_never_authorizes(self):
        out = inv.report(self.case(("reporter", "It crashes.")))
        self.assertEqual(out["capability_origin"]["learned"], "nenhuma")
        self.assertFalse(out["execution_authorized"] or out["commands_executed"] or out["automatic_approval"])
        self.assertEqual(out["confirmed_causes"], [])

    def test_changing_checks_come_after_reading(self):
        log = logs.read_fixture("missing-dll.log")
        state = inv.state_of(dict(self.case(("reporter", "It crashes.")), log=log))
        self.assertFalse(inv.discriminate(state)[0]["changes_state"])

    def test_loop_does_not_repeat_itself(self):
        self.assertEqual(simulate.summary(simulate.run("discriminate"))["repeated_questions"], 0)
        self.assertEqual(simulate.summary(simulate.run("discriminate"))["confirmed_causes"], 0)


class Scoring(unittest.TestCase):
    def test_false_positive_counted_at_unit_level(self):
        gold = [dict(overall="no_claim"), dict(overall="resolved")]
        m = score.outcome_metrics(gold, ["resolved", "resolved"])
        self.assertEqual(m["false_positive"], 1)
        self.assertEqual(score.fine_metrics(gold, ["resolved", "resolved"])["false_resolved"], 1)

    def test_v7_positive_maps_to_merged_class(self):
        gold = [dict(overall="improved")]
        self.assertEqual(score.outcome_metrics(gold, ["positive"])["merged_class_correct"], [1, 1])

    def test_group_needs_relation_and_members(self):
        text = "A B"
        gold = dict(relation="joint", actions=["A", "B"])
        items = [dict(span=[0, 1]), dict(span=[2, 3])]
        self.assertTrue(score.group_correct(text, gold, "joint", items))
        self.assertFalse(score.group_correct(text, gold, "sequence", items))
        self.assertFalse(score.group_correct(text, gold, "joint", items[:1]))


if __name__ == "__main__":
    unittest.main()
