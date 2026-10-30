"""Build final proposals without annotation or printing new source bodies."""
import argparse
from pathlib import Path
import json
import hashlib
import re
from common import read,write,jsonl,units
from events import ACTION

RESULT=re.compile(r'\b(?:work\w*|fix\w*|resolv\w*|crash\w*|freez\w*|froz\w*|worse|better|improv\w*|help\w*|flicker\w*|stutter\w*|artifact\w*|launch\w*|no change|no luck)\b',re.I)

def proposals(text):
 chunks=units(text);action=None
 for a,b in chunks:
  m=ACTION.search(text[a:b])
  if m:
   action=[a+m.start(),min(b,a+m.start()+180)];break
 if action is None:action=chunks[0] if chunks else [0,min(len(text),180)]
 options=[s for s in chunks if RESULT.search(text[s[0]:s[1]])]
 options.sort(key=lambda s:(min(abs(s[0]-action[1]),abs(s[1]-action[0])),s[0]))
 result=options[0] if options else action
 return action,result

def prepare(root):
 if (root/'frozen.json').exists():raise ValueError('already sealed')
 reservation=json.loads((root/'reservation.json').read_text());rows=[]
 for item in reservation['records']:
  sources=json.loads((root/'sources'/f"{item['thread']}-comments.json").read_text());x=next(x for x in sources if x['id']==item['id']);text=x['body']
  assert hashlib.sha256(text.encode()).hexdigest()==item['body_sha256']
  action,result=proposals(text);rows.append(dict(item,text=text,action_span=action,observation_span=result,origin='real-public-github'))
 jsonl(root/'final-candidates.jsonl',rows);print('Prepared',len(rows),'blind source proposals')

if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('root',type=Path);prepare(p.parse_args().root)
