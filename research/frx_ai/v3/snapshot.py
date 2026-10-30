"""Publish small metadata only; source bodies and all learned artifacts stay out."""
import argparse
from pathlib import Path
import json
import hashlib
import shutil
from common import read,write,sha,HERE

def compact(value):
 if isinstance(value,dict):return {k:compact(v) for k,v in value.items() if k!='records'}
 if isinstance(value,list):return [compact(v) for v in value]
 return value

def run(root):
 dest=HERE/'snapshot';dest.mkdir(exist_ok=True);records=[]
 for folder in ['events','ranking','simulation','structure']:
  records+=json.loads((root/folder/'experiments.json').read_text())
 write(dest/'experiments.json',dict(configurations=len(records),timed_stage_seconds=sum(r['seconds'] for r in records),
  timing_scope='Recorded fitting/evaluation stages only; embedding generation, source annotation and later evaluation excluded',records=records))
 for name in ['frozen.json','reservation.json','annotation-provenance.json','final-results.json','preserved-v2-uncertainty.json',
              'diagnostic-study.json','benchmark-review.json','benchmark-review-first.json','benchmark-cold.json','restart-validation.json']:
  shutil.copyfile(root/name,dest/name)
 for folder,name in [('events','selection'),('ranking','selection'),('ranking','development'),('structure','frozen'),('structure','final'),('structure','development'),('simulation','assumptions')]:
  write(dest/(folder+'-'+name+'.json'),compact(json.loads((root/folder/(name+'.json')).read_text())))
 for name in ['pooled-a1','context-a1','context-a5']:
  write(dest/('simulation-'+name+'.json'),compact(json.loads((root/'simulation'/(name+'.json')).read_text())))
 annotations=json.loads((root/'final-annotations.json').read_text());source={r['id']:r for r in read(root/'final-candidates.jsonl')};items=[]
 bodies=root/'bodies';bodies.mkdir(exist_ok=True)
 for ann in annotations:
  row=source[ann['id']];body=row['text'];digest=hashlib.sha256(body.encode()).hexdigest();(bodies/(digest+'.txt')).write_bytes(body.encode())
  items.append(ann|{k:row[k] for k in ['url','app_id','thread','title','body_sha256','action_span','observation_span']})
 write(dest/'final-annotations.json',items)
 blind=[{k:row[k] for k in ['id','url','app_id','body_sha256','action_span','observation_span']} for row in items if row['index'] in [15,20,35,51,63,68,69,75,77,86,91]]
 write(dest/'human-review-queue.json',dict(review_completed=False,labels_hidden=True,items=blind))
 for source in json.loads((root/'historical-proposed.json').read_text()):
  digest=hashlib.sha256(source['text'].encode()).hexdigest();(bodies/(digest+'.txt')).write_bytes(source['text'].encode())
 frozen=json.loads((root/'frozen.json').read_text());write(dest/'post-final-assembly.json',dict(
  files_added_after_language_retrieval_freeze=sorted(p.name for p in HERE.glob('*.py') if p.name not in frozen['code']),
  explanation='Conservative baseline wrapper, local memory, separate synthetic-structure experiment and reproduction/reporting tools; no alteration of frozen language/retrieval code, selected challenger or first final results. Whole assembled diagnosis workflow is not independently validated.'))
 artifact_paths=list((root/'events').glob('*.joblib'))+[root/'structure/models.json'];write(dest/'artifact-manifest.json',[
  dict(file=str(p.relative_to(root)),bytes=p.stat().st_size,sha256=sha(p),in_git=False) for p in artifact_paths])
 manifest=dict(files={p.name:sha(p) for p in sorted(dest.glob('*.json')) if p.name!='manifest.json'},raw_source_text_in_git=False,learned_weights_in_git=False,all_labels_provisional=True)
 write(dest/'manifest.json',manifest);print('Snapshot:',len(records),'configurations,',len(items),'annotation metadata records; no raw texts or weights')

if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('root',type=Path);run(p.parse_args().root)
