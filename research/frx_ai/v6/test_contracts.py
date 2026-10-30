"""Synthetic invariants are contracts, not independent language evaluation."""
import unittest,json,os,copy,tempfile,subprocess,sys
from pathlib import Path
from bridge import CompactSyntax
from grounding import deconstruct,identity
from pairs import candidates,verify_literal
from investigate import Investigator
ROOT=Path(os.environ.get('FRX_V6_CACHE',Path.home()/'.cache/frx-ai-rnd-v6-20261030'));OLD=Path.home()/'.cache/frx-ai-rnd-v2-20261029'
CAT=[dict(app_id='10001',name='Game Alpha',verified=True,source='urn:synthetic-fixture',aliases=[]),dict(app_id='10002',name='Game Beta',verified=True,source='urn:synthetic-fixture',aliases=[])]
CTX=dict(app_id='10001',gpu='fixture-amd',gpu_vendor='AMD',driver='fixture-driver',proton='fixture-proton',game_version='fixture-build',technology='fixture-tech',configuration='fixture-config')
class Contracts(unittest.TestCase):
 @classmethod
 def setUpClass(cls):cls.parser=CompactSyntax(ROOT)
 def bundle(self,text):return deconstruct(dict(text=text,app_id='10001',document_id='fixture',reporter_id='fixture-author'),self.parser,CAT)
 def test_thread_not_event(self):self.assertTrue(all(e['identity']['game_app_id'] is None for e in self.bundle('I installed the missing library and the game worked.')['events']))
 def test_explicit_distinct_games(self):
  b=self.bundle('In Game Beta I installed the library and it worked, but in Game Alpha I disabled OPTION_X and it crashed.');ids={e['identity']['game_app_id'] for e in b['events']};self.assertEqual(ids,{'10001','10002'})
 def test_unknown_name_not_fuzzy_link(self):self.assertTrue(all(e['identity']['game_app_id'] is None for e in self.bundle('In Game Alphaa I installed the missing library.')['events']))
 def test_quote_not_reporter(self):self.assertTrue(all(e['identity']['actor_kind']=='quoted' and e['identity']['actor_id'] is None for e in self.bundle('> I installed the runtime and it worked yesterday.')['events']))
 def test_friend_not_reporter(self):self.assertEqual(self.bundle('My friend installed the missing library yesterday.')['events'][0]['identity']['actor_kind'],'third_party')
 def test_suggested(self):self.assertFalse(any(e['status']=='performed' for e in self.bundle('You should try enabling OPTION_X.')['events']))
 def test_told_not_executed(self):self.assertEqual(self.bundle('I was told to enable OPTION_X.')['events'][0]['status'],'suggested')
 def test_conditional(self):self.assertFalse(any(e['status']=='performed' for e in self.bundle('If I try OPTION_X, the game crashes.')['events']))
 def test_preexisting(self):self.assertFalse(any(e['status']=='performed' for e in self.bundle('I was playing for hours with fsync enabled on the current kernel.')['events'] if e['trigger']['text']=='enabled'))
 def test_passive_completed_change(self):self.assertEqual(self.bundle('After the runtime was installed, the game crashed.')['events'][0]['status'],'performed')
 def test_no_invented_library(self):self.assertIsNone(self.bundle('I installed the missing library yesterday.')['events'][0]['target_identity'])
 def test_no_next_action_target_theft(self):
  b=self.bundle('I tried a workaround, and then I enabled PROTON_NO_ESYNC=1.');self.assertIsNone(b['events'][0]['target_identity'])
 def test_purpose_not_observation(self):
  t='I installed the runtime to fix the crash.';b=self.bundle(t);ps=candidates(t,b);self.assertTrue(all(verify_literal(t,b,p)[0]!='candidate' for p in ps))
 def test_joint_no_individual_credit(self):
  t='I disabled OPTION_X and installed the runtime. Then the game worked.';b=self.bundle(t);ps=candidates(t,b);self.assertTrue(any(len(p['event_indices'])==2 for p in ps));self.assertTrue(all(verify_literal(t,b,p)[0]!='candidate' for p in ps if len(p['event_indices'])==1))
 def test_literal_spans(self):
  t='I installed the library, but the game still crashed.'
  for e in self.bundle(t)['events']:self.assertEqual(e['trigger']['text'],t[slice(*e['trigger']['span'])])
 def test_code_is_not_executed_evidence(self):self.assertFalse(self.bundle('```\nI started the game and installed the library.\n```')['events'])
 def test_result_first_requires_link(self):
  t='The game worked yesterday. I installed the runtime today.';b=self.bundle(t);self.assertTrue(all(verify_literal(t,b,p)[0]!='candidate' for p in candidates(t,b)))
 def test_version_context_not_merged(self):
  t='With Proton 7.0 I installed the runtime. But with Proton 6.0 the game still crashed.';b=self.bundle(t);self.assertTrue(all(verify_literal(t,b,p)[0]!='candidate' for p in candidates(t,b)))
 def test_other_actor_not_merged(self):
  t='My friend installed the runtime. But I still have the same problem.';b=self.bundle(t);self.assertTrue(all(verify_literal(t,b,p)[0]!='candidate' for p in candidates(t,b)))
 def test_postposed_speech_subject_spans(self):
  text='I installed the runtime, said my friend.'
  bundle=self.bundle(text)
  for event in bundle['events']:
   source=event['identity'].get('actor_evidence')
   if source:self.assertEqual(source['text'],text[slice(*source['span'])])
  # Explicit dependency fixture reaches the repaired branch regardless of
  # how the upstream parser attaches this particular English sentence.
  text='I installed it, my friend said.'
  words=[]
  for i,(token,lemma,head,rel) in enumerate([('I','I',1,'nsubj'),('installed','install',3,'ccomp'),('friend','friend',3,'nsubj'),('said','say',None,'root')]):
   a=text.index(token);words.append(dict(i=i,text=token,lemma=lemma,head=head,rel=rel,span=[a,a+len(token)],sentence=0))
  result=identity(text,dict(words=words,sentences=[dict(span=[0,len(text)])]),1,dict(document_id='fixture',reporter_id='fixture-author'),[])
  self.assertEqual(result['actor_kind'],'third_party');self.assertEqual(result['actor_evidence']['text'],'installed it, my friend')
 def test_memory_and_requirements(self):
  with tempfile.TemporaryDirectory() as directory:
   e=Investigator(ROOT,OLD,Path(directory),verify=False);action=dict(kind='env',target='OPTION_X',value='1');url='https://github.com/ValveSoftware/Proton/issues/1';case=dict(context=CTX,symptom='fixture crash',origin='synthetic-contract',attempts=[dict(id='fixture-A',context=CTX,action=action,outcome='not-resolved')],candidate_actions=[dict(context=CTX,action=action,url=url)],evidence=[])
   self.assertEqual(e.analyze(case)['candidate_actions'][0]['status'],'previously-failed-same-exact-context');again=copy.deepcopy(case);again['attempts']=[];fresh=Investigator(ROOT,OLD,Path(directory),verify=False);self.assertEqual(fresh.analyze(again)['candidate_actions'][0]['status'],'previously-failed-same-exact-context')
   again['context']=dict(CTX,driver='new-driver');again['candidate_actions'][0]['context']=again['context'];self.assertFalse(fresh.analyze(again)['attempts'])
 def test_incompatible_source_retained_as_analogy(self):
  e=Investigator(ROOT,OLD,verify=False);case=dict(context=CTX,symptom='fixture crash',evidence=[dict(app_id='10001',url='https://github.com/ValveSoftware/Proton/issues/1',text='I installed a component and the game worked.',context=dict(CTX,gpu='fixture-nvidia'))],candidate_actions=[dict(context=CTX,action={},url='https://github.com/ValveSoftware/Proton/issues/1',requirements=dict(gpu_vendor='NVIDIA'))]);out=e.analyze(case);self.assertEqual(len(out['ranked_evidence']),1);self.assertEqual(out['ranked_evidence'][0]['applicability'],'incompatible');self.assertEqual(out['candidate_actions'][0]['status'],'context-incompatible')
 def test_gpu_does_not_prove_physx(self):
  out=Investigator(ROOT,OLD,verify=False).analyze(dict(context=dict(CTX,gpu='RTX4060'),symptom='fixture crash',evidence=[]));self.assertFalse(any('physx' in str(h).lower() for h in out['hypotheses']));self.assertFalse(out['confirmed_causes'])
if __name__=='__main__':unittest.main()
