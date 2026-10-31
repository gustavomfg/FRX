"""Contracts of the v7 lab. They protect boundaries; they do not measure language skill."""
import ast
import json
import sys
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import investigate as I  # noqa: E402
from knowledge import CHECKS, FORBIDDEN_WORDS, HYPOTHESES  # noqa: E402
from signals import log_signals, symptoms  # noqa: E402


def imports(path):
    names = set()
    for node in ast.walk(ast.parse(path.read_text())):
        if isinstance(node, ast.Import):
            names |= {a.name.split(".")[0] for a in node.names}
        elif isinstance(node, ast.ImportFrom) and node.module:
            names.add(node.module.split(".")[0])
    return names


class Boundaries(unittest.TestCase):
    def test_inference_never_imports_the_oracle_or_gold(self):
        for name in ["procedures", "investigate", "frx_lab", "retrieve", "signals", "knowledge", "l1_adapter"]:
            self.assertNotIn("oracle", imports(HERE / f"{name}.py"), name)

    def test_nothing_here_can_run_a_process_or_touch_the_network(self):
        for name in ["procedures", "investigate", "frx_lab", "retrieve", "signals", "knowledge"]:
            self.assertFalse(imports(HERE / f"{name}.py") & {"subprocess", "socket", "urllib", "requests", "shutil"}, name)
            self.assertNotIn("os.system", (HERE / f"{name}.py").read_text())

    def test_catalog_has_no_install_instruction_and_every_check_exists(self):
        for c in CHECKS.values():
            self.assertFalse(any(w in (c["what"] + c["how"]).lower() for w in FORBIDDEN_WORDS), c["what"])
        for h in HYPOTHESES.values():
            for c in h["checks"]:
                self.assertIn(c, CHECKS)


class Investigation(unittest.TestCase):
    def test_undefined_symbol_is_a_typed_signal_and_invents_no_library(self):
        r = I.analyse(dict(game="x", problem="The game does not start", log="undefined symbol: SDL_fabsf", context=dict(app_id="10")), "rules")
        self.assertEqual(r["signals"][0]["details"]["symbol"], "SDL_fabsf")
        self.assertEqual(r["next_check"]["id"], "identify-requiring-object")
        text = json.dumps(r["next_checks"]).lower()
        self.assertNotIn("libsdl2", text)
        self.assertEqual(r["confirmed_causes"], [])
        self.assertIsNone(r["cause_probability"])

    def test_hypothesis_never_becomes_a_confirmed_cause(self):
        case = dict(problem="crash", log="undefined symbol: foo", context=dict(app_id="10"), results=[dict(check="resolve-library-path", verdict="supports", finding="outra cópia")])
        r = I.analyse(case, "rules")
        self.assertTrue(any(h["status"] == "supported-by-reported-check" for h in r["hypotheses"]))
        self.assertEqual(r["confirmed_causes"], [])
        self.assertTrue(all(h["confirmed"] is False for h in r["hypotheses"]))

    def test_done_checks_are_not_repeated(self):
        first = I.analyse(dict(problem="crash", log="undefined symbol: foo", context=dict(app_id="10")), "rules")["next_check"]["id"]
        again = I.analyse(dict(problem="crash", log="undefined symbol: foo", context=dict(app_id="10"), results=[dict(check=first, verdict="inconclusive")]), "rules")
        self.assertNotIn(first, [c["id"] for c in again["next_checks"]])

    def test_physx_on_amd_keeps_the_contradiction(self):
        r = I.analyse(dict(problem="PhysX com desempenho ruim", context=dict(app_id="200260", gpu_vendor="amd")), "rules")
        chain = next(h for h in r["hypotheses"] if h["id"] == "gpu-physx-chain")
        self.assertTrue(chain["contradictions"])
        self.assertEqual(r["hypotheses"][0]["id"], "physx-unavailable-hardware")

    def test_no_evidence_stays_unknown(self):
        r = I.analyse(dict(problem="it is weird", context=dict(app_id="10")), "rules")
        self.assertEqual([h["id"] for h in r["hypotheses"]], ["unknown-cause"])
        self.assertIn(r["next_check"]["id"], ("collect-proton-log", "collect-system-info"))

    def test_history_of_another_environment_does_not_transfer(self):
        new = dict(problem="crash", context=dict(app_id="10", gpu_vendor="amd"), results=[])
        old = dict(problem="crash", context=dict(app_id="10", gpu_vendor="nvidia"), results=[dict(check="x", verdict="supports")])
        merged, rejected = I.merge_history(new, old)
        self.assertEqual(merged["results"], [])
        self.assertEqual(rejected[0]["relation"], "incompatible")

    def test_literal_quotes_really_come_from_the_log(self):
        log = "line\n0024:err:module:import_dll Library d3d11.dll (which is needed by L\"x.exe\") not found\n"
        s = log_signals(log)[0]
        self.assertEqual(log[slice(*s["span"])], s["quote"])
        self.assertEqual(s["details"]["library"], "d3d11.dll")


class PostTestFixes(unittest.TestCase):
    def test_template_video_driver_line_is_not_a_video_symptom(self):
        self.assertFalse([s for s in symptoms("- Video driver version: Mesa 25") if s["kind"] == "video"])

    def test_attached_log_is_not_requested_again_but_is_flagged_unread(self):
        r = I.analyse(dict(problem="It crashes [steam-1234.log](https://x/y)", context=dict(app_id="10")), "rules")
        self.assertNotIn("collect-proton-log", [c["id"] for c in r["next_checks"]])
        self.assertTrue(any("não o baixa" in u for u in r["uncertainty"]))


class Procedures(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        sys.path.insert(0, str(HERE.parent / "v6"))
        import bridge
        import events
        import procedures
        inherited = events.evidence
        events.evidence = lambda text, a, b: inherited(text, min(a, b), max(a, b))
        cache = Path.home() / ".cache/frx-ai-rnd-v7-20261030"
        cls.parser = bridge.CompactSyntax(cache)
        cls.catalog = json.loads((cache / "catalog.json").read_text())
        cls.analyze = staticmethod(procedures.analyze)

    def read(self, text):
        return self.analyze(dict(text=text, document_id="t", reporter_id="r"), self.parser, self.catalog, 3.4)

    def test_joint_report_gives_no_individual_cause(self):
        r = self.read("I changed Proton, disabled esync and installed a library. After that, the game launched.")
        for c in r["observations"]:
            self.assertEqual(c["individual_cause"], "unknown")
            self.assertFalse(c["causality_confirmed"])
            if len(c["event_indices"]) > 1:
                self.assertEqual(c["attribution"]["kind"], "joint-unattributed" if c["structure"] != "alternatives" else "distributive-attempts")

    def test_question_is_not_a_result(self):
        self.assertEqual(self.read("Does the game work for you on Proton 9?")["observations"], [])

    def test_no_output_authorizes_execution(self):
        r = self.read("Disabling esync fixed the crash for me.")
        self.assertFalse(r["execution_authorized"])
        self.assertFalse(r["automatic_approval"])


if __name__ == "__main__":
    unittest.main()
