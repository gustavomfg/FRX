"""Post-evaluation guard tests, not additional blind language evaluation."""
import unittest
from run import guarded
class Stub:
 def analyze(self,case):return dict(event_reports=[dict(app_id='1817070',events=[dict(status='performed')])],limitations=[])
class ScopeGuard(unittest.TestCase):
 def test_thread_does_not_become_event_appid(self):
  r=guarded(Stub(),{});p=r['event_reports'][0];self.assertNotIn('app_id',p);self.assertEqual(p['source_thread_app_id'],'1817070');self.assertIsNone(p['event_app_id']);self.assertFalse(p['event_environment_verified'])
 def test_unguessed_environment(self):self.assertEqual(guarded(Stub(),{})['event_reports'][0]['events'][0]['environment_attribution'],'unverified')
if __name__=='__main__':unittest.main()
