"""Reconstruct published annotations, preserving exact source/hash/Unicode offsets.

Public fetching is explicit and bounded. This is replay of known labels, never
new independent validation. No source text is executed or used as instructions.
"""
import argparse,hashlib,json,re,subprocess
from pathlib import Path
from data import validate

HERE=Path(__file__).resolve().parent

def hydrate(root,fetch=False,body_cache=None):
 if root.resolve().is_relative_to(HERE.parents[2]):raise ValueError('raw sources must remain outside the repository')
 root.mkdir(parents=True,exist_ok=True);cache=root/'bodies';cache.mkdir(exist_ok=True);requests=0
 summary=json.loads((HERE/'snapshot/datasets.json').read_text());split_apps=[]
 for split in ['train','dev','final']:
  rows=[];annotations=json.loads((HERE/'snapshot'/(split+'-annotations.json')).read_text())
  for ann in annotations:
   fields=ann['data'];digest=fields['source_sha256'];file=cache/(digest+'.txt')
   if not file.exists() and body_cache:
    other=body_cache/(digest+'.txt')
    if other.exists():file.write_bytes(other.read_bytes())
   if not file.exists():
    if not fetch:raise FileNotFoundError('original body cache required; fetching needs explicit --fetch')
    match=re.fullmatch(r'https://github.com/ValveSoftware/Proton/issues/\d+#issuecomment-(\d+)',fields['url'])
    if not match:raise ValueError('unsupported public source identity')
    requests+=1
    if requests>150:raise ValueError('public request count bound')
    raw=subprocess.check_output(['gh','api','repos/ValveSoftware/Proton/issues/comments/'+match[1]],timeout=45)
    if len(raw)>2000000:raise ValueError('public response size bound')
    body=json.loads(raw)['body'];encoded=body.encode()
    if hashlib.sha256(encoded).hexdigest()!=digest:raise ValueError('source changed; original source cache required, do not relabel offsets')
    file.write_bytes(encoded)
   body=file.read_bytes().decode()
   if hashlib.sha256(body.encode()).hexdigest()!=digest:raise ValueError('source integrity mismatch')
   start=fields['source_offset'];text=body[start:start+ann['paragraph_length']]
   if hashlib.sha256(text.encode()).hexdigest()!=ann['paragraph_sha256']:raise ValueError('paragraph changed')
   row={k:body if k=='source_text' else text if k=='text' else fields[k] for k in ann['row_keys']};validate(row);rows.append(row)
  payload=''.join(json.dumps(r,ensure_ascii=False)+'\n' for r in rows).encode()
  if hashlib.sha256(payload).hexdigest()!=summary['datasets'][split]['file_sha256']:raise ValueError('reconstructed dataset differs from snapshot')
  target=root/(split+'-audited.jsonl')
  if target.exists() and target.read_bytes()!=payload:raise ValueError('preserve existing different dataset')
  target.write_bytes(payload)
  if split=='final':
   excluded={'annotation_version','annotation_provenance','events','observations','temporal','audit_note'}
   original=''.join(json.dumps({k:v for k,v in r.items() if k not in excluded},ensure_ascii=False)+'\n' for r in rows).encode()
   expected=json.loads((HERE/'snapshot/frozen.json').read_text())['files']['final-paragraphs.jsonl']
   if hashlib.sha256(original).hexdigest()!=expected:raise ValueError('reserved source units changed')
   for name,content in [('final-paragraphs.jsonl',original),('reservation.json',(HERE/'snapshot/reservation.json').read_bytes())]:
    dest=root/name
    if dest.exists() and dest.read_bytes()!=content:raise ValueError('preserve different reserved sources')
    dest.write_bytes(content)
  split_apps.append({r['app_id'] for r in rows});print(split,len(rows),'hash verified')
 if any(split_apps[i]&split_apps[j] for i in range(3) for j in range(i)):raise ValueError('cross-game partition overlap')

if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('root',type=Path);p.add_argument('--fetch',action='store_true');p.add_argument('--body-cache',type=Path);a=p.parse_args();hydrate(a.root,a.fetch,a.body_cache)
