"""Freeze inference/config and hidden acceptance membership before inspection."""
import argparse,json
from pathlib import Path
from datetime import datetime,timezone
from bridge import HERE,sha,write
CORE=['bridge.py','grounding.py','pairs.py','engine.py','classifier.py','diagnostic.py','investigate.py','evaluate.py','final.py']
INPUTS=['selection.json','catalog.json','compact-manifest.json','syntax-manifest.json','reservation.json','identity-challenge-reservation.json','context-challenge-reservation.json','acceptance-reserved.jsonl','identity-challenge-reserved.jsonl','context-challenge-reserved.jsonl','development-gold-v2.jsonl','training-pairs.jsonl','verifiers/L1-structured-real-text.json']
def freeze(root,old):
 if (root/'frozen.json').exists():raise ValueError('preserve first freeze')
 previous=HERE.parent/'v5';prior_sources={str(p.relative_to(previous)):sha(p) for p in previous.glob('*.py')};v2={str(p.relative_to(old)):sha(p) for p in (old/'runtime-retrieval').glob('*') if p.is_file()}
 meta=dict(frozen_at=datetime.now(timezone.utc).isoformat(),inference_sources={n:sha(HERE/n) for n in CORE},artifacts={n:sha(root/n) for n in INPUTS},prior_v5_sources=prior_sources,preserved_v2=v2,acceptance_bodies_inspected=False,acceptance_labels_seen=False,human_review=False)
 write(root/'frozen.json',meta);print('Frozen v6 before acceptance body inspection')
def verify(root,old=None):
 meta=json.loads((root/'frozen.json').read_text())
 for name,h in meta['inference_sources'].items():
  if sha(HERE/name)!=h:raise ValueError('frozen inference source changed: '+name)
 for name,h in meta['artifacts'].items():
  if sha(root/name)!=h:raise ValueError('frozen artifact changed: '+name)
 for name,h in meta['prior_v5_sources'].items():
  if sha(HERE.parent/'v5'/name)!=h:raise ValueError('previous v5 source changed')
 if old:
  for name,h in meta['preserved_v2'].items():
   if sha(old/name)!=h:raise ValueError('preserved v2 artifact changed')
 return meta
if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('root',type=Path);p.add_argument('old',type=Path);p.add_argument('--verify',action='store_true');a=p.parse_args();verify(a.root,a.old) if a.verify else freeze(a.root,a.old)
