"""Reconstruct a known published evaluation, with explicit bounded API fetching.

Never call a reconstruction of published labels a new independent evaluation.
No source snippets are executed. Existing research model cache is required.
"""
import argparse,json,re,hashlib,subprocess,shutil
from pathlib import Path
from datetime import datetime,timezone
from common import HERE,write,jsonl,digest
from seal import sha

def hydrate(root,models,old,body_cache,fetch=False,training=False):
 if root.resolve().is_relative_to(HERE.parents[2]):raise ValueError('raw research cache must be outside repository')
 root.mkdir(parents=True,exist_ok=True)
 if (root/'frozen.json').exists():raise ValueError('preserve existing reconstruction')
 snapshot=HERE/'snapshot';local=root/'bodies';local.mkdir(exist_ok=True);requests=0;groups={}
 for name in ['train-known','dev-known','extra-audited']+([] if training else ['final-audited']):
  rows=[]
  for meta in json.loads((snapshot/(name+'.json')).read_text()):
   key=meta['source_sha256'];dest=local/(key+'.txt')
   if not dest.exists() and body_cache and (body_cache/(key+'.txt')).exists():shutil.copyfile(body_cache/(key+'.txt'),dest)
   if not dest.exists():
    if not fetch:raise FileNotFoundError('source cache required or explicit --fetch: '+key)
    match=re.fullmatch(r'https://github.com/ValveSoftware/Proton/issues/\d+#issuecomment-(\d+)',meta['url'])
    if not match:raise ValueError('bounded public source identity')
    if requests>=100:raise ValueError('API budget100; resume using preserved local bodies')
    requests+=1;raw=subprocess.check_output(['gh','api','repos/ValveSoftware/Proton/issues/comments/'+match[1]],timeout=45)
    if len(raw)>2000000:raise ValueError('API body bound')
    body=json.loads(raw)['body']
    if len(body)>200000 or digest(body)!=key:raise ValueError('source changed; original cached snapshot required')
    dest.write_bytes(body.encode())
   body=dest.read_bytes().decode()
   if digest(body)!=key:raise ValueError('cached source changed')
   text=body[meta['source_offset']:meta['source_offset']+meta['paragraph_length']]
   if digest(text)!=meta['paragraph_sha256']:raise ValueError('unit changed')
   rows.append(dict(meta,text=text,source_text=body))
  groups[name]=rows;jsonl(root/(name+'.jsonl'),rows)
 for name in ['syntax-manifest','selection','reservation']:
  shutil.copyfile(snapshot/(name+'.json'),root/(name+'.json'))
 # Explicit reuse/copy; inference never downloads models.
 shutil.copytree(models/'syntax-models',root/'syntax-models',dirs_exist_ok=True)
 compact=json.loads((snapshot/'compact-manifest.json').read_text());compact['directory']=json.loads((models/'compact-manifest.json').read_text())['directory'];write(root/'compact-manifest.json',compact)
 if training:
  print('Prepared known-data refit inputs only; no final labels loaded; API requests',requests);return
 reserved=[{k:v for k,v in r.items() if k not in ['events','observations','temporal','audit_note','annotation_version','annotation_provenance']} for r in groups['final-audited']];jsonl(root/'final-reserved.jsonl',reserved)
 original=json.loads((snapshot/'frozen.json').read_text());meta=dict(original,original_frozen_at=original['frozen_at'],reconstructed_at=datetime.now(timezone.utc).isoformat(),evaluation_mode='known published replay; not a new blind test',final_bodies_inspected=True)
 meta['artifacts']={name:sha(root/name) for name in original['artifacts']}
 write(root/'frozen.json',meta);write(root/'annotation-lock.json',dict(sha256=sha(root/'final-audited.jsonl'),review='published provisional labels; known replay',predictions_seen=True))
 print('Reconstructed published data; no new independent validation; API requests',requests)
if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('root',type=Path);p.add_argument('models',type=Path);p.add_argument('old',type=Path);p.add_argument('--body-cache',type=Path);p.add_argument('--fetch',action='store_true');p.add_argument('--for-training',action='store_true');a=p.parse_args();hydrate(a.root,a.models,a.old,a.body_cache,a.fetch,a.for_training)
