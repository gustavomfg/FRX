"""First held-out evaluation only. Does not train or change thresholds."""
import argparse
import hashlib
import json
import pathlib
import numpy as np
from scipy.special import softmax
from model import labels
from metrics import evaluate,risk_curve,wilson
from runtime import Runtime
from train import read


def component_metrics(rows,logits,threshold=.7):
    gold=labels(rows).numpy();result={}
    for j,name in enumerate(['outcome','direction','modality','grouping']):
        p=softmax(logits[j],axis=1);prediction=p.argmax(1)
        accepted=(p.max(1)>=threshold)
        if j<3:accepted&=prediction!=0
        n=int(accepted.sum());correct=int((accepted&(prediction==gold[:,j])).sum())
        result[name]=dict(accepted=n,correct=correct,precision=correct/n if n else None,
            coverage=n/len(rows),wilson=wilson(correct,n),
            confusion_between_suggestion_and_experience=int((accepted&(((prediction==1)&(gold[:,j]==2))|((prediction==2)&(gold[:,j]==1)))).sum()) if j==2 else None)
    return result


def bootstrap_games(rows,logits,threshold):
    apps=sorted({r['app_id'] for r in rows});rng=np.random.default_rng(919);values=[];empty=0
    for _ in range(2000):
        chosen=rng.choice(apps,len(apps),replace=True)
        ids=[i for app in chosen for i,r in enumerate(rows) if r['app_id']==app]
        # Avoid recomputing class reports thousands of times.
        gold=labels([rows[i] for i in ids]).numpy();p=[softmax(z[ids],axis=1) for z in logits]
        pred=np.stack([z.argmax(1) for z in p],1);conf=np.stack([z.max(1) for z in p[:3]],1).min(1)
        selected=(pred[:,0]!=0)&(pred[:,1]!=0)&(pred[:,2]==2)&(conf>=threshold)
        if not selected.any():empty+=1;continue
        values.append(float((pred[selected]==gold[selected]).all(1).mean()))
    return dict(interval=np.quantile(values,[.025,.975]).tolist() if values else None,
                games=len(apps),replicates=2000,undefined_replicates=empty)


def main(root,goal=False):
    result_file=root/('final-goal-results-v2.json' if goal else 'final-results-v2.json')
    if result_file.exists():raise ValueError('preserve first final evaluation')
    frozen=json.loads((root/'frozen.json').read_text())
    for name,digest in frozen['code_sha256'].items():
        if hashlib.sha256((pathlib.Path(__file__).parent/name).read_bytes()).hexdigest()!=digest:
            raise ValueError('frozen inference/evaluation code changed: '+name)
    for name,digest in frozen['files'].items():
        if hashlib.sha256((root/name).read_bytes()).hexdigest()!=digest:raise ValueError('frozen research input changed: '+name)
    runtime=Runtime(root/('runtime-goal' if goal else 'runtime-relation'))
    if runtime.meta!=frozen['goal_model' if goal else 'model']:raise ValueError('frozen model metadata changed')
    rows=read(root/('final-goal-events.jsonl' if goal else 'final-events.jsonl'))
    candidates=read(root/('final-goal-candidates.jsonl' if goal else 'final-candidates.jsonl'))
    assert [r['id'] for r in rows]==[r['id'] for r in candidates]
    assert all(r['text']==c['text'] and r['action_span']==c['action_span'] for r,c in zip(rows,candidates))
    if goal:assert all(r['candidate_result_span']==c['candidate_result_span'] for r,c in zip(rows,candidates))
    # Only the Runtime representation accesses model inputs; no gold result
    # span, outcome, state, modality or topic is consumed by inference.
    raw,witnesses=runtime.classify(rows)
    scaled=[z/t for z,t in zip(raw,runtime.meta['temperatures'])]
    main_metrics=evaluate(rows,scaled,runtime.meta['threshold'])
    p=softmax(scaled[0],axis=1);gold=labels(rows).numpy()
    sets=(1-p)<=runtime.meta['conformal_q']
    selected=sets.sum(1)==1
    conformal=dict(empirical_label_coverage=float(sets[np.arange(len(rows)),gold[:,0]].mean()),
        mean_set_size=float(sets.sum(1).mean()),singletons=int(selected.sum()),
        singleton_accuracy=float((p.argmax(1)[selected]==gold[selected,0]).mean()) if selected.any() else None,
        guarantee='No distribution-free guarantee claimed under game shift.')
    result=dict(status='first-new-game-held-out-test; provisional single-agent labels',
        n=len(rows),games=sorted({r['app_id'] for r in rows}),selected=main_metrics,
        raw=evaluate(rows,raw,runtime.meta['threshold']),curve=risk_curve(rows,scaled),
        components=component_metrics(rows,scaled),conformal=conformal,
        game_bootstrap=bootstrap_games(rows,scaled,runtime.meta['threshold']),
        model_name=runtime.meta['model_name'],
        gold_outcome_counts={name:sum(r['outcome']==name for r in rows) for name in ['unknown','positive','not-resolved','worsened','mixed']},
        witness_spans=witnesses,
        task='relation given proposed action/result spans' if goal else 'comment/action tuple classification',
        gold_sha256=hashlib.sha256((root/('final-goal-events.jsonl' if goal else 'final-events.jsonl')).read_bytes()).hexdigest())
    n=main_metrics['accepted'];precision=main_metrics['precision'] or 0
    result['quality_target_met']=bool(n>=30 and precision>=.9 and main_metrics['coverage']>=.2 and main_metrics['severe_inversions']==0 and main_metrics['false_success']==0 and sum(x['accepted']>0 for x in main_metrics['by_game'].values())>=4)
    with result_file.open('x') as f:json.dump(result,f,indent=2)
    np.savez(root/('final-goal-predictions.npz' if goal else 'final-predictions.npz'),**{f'head{i}':z for i,z in enumerate(raw)})
    print(json.dumps({k:result[k] for k in ['model_name','quality_target_met','selected','components']},indent=2))


if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('root',type=pathlib.Path);p.add_argument('--goal',action='store_true');a=p.parse_args();main(a.root,a.goal)
