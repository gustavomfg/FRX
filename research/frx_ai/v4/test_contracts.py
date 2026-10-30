import unittest,tempfile,json
from pathlib import Path
import numpy as np
from data import literal,annotate
from proof import verify,attach_observations,temporal_edges
from decode import decode
from diagnosis import diagnose
from context_memory import ContextMemory,scope

class EvidenceContracts(unittest.TestCase):
 def action(self,text,word):return verify(text,literal(text,word))
 def test_performed_suggested_negated_preexisting(self):
  for text,word,status in [('I enabled OPTION.','enabled','performed'),('Try enabling OPTION.','enabling','suggested'),("I haven't tried enabling OPTION.",'enabling','negated'),('OPTION was already enabled before this incident.','enabled','preexisting'),('Tomorrow I will disable OPTION.','disable','suggested')]:
   self.assertEqual(self.action(text,word)['status'],status,(text,word))
 def test_past_attempt_is_not_negated_by_negative_result(self):
  x=self.action('I tried OPTION, but it did not work.','tried');self.assertEqual(x['status'],'performed')
 def test_contraction_and_unknown_library_identity(self):
  x=self.action("I've installed the missing library.",'installed');self.assertEqual(x['status'],'performed');self.assertIsNone(x['target_identity'])
 def test_file_identity_is_literal(self):
  x=self.action('I installed example.dll.','installed');self.assertEqual(x['target_identity'],'example.dll');self.assertEqual(x['target_evidence']['text'],'example.dll')
 def test_comment_update_not_game_procedure(self):self.assertIsNone(self.action("I've updated my comment.",'updated'))
 def test_quote_not_author_execution(self):self.assertIsNone(self.action('> I installed example.dll.','installed'))
 def test_no_execution_from_commands(self):self.assertIsNone(self.action('sudo apt install something','install'))
 def test_joint_outcome_never_individual_cause(self):
  text='I disabled A and installed B. It made no difference.';events=[self.action(text,'disabled'),self.action(text,'installed')];obs=attach_observations(text,events)
  self.assertEqual(len(obs),1);self.assertEqual(obs[0]['event_indices'],[0,1]);self.assertTrue(obs[0]['joint']);self.assertFalse(obs[0]['causality_confirmed'])
 def test_suggestion_cannot_own_a_success(self):
  text='Try enabling OPTION. It fixed the crashes.';event=self.action(text,'enabling');self.assertEqual(attach_observations(text,[event]),[])
 def test_explicit_sequence_not_causal_proof(self):
  text='I disabled A. Then I installed B.';events=[self.action(text,'disabled'),self.action(text,'installed')];edges=temporal_edges(text,events)
  self.assertEqual(len(edges),1);self.assertFalse(edges[0]['causality_confirmed'])
 def test_unicode_literal_offsets(self):
  text='Olá 🎮: I installed example.dll.';x=self.action(text,'installed');a,b=x['trigger']['span'];self.assertEqual(text[a:b],'installed')
 def test_gold_labels_do_not_enter_decoder(self):
  text='I enabled OPTION.';positions=[[0,0],[0,1],[2,9],[10,16],[16,17]];prob=[np.array([[1.,0],[1,0],[0,1],[1,0],[1,0]]),np.ones((5,5))/5,np.ones((5,9))/9]
  a=decode({'text':text},positions,prob,verified=True);b=decode({'text':text,'events':[{'status':'negated'}]},positions,prob,verified=True);self.assertEqual(a,b)
 def test_iou_does_not_reward_a_whole_paragraph(self):
  from metrics import evaluate
  text='I installed a library and this is a long sentence.';row=annotate(dict(id='fixture',app_id='200260',text=text),[('installed','performed','install','a library','I installed')]);event=self.action(text,'installed');event['trigger']['span']=[0,len(text)];m=evaluate([row],[dict(events=[event])],strict=True);self.assertEqual(m['matched'],0)
 def test_unimplemented_function_selects_specific_check(self):
  case=dict(context={'app_id':'200260'},log='Unhandled exception: unimplemented function example.dll.SampleFunction called in 64-bit code')
  out=diagnose(case);self.assertEqual(out['next_check']['id'],'verify-component-function-version');self.assertFalse(out['confirmed_causes']);self.assertIsNone(out['cause_probability'])
 def test_bare_error_name_not_runtime(self):
  out=diagnose(dict(context={'app_id':'200260'},log='Documentation about VK_ERROR_INCOMPATIBLE_DRIVER'))
  self.assertFalse(out['observations'])
 def test_duplicated_comments_and_threads_not_independent(self):
  e=dict(app_id='200260',url='https://github.com/ValveSoftware/Proton/issues/309#issuecomment-1',text='A repeated source statement.');out=diagnose(dict(context={'app_id':'200260'},evidence=[e,e,dict(e,url=e['url'][:-1]+'2',text='Another comment in the same discussion.')]))
  self.assertEqual(len(out['sources']),2);self.assertEqual(len(out['evidence_thread_groups']),1);self.assertFalse(out['evidence_thread_groups'][0]['independence_established'])
 def test_other_game_does_not_enter_case(self):
  e=dict(app_id='99999',url='https://github.com/ValveSoftware/Proton/issues/309',text='Some report from another game.');out=diagnose(dict(context={'app_id':'200260'},evidence=[e]));self.assertFalse(out['sources'])
 def test_more_specific_memory_survives_restart_without_crossing_context(self):
  context=dict(app_id='200260',gpu='fixture',driver='fixture',proton='fixture',game_version='one',technology='fixture-api',configuration={'flag':1});attempt=dict(id='a',context=context,action={'kind':'fixture'},outcome='not-resolved');case=dict(context=context,symptom='fixture-symptom',attempts=[attempt])
  with tempfile.TemporaryDirectory() as tmp:
   m=ContextMemory(tmp);m.exchange(case);out,key=ContextMemory(tmp).exchange(dict(case,attempts=[]));self.assertEqual(out['attempts'],[attempt])
   for changed in [dict(context,game_version='two'),dict(context,configuration={'flag':0}),dict(context,technology='other')]:
    other,_=m.exchange(dict(case,context=changed,attempts=[]));self.assertEqual(other['attempts'],[])

if __name__=='__main__':unittest.main()
