"""Explicit synthetic contrast controls, never real benchmark observations."""
from data import annotate

def build(count=160):
 output=[]
 patterns=[
  ('I enabled {x} and the crash stopped.','enabled','performed','enable'),
  ('Try enabling {x} to see whether it helps.','enabling','suggested','enable'),
  ("I haven't tried enabling {x}.",'enabling','negated','enable'),
  ('{x} was already enabled before this incident.','enabled','preexisting','enable'),
  ('I disabled {x} and it caused a crash.','disabled','performed','disable'),
  ('Tomorrow I will disable {x}.','disable','suggested','disable'),
  ('I installed {x}, but the crashes persisted.','installed','performed','install'),
  ('I did not install {x}.','install','negated','install'),
  ('I removed {x} and the game started.','removed','performed','remove'),
  ('You could remove {x}.','remove','suggested','remove'),
  ('I updated my comment about {x}.',None,None,None),
  ('The GPU log says: VRAM table enabled at 0x1234 for {x}.',None,None,None),
  ('I have checked whether updates for my system are available: {x}.',None,None,None),
  ('I tried {x}, then I installed library_beta.dll.','tried','performed','configure'),
  ('If you enable {x}, it might help.','enable','unknown','enable'),
  ('We replaced {x} and cleared the cache.','replaced','performed','replace')]
 for i in range(count):
  fmt,trigger,status,direction=patterns[i%len(patterns)];name=['FEATURE_ALPHA','PROTON_NO_ESYNC=1','component_alpha.dll','OPTION_BETA'][i//len(patterns)%4];text=fmt.format(x=name)
  row=dict(id='synthetic-'+str(i),app_id='synthetic-training',url=None,text=text,origin='synthetic-training-only')
  events=[]
  if trigger:events.append((trigger,status,direction,name,None))
  if i%len(patterns)==13:events.append(('installed','performed','install','library_beta.dll',None))
  if i%len(patterns)==15:events.append(('cleared','performed','remove','the cache',None))
  output.append(annotate(row,events,note='Synthetic contrast; no measurement or source assertion.'))
 return output
