"""Seal all v3 code/selected artifacts before reading new final source texts."""
import argparse
from pathlib import Path
import json
import time
from common import sha,write,HERE

def seal(root):
 dest=root/'frozen.json'
 if dest.exists():raise ValueError('preserve first freeze')
 paths=[root/'reservation.json',root/'final-candidates.jsonl']
 artifacts=list((root/'events').glob('*.joblib'))+[root/'events/selection.json',root/'ranking/selection.json']
 payload=dict(created_utc=time.strftime('%Y-%m-%dT%H:%M:%SZ',time.gmtime()),final_texts_inspected=False,final_labels_seen=False,
  code={p.name:sha(p) for p in HERE.glob('*.py')},artifacts={str(p.relative_to(root)):sha(p) for p in artifacts},
  inputs={p.name:sha(p) for p in paths},selection_uses='v2 final is known development; v3 reservation unseen',automatic_approval=False)
 with dest.open('x') as f:json.dump(payload,f,indent=2)
 print('Sealed code and selected artifacts; new final labels not accessed')

def verify(root):
 x=json.loads((root/'frozen.json').read_text())
 for n,h in x['code'].items():
  if sha(HERE/n)!=h:raise ValueError('changed frozen code: '+n)
 for section in ['artifacts','inputs']:
  for n,h in x[section].items():
   if sha(root/n)!=h:raise ValueError('changed frozen '+section+': '+n)
 return x

if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('root',type=Path);p.add_argument('--verify',action='store_true');a=p.parse_args();verify(a.root) if a.verify else seal(a.root)
