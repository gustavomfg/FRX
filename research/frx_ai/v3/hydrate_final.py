"""Rebuild reserved final texts from hashes or explicit bounded GitHub fetches.

This post-evaluation reproduction tool is not a source of training labels.
"""
import argparse
from pathlib import Path
import hashlib
import json
import subprocess
import re

def main():
 p=argparse.ArgumentParser();p.add_argument('root',type=Path);p.add_argument('--fetch',action='store_true');a=p.parse_args()
 root=a.root;root.mkdir(parents=True,exist_ok=True);cache=root/'bodies';cache.mkdir(exist_ok=True)
 snapshot=Path(__file__).parent/'snapshot';annotations=json.loads((snapshot/'final-annotations.json').read_text());rows=[];labels=[];requests=0
 for ann in annotations:
  file=cache/(ann['body_sha256']+'.txt')
  if not file.exists():
   if not a.fetch:raise FileNotFoundError('original source cache missing; explicit --fetch required')
   m=re.fullmatch(r'https://github.com/ValveSoftware/Proton/issues/\d+#issuecomment-(\d+)',ann['url'])
   if not m:raise ValueError('source identity')
   requests+=1
   if requests>100:raise ValueError('bounded public fetch count')
   raw=subprocess.check_output(['gh','api','repos/ValveSoftware/Proton/issues/comments/'+m[1]],timeout=45)
   if len(raw)>2000000:raise ValueError('source response bound')
   text=json.loads(raw)['body']
   if hashlib.sha256(text.encode()).hexdigest()!=ann['body_sha256']:raise ValueError('public source changed; original cache required')
   file.write_bytes(text.encode())
  text=file.read_bytes().decode()
  if hashlib.sha256(text.encode()).hexdigest()!=ann['body_sha256']:raise ValueError('cached source changed')
  rows.append({k:ann[k] for k in ['id','url','app_id','thread','title','body_sha256','action_span','observation_span']}|dict(text=text,origin='real-public-github'))
  labels.append({k:ann[k] for k in ['id','index','topics','modality','observation_outcome','note','annotation']})
 (root/'final-candidates.jsonl').write_text(''.join(json.dumps(r,ensure_ascii=False)+'\n' for r in rows))
 (root/'final-annotations.json').write_text(json.dumps(labels,indent=2))
 print('Rebuilt',len(rows),'provisional annotated source units; no model fitting')

if __name__=='__main__':main()
