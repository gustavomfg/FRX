"""Post-evaluation engineering: persist caller-reported attempts, not predictions.

This is contextual case memory, not learned causal effects or an online neural
update. The path is a CLI parameter, never chosen by retrieved text or a case.
"""
import fcntl
import json
import os
from pathlib import Path
import tempfile
from investigation import context_id,compatible

class CaseMemory:
 def __init__(self,path):self.path=Path(path)
 def exchange(self,context,attempts):
  self.path.parent.mkdir(parents=True,exist_ok=True)
  lock=self.path.with_suffix(self.path.suffix+'.lock')
  with lock.open('a') as guard:
   fcntl.flock(guard,fcntl.LOCK_EX)
   if self.path.exists():
    if self.path.is_symlink() or self.path.stat().st_size>1000000:raise ValueError('memory file bound')
    data=json.loads(self.path.read_text())
   else:data={'format':1,'contexts':{}}
   if data.get('format')!=1 or not isinstance(data.get('contexts'),dict):raise ValueError('unknown memory format')
   key=context_id(context);known=data['contexts'].get(key,[]);by_id={x['id']:x for x in known}
   for record in attempts:
    # A fully specified same environment is necessary to reuse an attempt.
    if compatible(context,record.get('context',{}))!='compatible':continue
    identity=record.get('id')
    if not isinstance(identity,str) or len(identity)>120:raise ValueError('attempt ID required')
    if identity in by_id and by_id[identity]!=record:raise ValueError('conflicting historical attempt; preserve original')
    by_id[identity]=record
   result=list(by_id.values())
   if len(result)>64:raise ValueError('context attempt bound')
   if result:data['contexts'][key]=result
   payload=json.dumps(data,ensure_ascii=False)
   if len(payload.encode())>1000000:raise ValueError('memory total size bound')
   fd,tmp=tempfile.mkstemp(prefix='.frx-ai-memory-',dir=self.path.parent)
   try:
    with os.fdopen(fd,'w') as file:file.write(payload);file.flush();os.fsync(file.fileno())
    os.replace(tmp,self.path)
   finally:
    if os.path.exists(tmp):os.unlink(tmp)
   return result
