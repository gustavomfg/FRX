"""Replay six training fits and development comparisons into a NEW external run.

Published labels make this replication, not a second independent evaluation.
The original sealed run is never modified. Requires prepared v2/v3 artifacts.
"""
import argparse,hashlib,json,subprocess,sys,time
from pathlib import Path
from hydrate import hydrate

HERE=Path(__file__).resolve().parent

def main():
 p=argparse.ArgumentParser();p.add_argument('root',type=Path);p.add_argument('old',type=Path);p.add_argument('v3',type=Path);p.add_argument('--body-cache',type=Path);p.add_argument('--fetch',action='store_true');a=p.parse_args()
 if a.root.resolve().is_relative_to(HERE.parents[2]):raise ValueError('models/datasets must stay outside the repository')
 if any((a.root/x).exists() for x in ['frozen.json','runs','selection.json']):raise ValueError('new run required; preserve previous experiments')
 hydrate(a.root,a.fetch,a.body_cache)
 for name in ['S1-legacy-span','S2-audited-real','S3-contrast-events','S4-frozen-body','S5-balanced-events','S6-balanced-no-contrast']:
  subprocess.run([sys.executable,str(HERE/'train.py'),str(a.root),str(a.old),name],check=True)
 subprocess.run([sys.executable,str(HERE/'study.py'),str(a.root),str(a.old),str(a.v3)],check=True)
 subprocess.run([sys.executable,str(HERE/'export.py'),str(a.root),str(a.old)],check=True)
 files=['reservation.json','final-paragraphs.jsonl','train-audited.jsonl','dev-audited.jsonl','selection.json','lexicon.json','runtime-events/runtime.json','runtime-events/tokenizer.json']
 meta=json.loads((a.root/'runtime-events/runtime.json').read_text());files.append('runtime-events/'+meta['model_file'])
 sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
 value=dict(created_utc=time.strftime('%Y-%m-%dT%H:%M:%SZ',time.gmtime()),replication_of_published_data=True,final_source_bodies_inspected=True,final_labels_seen=True,files={n:sha(a.root/n) for n in files},code={p.name:sha(p) for p in HERE.glob('*.py')},selected_model=meta,result_associations_default_enabled=False,automatic_approval=False)
 (a.root/'frozen.json').write_text(json.dumps(value,indent=2))
 print('Replication completed. Published final labels are known; use a new cohort for further selection.')
 print('A clearly marked replication seal permits local CLI use; it is not a blind final test or an approved model. The original seal is preserved.')

if __name__=='__main__':main()
