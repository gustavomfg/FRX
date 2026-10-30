"""Freeze selected v4 artifacts and code before independent final annotation."""
import argparse
from pathlib import Path
import json,time,hashlib

HERE=Path(__file__).resolve().parent
def sha(p):return hashlib.sha256(Path(p).read_bytes()).hexdigest()

def seal(root):
 if (root/'frozen.json').exists():raise ValueError('preserve first freeze')
 files=['reservation.json','final-paragraphs.jsonl','train-audited.jsonl','dev-audited.jsonl','selection.json','lexicon.json','runtime-events/runtime.json','runtime-events/tokenizer.json']
 meta=json.loads((root/'runtime-events/runtime.json').read_text());files.append('runtime-events/'+meta['model_file'])
 value=dict(created_utc=time.strftime('%Y-%m-%dT%H:%M:%SZ',time.gmtime()),final_source_bodies_inspected=False,final_labels_seen=False,
  files={name:sha(root/name) for name in files},code={p.name:sha(p) for p in HERE.glob('*.py')},selected_model=meta,
  result_associations_default_enabled=False,automatic_approval=False)
 with (root/'frozen.json').open('x') as f:json.dump(value,f,indent=2)
 print('Sealed v4; no new final labels or bodies inspected')

def verify(root):
 frozen=json.loads((root/'frozen.json').read_text())
 for name,h in frozen['files'].items():
  if sha(root/name)!=h:raise ValueError('changed frozen artifact: '+name)
 for name,h in frozen['code'].items():
  if sha(HERE/name)!=h:raise ValueError('changed frozen source: '+name)
 return frozen

if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('root',type=Path);p.add_argument('--verify',action='store_true');a=p.parse_args();verify(a.root) if a.verify else seal(a.root)
