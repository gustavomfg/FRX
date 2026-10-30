"""One sealed prospective cohort, labels locked before any predictions."""
import argparse,json,hashlib
from pathlib import Path
from datetime import datetime,timezone
import numpy as np
from common import read,write,prior4
from seal import verify,sha
from syntax import Syntax
from compact import CompactSyntax
from events import extract
from syntax_control import extract as control
from relations import reconstruct
from assess import evaluate

def locked_rows(root):
 rows=read(root/'final-audited.jsonl');source=read(root/'final-reserved.jsonl')
 if [r['id'] for r in rows]!=[r['id'] for r in source]:raise ValueError('complete reserved cohort required')
 known=read(root/'train-known.jsonl')+read(root/'dev-known.jsonl')+read(root/'extra-audited.jsonl')
 if {r['app_id'] for r in rows}&{r['app_id'] for r in known}:raise ValueError('game leakage')
 for r,s in zip(rows,source):
  if r['text']!=s['text'] or r['source_sha256']!=s['source_sha256']:raise ValueError('changed source')
  prior4('data').validate(r)
 return rows
def lock(root):
 verify(root)
 if (root/'annotation-lock.json').exists() or (root/'final-results.json').exists():raise ValueError('preserve label lock')
 rows=locked_rows(root);write(root/'annotation-lock.json',dict(sha256=sha(root/'final-audited.jsonl'),locked_at=datetime.now(timezone.utc).isoformat(),rows=len(rows),predictions_seen=False,review='agent-provisional; no independent human review'))
def wilson(k,n):
 if not n:return None
 z=1.96;p=k/n;d=1+z*z/n;c=(p+z*z/(2*n))/d;h=z*np.sqrt(p*(1-p)/n+z*z/(4*n*n))/d;return [float(c-h),float(c+h)]
def run(root,old4):
 verify(root);dest=root/'final-results.json'
 if dest.exists() or (root/'final-predictions.json').exists():raise ValueError('preserve first final evaluation')
 locked=json.loads((root/'annotation-lock.json').read_text())
 if sha(root/'final-audited.jsonl')!=locked['sha256']:raise ValueError('labels changed after lock')
 rows=locked_rows(root);variants={};parser=Syntax(root);small=CompactSyntax(root)
 for name,p,fn in [('v5-selected',parser,extract),('syntax-initial-control',parser,control),('compact-challenger',small,extract)]:
  variants[name]=[reconstruct(r['text'],parsed,fn(r['text'],parsed)) for r in rows for parsed in [p.parse(r['text'])]]
 runtime=prior4('native').NativeEvents(old4/'runtime-events');variants['v4-verified']=runtime.predict(rows)
 variants['v4-lexical']=[dict(events=prior4('proof').lexical(r['text']),observations=[]) for r in rows]
 metrics={k:evaluate(rows,v) for k,v in variants.items()};apps=sorted({r['app_id'] for r in rows});per_game={}
 for app in apps:
  ids=[i for i,r in enumerate(rows) if r['app_id']==app];per_game[app]={k:evaluate([rows[i] for i in ids],[v[i] for i in ids]) for k,v in variants.items()}
 rng=np.random.default_rng(505);differences=[]
 for _ in range(4000):
  picked=rng.choice(apps,len(apps),replace=True);gold=sum(per_game[a]['v5-selected']['gold_performed'] for a in picked)
  differences.append(sum(per_game[a]['v5-selected']['performed_correct']-per_game[a]['v4-verified']['performed_correct'] for a in picked)/max(1,gold))
 result=dict(status='first-v5-new-game-evaluation',games=apps,paragraphs=len(rows),metrics=metrics,per_game=per_game,performed_precision_wilson_descriptive={k:wilson(m['performed_correct'],m['performed_claims']) for k,m in metrics.items()},paired_game_cluster_recall_gain_interval=np.quantile(differences,[.025,.975]).tolist(),annotation_sha256=locked['sha256'],automatic_approval=False,limitations=['Provisional research-agent labels; not independent human validation.','Event-enriched paragraphs, not population prevalence.','Six threads/games, confidence intervals are limited.','Result association is source-reported and experimental; not causal or local evidence.','Compact challenger cannot replace preselected model on these final numbers.'])
 with dest.open('x') as f:json.dump(result,f,indent=2)
 write(root/'final-predictions.json',variants)
 print(json.dumps({k:{a:v for a,v in m.items() if a!='details'} for k,m in metrics.items()},indent=2))
if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('root',type=Path);p.add_argument('old4',type=Path);p.add_argument('--lock-labels',action='store_true');a=p.parse_args();lock(a.root) if a.lock_labels else run(a.root,a.old4)
