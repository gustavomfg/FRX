"""Reuse v3 atomic history inside a more specific research context namespace."""
import hashlib,json,sys
from pathlib import Path

V3=Path(__file__).resolve().parents[1]/'v3'
if str(V3) not in sys.path:sys.path.append(str(V3))
from memory import CaseMemory

def scope(case):
 context=case['context'];extra={k:context.get(k) for k in ['app_id','gpu','driver','proton','game_version','technology','configuration']}
 extra['symptom']=case.get('symptom_id',case.get('symptom','')).casefold().strip()
 payload=json.dumps(extra,sort_keys=True,ensure_ascii=False)
 if len(payload)>8000:raise ValueError('context namespace bound')
 return hashlib.sha256(payload.encode()).hexdigest()

class ContextMemory:
 def __init__(self,directory):self.directory=Path(directory)
 def exchange(self,case):
  # Old v3 records lacking this namespace are not silently assigned to a build.
  name=scope(case);path=self.directory/(name+'.json')
  records=CaseMemory(path).exchange(case['context'],case.get('attempts',[]))
  return dict(case,attempts=records),name
