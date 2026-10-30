"""Freeze inference before reserved source inspection. Does not claim blind humans."""
import argparse,hashlib,json
from pathlib import Path
from datetime import datetime,timezone
from common import HERE,write

FILES=['events.py','relations.py','syntax.py','compact.py','linker.py','incidents.py','lab.py','common.py','assess.py','syntax_control.py','final_evaluate.py']
def sha(path):return hashlib.sha256(path.read_bytes()).hexdigest()
def freeze(root,old):
 if (root/'frozen.json').exists():raise ValueError('already frozen')
 sources={p:sha(HERE/p) for p in FILES}
 artifacts={p:sha(root/p) for p in ['selection.json','syntax-manifest.json','compact-manifest.json','train-known.jsonl','dev-known.jsonl','extra-audited.jsonl','reservation.json','final-reserved.jsonl']}
 preserved={str(p.relative_to(old)):sha(p) for p in sorted((old/'runtime-retrieval').rglob('*')) if p.is_file()}
 write(root/'frozen.json',dict(frozen_at=datetime.now(timezone.utc).isoformat(),inference_sources=sources,artifacts=artifacts,preserved_v2=preserved,final_bodies_inspected=False,independent_human_review=False))
def verify(root,old=None):
 meta=json.loads((root/'frozen.json').read_text())
 for name,value in meta['inference_sources'].items():
  if sha(HERE/name)!=value:raise ValueError('frozen inference source changed: '+name)
 for name,value in meta['artifacts'].items():
  if sha(root/name)!=value:raise ValueError('frozen input/artifact changed: '+name)
 if old:
  for name,value in meta['preserved_v2'].items():
   if sha(old/name)!=value:raise ValueError('preserved retrieval changed')
 return meta
if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('root',type=Path);p.add_argument('old',type=Path);p.add_argument('--verify',action='store_true');a=p.parse_args();print(verify(a.root,a.old) if a.verify else freeze(a.root,a.old))
