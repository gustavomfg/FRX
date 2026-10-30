"""One prospective evaluation. No model selection or final-label tuning."""
import argparse,json,copy
from pathlib import Path
from datetime import datetime,timezone
import numpy as np
from bridge import read,write,sha,prior4,CompactSyntax,Syntax,prior_events,prior_reconstruct
from classifier import Classifier
from engine import Interpreter
from evaluate import evaluate
from seal import verify
def source_rows(root):return read(root/'acceptance-reserved.jsonl')+read(root/'identity-challenge-reserved.jsonl')+read(root/'context-challenge-reserved.jsonl')
def locked_rows(root):
 rows=read(root/'acceptance-gold.jsonl');original=source_rows(root)
 if [r['id'] for r in rows]!=[r['id'] for r in original]:raise ValueError('complete unchanged reserved cohort required')
 forbidden={r['app_id'] for r in read(root/'development-gold-v2.jsonl')}
 for r,s in zip(rows,original):
  if r['text']!=s['text'] or r['source_sha256']!=s['source_sha256'] or r['app_id'] in forbidden:raise ValueError('source/split mismatch')
  prior4('data').validate(r)
 return rows
def lock(root):
 verify(root)
 if (root/'label-lock.json').exists() or (root/'final-results.json').exists():raise ValueError('preserve label lock')
 rows=locked_rows(root);write(root/'label-lock.json',dict(sha256=sha(root/'acceptance-gold.jsonl'),locked_at=datetime.now(timezone.utc).isoformat(),units=len(rows),predictions_seen=False,provisional_agent_labels=True,human_review=False))
def wilson(k,n):
 if not n:return None
 z=1.96;p=k/n;d=1+z*z/n;c=(p+z*z/(2*n))/d;h=z*np.sqrt(p*(1-p)/n+z*z/(4*n*n))/d;return [float(c-h),float(c+h)]
def run(root):
 verify(root);dest=root/'final-results.json'
 if dest.exists() or (root/'final-predictions.json').exists():raise ValueError('preserve first final evaluation')
 label=json.loads((root/'label-lock.json').read_text())
 if sha(root/'acceptance-gold.jsonl')!=label['sha256']:raise ValueError('labels changed after lock')
 rows=locked_rows(root);catalog=json.loads((root/'catalog.json').read_text());selection=json.loads((root/'selection.json').read_text());small=CompactSyntax(root);large=Syntax(root);models={'v6-selected':Interpreter(small,catalog,Classifier(root/selection['model']),selection['threshold'],mode='trained-structured-verifier'),'v6-structural':Interpreter(small,catalog,threshold=.5),'v6-nearest':Interpreter(small,catalog,threshold=-1.001,mode='nearest')};preds={}
 for name,engine in models.items():preds[name]=[engine.analyze(r) for r in rows]
 for name,parser in [('v5-stanza',large),('v5-compact',small)]:preds[name]=[prior_reconstruct(r['text'],p,prior_events(r['text'],p)) for r in rows for p in [parser.parse(r['text'])]]
 # Clearly separate this erroneous context-only rule from actual v5: public
 # v5 left every event game/actor unknown. This control measures that tradeoff.
 context=copy.deepcopy(preds['v5-stanza'])
 for row,p in zip(rows,context):
  for e in p['events']:e['identity']=dict(game_app_id=row['app_id'],actor_id=row['reporter_id'],actor_kind='reporter')
 preds['thread-context-rule']=context
 groups={'main':[i for i,r in enumerate(rows) if r['id'].startswith('acceptance-')],'identity-challenge':[i for i,r in enumerate(rows) if not r['id'].startswith('acceptance-')],'all':list(range(len(rows)))};results={}
 for group,ids in groups.items():results[group]={name:evaluate([rows[i] for i in ids],[pred[i] for i in ids]) for name,pred in preds.items()}
 main=groups['main'];apps=sorted({rows[i]['app_id'] for i in main});per_game={}
 for app in apps:
  ids=[i for i in main if rows[i]['app_id']==app];per_game[app]={name:evaluate([rows[i] for i in ids],[pred[i] for i in ids]) for name,pred in preds.items()}
 rng=np.random.default_rng(606);deltas=[]
 for _ in range(4000):
  sample=rng.choice(apps,len(apps),replace=True);gold=sum(per_game[a]['v6-selected']['gold_relations'] for a in sample);deltas.append(sum(per_game[a]['v6-selected']['relation_correct']-per_game[a]['v5-stanza']['relation_correct'] for a in sample)/max(1,gold))
 v=results['main']['v6-selected'];out=dict(status='first-v6-new-discussion-evaluation; agent-provisional labels',results=results,per_game=per_game,games=apps,main_units=len(main),challenge_units=len(groups['identity-challenge']),relation_recall_gain_game_cluster_interval=np.quantile(deltas,[.025,.975]).tolist(),descriptive_performed_precision_wilson=wilson(v['performed_correct'],v['performed']),descriptive_relation_precision_wilson=wilson(v['relation_correct'],v['relations']),label_sha256=label['sha256'],automatic_approval=False,limitations=['No independent human annotation validation.','Six source games/threads; quote copies are dependent, not new confirmations.','Main cohort is event-enriched; challenge is explicitly context-enriched.','Game-unknown events are not assigned the thread AppID.','Reported association is not causal evidence or a local benchmark.','No selection or tuning is allowed after this evaluation.'])
 with dest.open('x') as f:json.dump(out,f,indent=2)
 write(root/'final-predictions.json',preds);print(json.dumps({g:{k:{a:v for a,v in m.items() if a!='details'} for k,m in ms.items()} for g,ms in results.items()},indent=2))
if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('root',type=Path);p.add_argument('--lock-labels',action='store_true');a=p.parse_args();lock(a.root) if a.lock_labels else run(a.root)
