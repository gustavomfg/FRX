"""Publish own labels/metrics/hashes, never public source bodies or checkpoints."""
import argparse,json,hashlib
from pathlib import Path
from common import HERE,read,write,digest
from seal import verify

def publish(root):
 verify(root);dest=HERE/'snapshot';dest.mkdir(exist_ok=True);bodies=root/'bodies';bodies.mkdir(exist_ok=True)
 for name in ['train-known','dev-known','extra-audited','final-audited']:
  rows=read(root/(name+'.jsonl'));labels=[]
  for r in rows:
   body=r['source_text'];key=r['source_sha256']
   if digest(body)!=key or body[r['source_offset']:r['source_offset']+len(r['text'])]!=r['text']:raise ValueError('source/unit mismatch')
   file=bodies/(key+'.txt')
   if not file.exists():file.write_bytes(body.encode())
   clean={k:v for k,v in r.items() if k not in ['text','source_text']};clean.update(paragraph_length=len(r['text']),paragraph_sha256=digest(r['text']));labels.append(clean)
  write(dest/(name+'.json'),labels)
 for name in ['selection','frozen','reservation','annotation-lock','final-results','posthoc-review','incident-replay','incident-retrieval','retrieval-replay','benchmark','benchmark-runtime','label-audit','atomic-metadata-erratum','experiments','replication-controls','seed-audit','preservation','reproduction-parity','initial-development','preregistered-selection-results','compact-selection-results','linker-development','linker-reconstructed-development','linker-audited-development']:
  p=root/(name+'.json')
  if p.exists():write(dest/(name+'.json'),json.loads(p.read_text()))
 # Results are source-free; omit model parameters from the large threshold
 # grids only if a future format changes. Current grids contain metrics only.
 for p in sorted((root/'atomic-runs').glob('*.json')):write(dest/p.name,json.loads(p.read_text()))
 write(dest/'syntax-manifest.json',json.loads((root/'syntax-manifest.json').read_text()))
 compact=json.loads((root/'compact-manifest.json').read_text());compact.pop('directory',None);write(dest/'compact-manifest.json',compact)
 # Compact queue for a future person to inspect. Model disagreement is a
 # reason to review, never an automatic change to the frozen annotation.
 rows={r['id']:r for r in read(root/'final-audited.jsonl')};audit=json.loads((root/'posthoc-review.json').read_text());queue=[]
 for item in audit['results']['v5-selected']['review_items']:
  r=rows[item['id']];queue.append(dict(item,url=r['url'],source_sha256=r['source_sha256'],unit_offset=r['source_offset'],review_status='awaiting-independent-human-review'))
 for identity,reason in [('final-48','cross-game coreference'),('final-17','joint result ownership'),('final-29','joint tentative improvement'),('final-62','required versus actually performed deletion')]:
  r=rows[identity];queue.append(dict(id=identity,reason=reason,url=r['url'],source_sha256=r['source_sha256'],unit_offset=r['source_offset'],events=r['events'],review_status='awaiting-independent-human-review'))
 write(dest/'human-review-queue.json',queue)
 files={str(p.relative_to(HERE)):hashlib.sha256(p.read_bytes()).hexdigest() for p in HERE.glob('*.py')}
 write(dest/'code-manifest.json',files)
 print('Published source-free snapshots; models/bodies remain outside Git')
if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('root',type=Path);publish(p.parse_args().root)
