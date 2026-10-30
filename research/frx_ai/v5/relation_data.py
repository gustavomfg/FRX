"""Explicit synthetic procedure contrasts, separate from real evaluation."""
from training_data import PAST

def synthetic_relations(n=84):
 rows=[]
 for i in range(n):
  a,da,ta=PAST[i%len(PAST)];b,db,tb=PAST[(i+2)%len(PAST)];kind=i%4
  if kind==0:
   text=f'I {a} {ta}. It did not work. Then I {b} {tb}. It fixed the crash.';spec=[('It did not work','persistent',[0]),('It fixed the crash','improved',[1])];states=['performed','performed']
  elif kind==1:
   text=f'I {a} {ta} and {b} {tb}. The crashes stopped, but performance got worse.';spec=[('The crashes stopped','improved',[0,1]),('performance got worse','regression',[0,1])];states=['performed','performed']
  elif kind==2:
   text=f'I have not {a} {ta}. I {b} {tb}, and this caused a crash.';spec=[('this caused a crash','regression',[1])];states=['negated','performed']
  else:
   text=f'I {a} {ta} and {b} {tb}. That made no difference.';spec=[('That made no difference','persistent',[0,1])];states=['performed','performed']
  events=[dict(trigger=[text.index(word),text.index(word)+len(word)],status=status,direction=direction) for word,status,direction in [(a,states[0],da),(b,states[1],db)]]
  obs=[dict(span=[text.index(q),text.index(q)+len(q)],outcome=outcome,event_indices=owners,joint=len(owners)>1) for q,outcome,owners in spec]
  rows.append(dict(id='synthetic-procedure-'+str(i),app_id='synthetic-training-only',origin='explicit-synthetic-training-only',text=text,events=events,observations=obs))
 return rows
