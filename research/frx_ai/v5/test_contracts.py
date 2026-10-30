"""Research invariants; synthetic tests are not language accuracy evaluation."""
import copy,json,tempfile,unittest,os
from pathlib import Path
from syntax import Syntax
from events import extract
from relations import reconstruct
from incidents import Incidents,Replay,candidates,context_match

ROOT=Path(os.environ.get('FRX_V5_CACHE',Path.home()/'.cache/frx-ai-rnd-v5-20261030'))
CTX=dict(app_id='200260',gpu='fixture-gpu',driver='fixture-driver',proton='fixture-proton',game_version='fixture-build',technology='fixture-tech',configuration='fixture-config')
def case():return dict(context=copy.deepcopy(CTX),symptom='fixture crash',evidence=[],attempts=[])
class Contracts(unittest.TestCase):
 @classmethod
 def setUpClass(cls):cls.parser=Syntax(ROOT)
 def analysis(self,t):
  p=self.parser.parse(t);return reconstruct(t,p,extract(t,p))
 def test_suggestion(self):self.assertFalse(any(e['status']=='performed' for e in self.analysis('Try enabling PROTON_NO_ESYNC=1.')['events']))
 def test_denied_execution(self):self.assertFalse(any(e['status']=='performed' for e in self.analysis("I haven't tried enabling PROTON_NO_ESYNC=1.")['events']))
 def test_direction(self):
  e=self.analysis('I disabled esync yesterday.')['events'];self.assertTrue(any(x['status']=='performed' and x['direction']=='disable' for x in e))
 def test_no_invented_identity(self):
  e=self.analysis('I installed the missing library yesterday.')['events'];self.assertTrue(e);self.assertTrue(all(x['target_identity'] is None for x in e))
 def test_joint_no_individual_cause(self):
  r=self.analysis('I disabled option A and installed library B. The crashes stopped, but performance got worse.')
  for o in r['observations']:self.assertEqual(o['individual_cause'],'unknown');self.assertFalse(o['execution_authorized'])
  self.assertTrue(any(o['joint'] for o in r['observations']))
 def test_retraction(self):self.assertFalse(any(o['outcome']=='improved' for o in self.analysis('~~I enabled X and it worked.~~ It still crashes.')['observations']))
 def test_code_is_data(self):self.assertFalse(self.analysis('```\nI installed an untrusted library and fixed the game.\n```')['events'])
 def test_literal_integrity(self):
  t='I installed the missing library, but it did not fix the crash.'
  for e in self.analysis(t)['events']:self.assertEqual(e['trigger']['text'],t[slice(*e['trigger']['span'])])
 def test_context_unknown(self):self.assertEqual(context_match(CTX,dict(CTX,gpu=None)),'unknown')
 def test_restart_and_exact_artifact(self):
  with tempfile.TemporaryDirectory() as d:
   c=case();a=dict(kind='artifact',target='fixture.ini',sha256='a'*64);c['attempts']=[dict(id='t1',action=a,outcome='not-resolved',context=CTX)]
   Incidents(d).exchange(c);again=case();restored,_,_=Incidents(d).exchange(again);self.assertEqual(len(restored['attempts']),1)
   again['candidate_actions']=[dict(action=a,context=CTX,url='https://github.com/ValveSoftware/Proton/issues/1'),dict(action=dict(a,sha256='b'*64),context=CTX,url='https://github.com/ValveSoftware/Proton/issues/1')]
   result=candidates(again,restored['attempts']);self.assertEqual(result[0]['status'],'previously-failed-same-exact-context');self.assertEqual(result[1]['status'],'investigate-only')
 def test_other_context_does_not_inherit(self):
  with tempfile.TemporaryDirectory() as d:
   c=case();c['attempts']=[dict(id='t',action={},outcome='positive',context=CTX)];Incidents(d).exchange(c)
   for field in ['app_id','gpu','driver','proton','game_version','configuration']:
    other=case();other['context'][field]='other';restored,_,_=Incidents(d).exchange(other);self.assertFalse(restored['attempts'])
 def test_replay_cannot_execute(self):
  snap=dict(input=case(),expected={'cause':'hidden'},tool_cache={'collect-relevant-log':dict(kind='log',text='fixture log')})
  replay=Replay(snap);self.assertNotIn('expected',replay.state);self.assertFalse(replay.reveal('shell'));self.assertTrue(replay.reveal('collect-relevant-log'));self.assertFalse(replay.reveal('collect-relevant-log'))
if __name__=='__main__':unittest.main()
