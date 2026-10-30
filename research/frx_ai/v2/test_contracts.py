import pathlib
import sys
import unittest
sys.path.insert(0,str(pathlib.Path(__file__).parent))
from contracts import check_evidence,checked_span,graph
from proposals import propose
from pairs import result_proposals,training_pairs,reduce_predictions
import numpy as np
import tempfile,json,hashlib


class Contracts(unittest.TestCase):
    def row(self):
        text="I disabled A and installed B. The crashes stopped, but performance got worse."
        return dict(app_id="200260",url="https://github.com/ValveSoftware/Proton/issues/309#issuecomment-1",
            text=text,action_span=[2,28],result_span=[30,len(text)],outcome="mixed",direction="multiple",
            modality="tested",joint=True)

    def test_game_isolation(self):
        self.assertEqual(check_evidence({'app_id':'200260'},dict(self.row(),app_id='123')),'different-or-unattributed-game')

    def test_explicit_context_isolation(self):
        self.assertEqual(check_evidence({'app_id':'200260','gpu':'AMD'},dict(self.row(),context={'gpu':'NVIDIA'})),'different-known-gpu')

    def test_unknown_hardware_stays_unknown(self):
        self.assertIsNone(check_evidence({'app_id':'200260'},self.row()))

    def test_unicode_offsets(self):
        text='Olá 🎮 — disabled X'
        a=text.index('disabled')
        self.assertEqual(checked_span(text,[a,len(text)])['text'],'disabled X')

    def test_invalid_offsets(self):
        for span in [[-1,2],[1,999],[True,2],[2,1]]:
            with self.assertRaises(ValueError):checked_span('abc',span)

    def test_synthetic_rejected(self):
        self.assertEqual(check_evidence({'app_id':'200260'},dict(self.row(),origin='synthetic-training-only')),'synthetic-input-not-real-evidence')

    def test_urls_are_data_not_commands(self):
        for url in ['javascript:alert(1)','https://github.com.evil.test/a/b/issues/1','https://user@github.com/a/b/issues/1']:
            self.assertIsNotNone(check_evidence({'app_id':'200260'},dict(self.row(),url=url)))

    def test_no_individual_cause_from_bundle(self):
        r=self.row();g=graph(r,'mixed','multiple','tested',True,.999,r['result_span'])
        self.assertEqual(g['combination']['individual_cause'],'unknown')
        self.assertFalse(g['accepted_automatically']);self.assertFalse(g['execution_authorized'])

    def test_quotes_are_not_authored_action_proposals(self):
        text='> Enabling OTHER fixed it.\nTry enabling X.'
        self.assertTrue(all(text[a:b].startswith('enabling') for a,b in propose(text)))
        self.assertTrue(all(a>text.index('\n') for a,b in propose(text)))

    def test_inference_edge_selection_never_reads_gold(self):
        logits=[np.array([[0.,3.,0.,0.,0.],[4.,0.,0.,0.,0.]]),
                np.ones((2,10)),np.array([[0.,0.,4.],[0.,0.,4.]]),np.ones((2,2))]
        _,selected=reduce_predictions(logits,[(0,2)])
        self.assertEqual(selected,[0])

    def test_unattributed_action_result_is_not_positive_training_gold(self):
        row=dict(self.row(),outcome='unknown',result_span=None)
        self.assertTrue(all(x['outcome']=='unknown' for x in training_pairs([row])))

    def test_budget_excludes_annotation_queue(self):
        from budget import spent
        with tempfile.TemporaryDirectory() as tmp:
            root=pathlib.Path(tmp);(root/'active').mkdir()
            (root/'active/run.json').write_text(json.dumps({'seconds':12.5}))
            (root/'active/review.json').write_text(json.dumps([{'id':'fixture'}]))
            self.assertEqual(spent(root),12.5)

    def test_cached_evidence_preserves_crlf_offsets(self):
        from data import hydrate
        with tempfile.TemporaryDirectory() as tmp:
            root=pathlib.Path(tmp);(root/'bodies').mkdir();text='I enabled X.\r\nNo result yet.'
            sha=hashlib.sha256(text.encode()).hexdigest();(root/'bodies'/(sha+'.txt')).write_bytes(text.encode())
            row=dict(self.row(),text_sha256=sha,action_span=[2,11],result_span=None,
                outcome='unknown',split='train',added_stage=1,origin='synthetic-test-fixture')
            row.pop('text');ann=root/'annotations.json';ann.write_text(json.dumps([row]));hydrate(ann,root)
            restored=json.loads((root/'train-events.jsonl').read_text())
            self.assertEqual(restored['text'],text)


if __name__=='__main__':unittest.main()
