"""Evaluate the preserved Rust rules on the same held-out action proposals.

Only variable targets are supported by that extractor; other procedures count
as abstentions. The application is neither built nor executed.
"""
import argparse
import json
import pathlib
import subprocess
import numpy as np
from representation import ENV,OUTCOMES,DIRECTIONS,MODALITIES
from train import read
from metrics import evaluate


def main(root,binary):
    if not (root/'frozen.json').exists():raise ValueError('freeze before final baseline evaluation')
    out=root/'rust-baseline-final.json'
    if out.exists():raise ValueError('preserve baseline evaluation')
    rows=read(root/'final-events.jsonl');payload=[];targets=[]
    for row in rows:
        a,b=row['action_span'];found=ENV.search(row['text'][a:b]);target=found.group() if found else '<unsupported-procedure>'
        targets.append(target);payload.append(dict(text=row['text'],target=target))
    result=subprocess.run([str(binary)],input=''.join(json.dumps(r)+'\n' for r in payload),capture_output=True,text=True,check=True,timeout=60)
    predictions=[json.loads(x) for x in result.stdout.splitlines()]
    logits=[np.full((len(rows),n),-20.) for n in [5,10,3,2]]
    for i,p in enumerate(predictions):
        fields=[0,0,0,0]
        if p:
            fields=[OUTCOMES.index(p['outcome']),DIRECTIONS.index('disable' if p['disabled'] else 'enable'),2,int(bool(p['combinedWith']))]
        for j,value in enumerate(fields):logits[j][i,value]=20.
    summary=dict(variable_rows=sum(x!='<unsupported-procedure>' for x in targets),
        full=evaluate(rows,logits,.7),scope='Original Rust variable extractor; unsupported procedures abstain.')
    out.write_text(json.dumps(summary,indent=2));print(summary['variable_rows'],summary['full']['accepted'],summary['full']['correct'])


if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('root',type=pathlib.Path);p.add_argument('binary',type=pathlib.Path);a=p.parse_args();main(a.root,a.binary)
