"""Analysis of the locked evaluation, never model selection or relabeling."""
import argparse,json,collections,copy
from pathlib import Path
from bridge import read,write,prior4
from evaluate import mapping,overlap,evaluate

def analyze(root):
 rows=read(root/'acceptance-gold.jsonl');preds=json.loads((root/'final-predictions.json').read_text());records=[];summary={}
 for name,outputs in preds.items():
  states=collections.Counter();directions=collections.Counter();failures=[];joint_gold=0;negative_gold=0;proposal_status=collections.Counter()
  for row,pred in zip(rows,outputs):
   links=mapping(row,pred);local=[]
   for i,e in enumerate(pred['events']):
    g=row['events'][links[i]] if i in links else None
    if g:
     states[g['status'],e['status']]+=1;directions[g['direction'],e['direction']]+=1
    if e['status']=='performed' and (not g or g['status']!='performed'):local.append(dict(kind='false-execution',trigger=e['trigger'],gold=g))
    identity=e.get('identity',{})
    if identity.get('actor_id') is not None and (not g or identity['actor_id']!=g.get('actor_id')):local.append(dict(kind='unsupported-reporter',trigger=e['trigger'],identity=identity,gold=g))
    if identity.get('game_app_id') is not None and (not g or identity['game_app_id']!=g.get('game_app_id')):local.append(dict(kind='unsupported-game',trigger=e['trigger'],identity=identity,gold=g))
   used=set()
   for o in pred.get('observations',[]):
    owners=sorted(links.get(i,-1) for i in o['event_indices']);matches=[j for j,g in enumerate(row.get('observations',[])) if j not in used and g['event_indices'] and overlap(o['evidence']['span'],g['span'])>0 and owners==sorted(g['event_indices']) and o['outcome']==g['outcome']]
    if matches:used.add(matches[0])
    else:local.append(dict(kind='false-success-attribution' if o['outcome']=='improved' else 'unsupported-result-association',prediction=o,mapped_owners=owners,overlapping_gold=[g for g in row['observations'] if overlap(o['evidence']['span'],g['span'])]))
   for j,g in enumerate(row['observations']):
    if not g['event_indices'] or g['outcome']=='unknown':continue
    joint_gold+=len(g['event_indices'])>1;negative_gold+=g['outcome'] in ['persistent','regression']
    if j not in used:
     statuses=sorted({row['events'][i]['status'] for i in g['event_indices']});proposal_status[','.join(statuses)]+=1
     local.append(dict(kind='missed-relation',gold=g,owner_statuses=statuses))
   if local:failures.append(dict(id=row['id'],url=row['url'],text=row['text'],failures=local))
  confusion=lambda counts:[dict(gold=a,prediction=b,count=n) for (a,b),n in sorted(counts.items())]
  summary[name]=dict(status_confusion=confusion(states),direction_confusion=confusion(directions),gold_joint_relations=joint_gold,gold_negative_relations=negative_gold,missed_owner_statuses=dict(proposal_status),failed_units=len(failures))
  records.append(dict(model=name,errors=failures))
 write(root/'posthoc-errors.json',dict(status='post-hoc error analysis; no fitting, selection or annotation changes',records=records))
 groups={'exact-name-2':list(range(72,74)),'context-22':list(range(74,96))}
 curve=[]
 for threshold in [.3,.4,.5,.6,.7,.8,.9]:
  outputs=copy.deepcopy(preds['v6-selected'])
  for out in outputs:out['observations']=[o for o in out['observations'] if o.get('score',0)>=threshold]
  m=evaluate(rows,outputs);curve.append(dict(threshold=threshold,accepted=m['relations'],correct=m['relation_correct'],false_success=m['false_success'],precision=m['relation_precision'],recall=m['relation_recall']))
 write(root/'posthoc-metrics.json',dict(summary=summary,separate_challenges={g:{name:evaluate([rows[i] for i in ids],[out[i] for i in ids]) for name,out in preds.items()} for g,ids in groups.items()},risk_coverage=dict(status='descriptive after final evaluation; no threshold selected or deployed',points=curve),temporal_evaluation='No exhaustively annotated temporal edges in the acceptance set; no temporal competence conclusion.',calibration='No independent calibration set and too few accepted links; raw scores remain uncalibrated.'))
 print({k:v['failed_units'] for k,v in summary.items()})
if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('root',type=Path);analyze(p.parse_args().root)
