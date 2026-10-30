"""Real span labels and explicitly synthetic contrasts; no final-source access."""
import random
from common import read

PAST=[('enabled','enable','OPTION_A'),('disabled','disable','OPTION_B'),('installed','install','the missing runtime'),('removed','remove','the extra library'),('updated','update','the graphics driver'),('switched','switch','the Proton version'),('replaced','replace','the configuration file')]

def synthetic(n=120):
 rng=random.Random(504);rows=[]
 for i in range(n):
  word,direction,target=PAST[i%len(PAST)];kind=i%6
  if kind==0:text=f'I {word} {target}. The game still crashed.';status='performed'
  elif kind==1:text=f'I have {word} {target}, and the game started working.';status='performed'
  elif kind==2:text=f'Could you try to {direction} {target}?';status='suggested'
  elif kind==3:text=f"I have not {word} {target}. No outcome of that trial is known.";status='negated'
  elif kind==4:text=f'{target.capitalize()} was already {word} before this problem.';status='preexisting'
  else:text=f'Tomorrow I will investigate {target}. I have not {word} it yet.';status='negated'
  trigger=direction if kind==2 else word;a=text.index(trigger);rows.append(dict(id='synthetic-status-'+str(i),app_id='synthetic-training-only',text=text,events=[dict(trigger=[a,a+len(trigger)],status=status,direction=direction)],origin='explicit-synthetic-training-only'))
 return rows

def units(rows):
 return [dict(text=r['text'],span=e['trigger'],status=e['status'],id=r['id'],app_id=r['app_id']) for r in rows for e in r['events']]
