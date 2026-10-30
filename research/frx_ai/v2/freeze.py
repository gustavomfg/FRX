"""Calibrate a DEV candidate on a separate game, then seal final input.

No final body or final label file is loaded by this module.
"""
import argparse
import hashlib
import json
import pathlib
import time
import numpy as np
from scipy.optimize import minimize_scalar
from scipy.special import softmax
from model import labels
from train import read
from metrics import evaluate,risk_curve
from runtime import Runtime


def calibrate(root,goal=False):
    if (root/'frozen.json').exists():raise ValueError('already frozen')
    directory=root/('runtime-goal' if goal else 'runtime-relation')
    runtime=Runtime(directory);dev=read(root/('dev-goals.jsonl' if goal else 'dev-events.jsonl'));cal=read(root/('calibration-goals.jsonl' if goal else 'calibration-events.jsonl'))
    raw,_=runtime.classify(cal);gold=labels(cal).numpy();temperatures=[]
    for i,z in enumerate(raw):
        f=lambda t:float(-np.log(softmax(z/t,axis=1)[np.arange(len(cal)),gold[:,i]].clip(1e-9)).mean())
        fit=minimize_scalar(f,bounds=(.3,5.),method='bounded')
        temperatures.append(float(fit.x))
    # The evaluator takes one temperature; scale logits per head first instead.
    cal_scaled=[z/t for z,t in zip(raw,temperatures)]
    dv,_=runtime.classify(dev);scaled=[z/t for z,t in zip(dv,temperatures)]
    curve=risk_curve(dev,scaled)
    eligible=[(float(t),m) for t,m in curve.items() if m['accepted']>=10 and m['coverage']>=.2 and (m['precision'] or 0)>=.9 and m['severe_inversions']==0 and m['false_success']==0]
    threshold=min(eligible,key=lambda x:x[0])[0] if eligible else .7
    probabilities=softmax(cal_scaled[0],axis=1)
    nonconformity=1-probabilities[np.arange(len(cal)),gold[:,0]]
    k=min(len(cal),int(np.ceil((len(cal)+1)*.9)))
    q=float(np.sort(nonconformity)[k-1])
    meta=runtime.meta;meta.update(temperatures=temperatures,threshold=threshold,
        automatic_interpretation_approved=False,conformal_q=q)
    (directory/'runtime.json').write_text(json.dumps(meta,indent=2))
    summary=dict(calibration_n=len(cal),calibration_games=sorted({r['app_id'] for r in cal}),
        temperatures=temperatures,threshold=threshold,development_eligible=bool(eligible),
        raw_calibration=evaluate(cal,raw),scaled_calibration=evaluate(cal,cal_scaled),
        development_curve=curve,conformal_q=q,
        caveat='Small clustered calibration data; no exchangeability or conditional precision guarantee.')
    (root/('calibration-goal.json' if goal else 'calibration.json')).write_text(json.dumps(summary,indent=2))
    print('calibration',len(cal),'development eligible',bool(eligible),'threshold',threshold)


def seal(root,retrieval_name=None,alpha=.5):
    dest=root/'frozen.json'
    if dest.exists():raise ValueError('preserve first freeze')
    meta=json.loads((root/'runtime-relation/runtime.json').read_text())
    manifest={}
    for name in ['train-events.jsonl','dev-events.jsonl','calibration-events.jsonl','final-candidates.jsonl','calibration.json']:
        manifest[name]=hashlib.sha256((root/name).read_bytes()).hexdigest()
    code={p.name:hashlib.sha256(p.read_bytes()).hexdigest() for p in pathlib.Path(__file__).parent.glob('*.py')}
    frozen=dict(created_utc=time.strftime('%Y-%m-%dT%H:%M:%SZ',time.gmtime()),model=meta,
        files=manifest,code_sha256=code,retrieval_name=retrieval_name,retrieval_alpha=alpha,
        final_labels_seen=False,threshold_policy='dev >=90% with >=10 accepted and >=20% coverage; fallback .7, never automatic approval')
    if retrieval_name:
        f=root/'retriever-runs'/(retrieval_name+'.pt')
        frozen['retrieval_sha256']=hashlib.sha256(f.read_bytes()).hexdigest()
    if (root/'runtime-goal/runtime.json').exists():
        frozen['goal_model']=json.loads((root/'runtime-goal/runtime.json').read_text())
        for name in ['train-goals.jsonl','dev-goals.jsonl','calibration-goals.jsonl','final-goal-candidates.jsonl','calibration-goal.json']:
            frozen['files'][name]=hashlib.sha256((root/name).read_bytes()).hexdigest()
    if (root/'runtime-retrieval/runtime.json').exists():
        frozen['retrieval_runtime']=json.loads((root/'runtime-retrieval/runtime.json').read_text())
    with dest.open('x') as f:json.dump(frozen,f,indent=2)
    print('sealed',meta['model_name'])


if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('root',type=pathlib.Path);p.add_argument('--seal',action='store_true');p.add_argument('--goal',action='store_true');p.add_argument('--retrieval-name');p.add_argument('--alpha',type=float,default=.5)
    a=p.parse_args()
    if a.seal:seal(a.root,a.retrieval_name,a.alpha)
    else:calibrate(a.root,a.goal)
