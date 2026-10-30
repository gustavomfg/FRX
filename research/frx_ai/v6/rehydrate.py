"""Reconstruct the already-known v6 evaluation; no new blind-test claim.

Local cached originals are preferred. --fetch explicitly permits at most120
bounded public GitHub comment requests. No external source is executed.
"""
import argparse,json,re,shutil,subprocess
from pathlib import Path
from datetime import datetime,timezone
from bridge import HERE,read,write,jsonl,sha,digest

def hydrate(root,models,old,fetch=False,training=False):
 root=root.resolve();repo=HERE.parents[2]
 if root.is_relative_to(repo) or root==models.resolve():raise ValueError('use a fresh external research directory')
 root.mkdir(parents=True,exist_ok=True)
 if (root/'frozen.json').exists():raise ValueError('preserve prior reconstruction')
 snapshot=HERE/'snapshot';manifest=json.loads((snapshot/'manifest.json').read_text())
 for name,h in manifest['files'].items():
  if sha(snapshot/name)!=h:raise ValueError('published metadata changed: '+name)
 bodies={}
 for file in (models/'public').glob('*.json'):
  value=json.loads(file.read_text())
  for comment in value if isinstance(value,list) else [value]:
   if isinstance(comment,dict) and isinstance(comment.get('body'),str):bodies[digest(comment['body'])]=comment['body']
 requests=0;groups={}
 names=['development-reserved','development-gold','development-gold-v2']+([] if training else ['acceptance-reserved','identity-challenge-reserved','context-challenge-reserved','acceptance-gold'])
 for name in names:
  rows=[]
  for meta in json.loads((snapshot/(name+'.json')).read_text()):
   key=meta['source_sha256'];body=bodies.get(key)
   if body is None:
    if not fetch:raise FileNotFoundError('original public cache required or explicit --fetch: '+key)
    match=re.fullmatch(r'https://github.com/ValveSoftware/Proton/issues/\d+#issuecomment-(\d+)',meta['url'])
    if not match or requests>=120:raise ValueError('source identity/API budget120')
    requests+=1;raw=subprocess.check_output(['gh','api','repos/ValveSoftware/Proton/issues/comments/'+match[1]],timeout=45)
    if len(raw)>2000000:raise ValueError('API size bound')
    body=json.loads(raw)['body']
    if not isinstance(body,str) or len(body)>200000 or digest(body)!=key:raise ValueError('public source edited; original cached snapshot needed')
    bodies[key]=body
   if digest(body)!=key:raise ValueError('cached source hash mismatch')
   text=body[meta['source_offset']:meta['source_offset']+meta['paragraph_length']]
   if digest(text)!=meta['paragraph_sha256']:raise ValueError('unit offsets/hash mismatch')
   rows.append(dict(meta,text=text,source_text=body))
  groups[name]=rows;jsonl(root/(name+'.jsonl'),rows)
 for name in ['catalog','syntax-manifest','selection','reservation','identity-challenge-reservation','context-challenge-reservation']:
  shutil.copyfile(snapshot/(name+'.json'),root/(name+'.json'))
 compact=json.loads((snapshot/'compact-manifest.json').read_text());compact['directory']=json.loads((models/'compact-manifest.json').read_text())['directory'];write(root/'compact-manifest.json',compact)
 (root/'verifiers').mkdir(exist_ok=True);model='verifiers/L1-structured-real-text.json';reference=json.loads((snapshot/'artifact-reference.json').read_text())['classifier']
 if sha(models/model)!=reference['sha256']:raise ValueError('trained model hash mismatch')
 shutil.copyfile(models/model,root/model)
 if training:
  # Train matrix is generated from the old83 sources, never the new acceptance
  # set. Reuse its hashed original only for an explicitly known-data refit.
  shutil.copyfile(models/'training-pairs.jsonl',root/'training-pairs.jsonl');print('Known-data refit area; no acceptance labels loaded');return
 if not (root/'syntax-models').exists():(root/'syntax-models').symlink_to((models/'syntax-models').resolve(),target_is_directory=True)
 # The frozen schema needs training-matrix identity, not training access for
 # inference. Copy external matrix; it is never part of repository metadata.
 shutil.copyfile(models/'training-pairs.jsonl',root/'training-pairs.jsonl')
 original=json.loads((snapshot/'frozen.json').read_text());meta=dict(original,original_frozen_at=original['frozen_at'],reconstructed_at=datetime.now(timezone.utc).isoformat(),evaluation_mode='known published replay; not a new blind test',acceptance_bodies_inspected=True,acceptance_labels_seen=True)
 meta['artifacts']={name:sha(root/name) for name in original['artifacts']};write(root/'frozen.json',meta);write(root/'label-lock.json',dict(sha256=sha(root/'acceptance-gold.jsonl'),predictions_seen=True,review='published provisional labels; known replay',human_review=False))
 print('Known evaluation reconstructed; public API requests',requests)
if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('root',type=Path);p.add_argument('models',type=Path);p.add_argument('old',type=Path);p.add_argument('--fetch',action='store_true');p.add_argument('--for-training',action='store_true');a=p.parse_args();hydrate(a.root,a.models,a.old,a.fetch,a.for_training)
