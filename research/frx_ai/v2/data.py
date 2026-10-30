"""Reconstruct span datasets from small annotations and original public sources.

Raw comments stay outside Git. Changed/deleted source bodies require the
original cache; hashes must match, never silently relabel a revised document.
"""
import argparse
import hashlib
import json
import pathlib
import re
import subprocess
from contracts import checked_span


def model_input_digest(rows):
    keys=['id','app_id','url','text','action_span','candidate_result_span','outcome','direction','modality','joint','origin']
    payload=[{k:r.get(k) for k in keys}|{'edge_weight':r.get('edge_weight',1.)} for r in rows]
    return hashlib.sha256(json.dumps(payload,sort_keys=True,ensure_ascii=False).encode()).hexdigest()


def validate(splits):
    apps={};urls={};texts={}
    for name,rows in splits.items():
        partition='train' if name=='weak' else name
        for row in rows:
            checked_span(row['text'],row['action_span'])
            if row.get('result_span'):checked_span(row['text'],row['result_span'])
            if row['origin']=='real-public-github' and row['outcome']!='unknown' and not row.get('result_span'):
                raise ValueError('a manual outcome needs literal supporting evidence')
            app=row['app_id'];url=row['url'];digest=hashlib.sha256(' '.join(row['text'].split()).encode()).hexdigest()
            for value,seen in [(app,apps),(url,urls),(digest,texts)]:
                if value in seen and seen[value]!=partition:raise ValueError('cross-split overlap: '+str(value))
                seen[value]=partition


def hydrate(annotation_file,root,fetch=False,goals=False):
    annotations=json.loads(annotation_file.read_text());cache=root/'bodies';cache.mkdir(parents=True,exist_ok=True)
    splits={}
    requests=0
    for ann in annotations:
        digest=ann['text_sha256'];f=cache/(digest+'.txt')
        if not f.exists():
            if not fetch:raise FileNotFoundError('missing original source cache; explicit --fetch is required')
            url=ann['url'];m=re.fullmatch(r'https://github.com/([\w.-]+)/([\w.-]+)/issues/\d+#issuecomment-(\d+)',url)
            if not m:raise ValueError('unsupported public source identity')
            requests+=1
            if requests>400:raise ValueError('public request budget exceeded')
            raw=subprocess.check_output(['gh','api',f'repos/{m[1]}/{m[2]}/issues/comments/{m[3]}'],timeout=60)
            if len(raw)>2_000_000:raise ValueError('public response size limit')
            body=json.loads(raw)['body']
            if hashlib.sha256(body.encode()).hexdigest()!=digest:raise ValueError('source changed; obtain original cache, do not reuse old spans')
            f.write_bytes(body.encode())
        body=f.read_bytes().decode('utf-8')
        if hashlib.sha256(body.encode()).hexdigest()!=digest:raise ValueError('cached body integrity mismatch')
        row=dict(ann,text=body);splits.setdefault(ann['split'],[]).append(row)
    validate(splits)
    for name,rows in splits.items():
        filename=name+'-goals.jsonl' if goals else name+'-events.jsonl'
        (root/filename).write_text(''.join(json.dumps(r)+'\n' for r in rows))
    for stage in ([] if goals else [1,2,3]):
        rows=[r for r in splits['train'] if r['added_stage']<=stage]
        (root/f'train-stage{stage}.jsonl').write_text(''.join(json.dumps(r)+'\n' for r in rows))
    if not goals:
        for split in ['train','dev']:
            unique={}
            for row in splits.get(split,[]):
                if row['url'] not in unique:unique[row['url']]=dict(row,topics=[])
                unique[row['url']]['topics']=sorted(set(unique[row['url']]['topics'])|set(row.get('topics',[])))
            (root/('retrieval-'+split+'.jsonl')).write_text(''.join(json.dumps(r)+'\n' for r in unique.values()))
    print({name:len(rows) for name,rows in splits.items()})


if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('annotations',type=pathlib.Path);p.add_argument('root',type=pathlib.Path);p.add_argument('--fetch',action='store_true');p.add_argument('--goals',action='store_true')
    a=p.parse_args();hydrate(a.annotations,a.root,a.fetch,a.goals)
