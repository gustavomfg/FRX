import tempfile
from pathlib import Path
import unittest
from memory import CaseMemory

class MemoryContracts(unittest.TestCase):
 def record(self):
  context=dict(app_id='200260',gpu='fixture-gpu',driver='fixture-driver',proton='fixture-proton')
  return context,dict(id='a',context=context,action=dict(kind='environment',target='OPTION',direction='enable'),outcome='not-resolved')
 def test_restart_and_driver_change(self):
  with tempfile.TemporaryDirectory() as tmp:
   path=Path(tmp)/'memory.json';context,a=self.record();CaseMemory(path).exchange(context,[a])
   self.assertEqual(CaseMemory(path).exchange(context,[]),[a])
   self.assertEqual(CaseMemory(path).exchange(dict(context,driver='changed'),[]),[])
 def test_conflict_preserves_original(self):
  with tempfile.TemporaryDirectory() as tmp:
   path=Path(tmp)/'memory.json';context,a=self.record();m=CaseMemory(path);m.exchange(context,[a]);before=path.read_bytes()
   with self.assertRaises(ValueError):m.exchange(context,[dict(a,outcome='positive')])
   self.assertEqual(path.read_bytes(),before)
 def test_other_game_and_unknown_context_do_not_enter_history(self):
  with tempfile.TemporaryDirectory() as tmp:
   context,a=self.record();m=CaseMemory(Path(tmp)/'memory.json')
   self.assertEqual(m.exchange(dict(context,app_id='12345'),[a]),[])
   self.assertEqual(m.exchange({'app_id':'200260'},[a]),[])
 def test_duplicate_not_counted_twice(self):
  with tempfile.TemporaryDirectory() as tmp:
   context,a=self.record();m=CaseMemory(Path(tmp)/'memory.json');self.assertEqual(m.exchange(context,[a,a]),[a])

if __name__=='__main__':unittest.main()
