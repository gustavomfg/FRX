"""Post-freeze research-context contracts; no model fitting or app tests."""
import unittest
from run import intake

class ContextIntake(unittest.TestCase):
 def test_supplied_attempt_cannot_cross_build_gpu_driver_or_game(self):
  context=dict(app_id='200260',gpu='fixture-nvidia',driver='fixture1',proton='fixture1',game_version='fixture1',technology='fixture-api',configuration={'flag':1})
  attempt=dict(id='fixture',context=context,action={'kind':'fixture'},outcome='not-resolved')
  clean,rejected=intake(dict(context=context,attempts=[attempt]));self.assertEqual(clean['attempts'],[attempt]);self.assertFalse(rejected)
  for key,value in [('app_id','99999'),('gpu','fixture-amd'),('driver','fixture2'),('proton','fixture2'),('game_version','fixture2'),('technology','other'),('configuration',{'flag':0})]:
   clean,rejected=intake(dict(context=context,attempts=[dict(attempt,context=dict(context,**{key:value}))]));self.assertFalse(clean['attempts']);self.assertTrue(rejected)
 def test_missing_build_does_not_inherit_known_build(self):
  context=dict(app_id='200260',gpu='fixture',driver='fixture',proton='fixture',game_version='fixture')
  missing={k:v for k,v in context.items() if k!='game_version'}
  clean,rejected=intake(dict(context=context,attempts=[dict(context=missing)]));self.assertFalse(clean['attempts'])
 def test_candidate_is_not_auto_promoted_across_optional_context(self):
  context=dict(app_id='200260',gpu='fixture',driver='fixture',proton='fixture',configuration={'a':1})
  clean,rejected=intake(dict(context=context,candidate_actions=[dict(context=dict(context,configuration={'a':2}))]));self.assertFalse(clean['candidate_actions']);self.assertEqual(rejected[0]['reason'],'different-or-unknown-configuration')

if __name__=='__main__':unittest.main()
