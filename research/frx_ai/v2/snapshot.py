"""Publish small reproducibility metadata; never publish original comments/weights."""
import argparse
import hashlib
import json
import pathlib
import shutil
from budget import spent
from data import model_input_digest


def read(path):return [json.loads(x) for x in path.read_text().splitlines()]


def main(root):
    dest=pathlib.Path(__file__).parent/'snapshot';dest.mkdir(exist_ok=True)
    records=[];configs={}
    for folder in ['runs','baselines','active','retriever-runs']:
        for file in sorted((root/folder).glob('*.json')):
            x=json.loads(file.read_text())
            if not isinstance(x,dict) or 'seconds' not in x:continue
            name=x.get('name',x.get('config',{}).get('name',file.stem))
            row=dict(name=name,family=folder,seconds=x['seconds'])
            if 'curve' in x:
                m=x['curve']['0.0'];row.update(relation_macro_f1=m['relation_macro_f1'],exact_accuracy=m['exact_all_accuracy'],
                    selective=x['curve']['0.7'],train_hash=x.get('train_hash'),dev_hash=x.get('dev_hash'))
            if 'selection_score' in x:row['selection_score']=x['selection_score']
            records.append(row)
            if 'config' in x:
                c=dict(x['config']);n=int(name[1:3])
                if n<=4:c['train_file']='train-stage1.jsonl'
                elif n<=10:c['train_file']='train-stage2.jsonl'
                elif n<=17:c['train_file']='train-stage3.jsonl'
                configs[name]=c
    (dest/'experiments.json').write_text(json.dumps(dict(completed_fits=len(records),timed_seconds=spent(root),records=records),indent=2))
    (dest/'experiment-configs.json').write_text(json.dumps(configs,indent=2))
    for name in ['pretrained-manifest.json','frozen.json','final-reservation.json','goal-reservation.json',
                 'calibration.json','calibration-goal.json','final-results-v2.json','final-goal-results-v2.json',
                 'rust-baseline-final.json','export-comparison.json','export-goal-comparison.json',
                 'retrieval-export-comparison.json','primary-selection.json','retrieval-final-v2.json',
                 'benchmark-runtime-relation.json','benchmark-runtime-goal.json']:
        if (root/name).exists():shutil.copyfile(root/name,dest/name)
    shutil.copyfile(root/'active/human-review-queue.json',dest/'human-review-queue.json')
    stages={1:read(root/'train-round1.jsonl'),2:read(root/'train-round2.jsonl'),3:read(root/'train-events.jsonl')}
    added={}
    for stage,rows in stages.items():
        for row in rows:added.setdefault(row['id'],stage)
        (root/f'train-stage{stage}.jsonl').write_text(''.join(json.dumps(r)+'\n' for r in rows))
    semantic={f'train-stage{stage}.jsonl':model_input_digest(rows) for stage,rows in stages.items()}
    for name in ['train-goals.jsonl','dev-events.jsonl','dev-goals.jsonl','calibration-events.jsonl','calibration-goals.jsonl']:
        semantic[name]=model_input_digest(read(root/name))
    (dest/'model-input-manifest.json').write_text(json.dumps(semantic,indent=2))
    topic_map=json.loads((root/'topic-map.json').read_text())
    bodies=root/'bodies';bodies.mkdir(exist_ok=True)
    for goals in [False,True]:
        annotations=[]
        for split in ['train','dev','calibration','final']+([] if goals else ['weak']):
            name=split+('-goals.jsonl' if goals else '-events.jsonl')
            if split=='final' and goals:name='final-goal-events.jsonl'
            if not (root/name).exists():continue
            for row in read(root/name):
                body=row['text'];sha=hashlib.sha256(body.encode()).hexdigest();(bodies/(sha+'.txt')).write_bytes(body.encode())
                ann={k:v for k,v in row.items() if k!='text'}
                ann.update(text_sha256=sha,split=split,added_stage=added.get(row['id'],3),
                    annotation_review='provisional agent annotation; no independent human review',
                    topics=row.get('topics',topic_map.get(str(row.get('legacy_index')),[])))
                annotations.append(ann)
        filename='goal-annotations.json' if goals else 'annotations.json'
        (dest/filename).write_text(json.dumps(annotations,indent=2,ensure_ascii=False))
    manifest=dict(files={p.name:hashlib.sha256(p.read_bytes()).hexdigest() for p in dest.glob('*.json')},
                  raw_comments_in_git=False,checkpoints_in_git=False,final_labels_provisional=True)
    (dest/'manifest.json').write_text(json.dumps(manifest,indent=2))
    print('snapshot',len(records),'fits; no raw source bodies or model weights')


if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('root',type=pathlib.Path);main(p.parse_args().root)
