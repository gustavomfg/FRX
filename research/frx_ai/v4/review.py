"""Post-test descriptive audit, not model selection or relabeling."""
import argparse,collections,hashlib,json
from pathlib import Path
from data import read,write
from metrics import evaluate,overlap
from proof import attach_observations

def matches(gold,pred):
 used=set();out={}
 for i,e in enumerate(pred['events']):
  a=e['trigger']['span'];choices=[]
  for j,g in enumerate(gold['events']):
   b=g['trigger'];iou=overlap(a,b)/(max(a[1],b[1])-min(a[0],b[0]))
   if j not in used:choices.append((iou,j))
  score,j=max(choices,default=(0,-1))
  if score>=.5:out[i]=j;used.add(j)
 return out

def run(root):
 rows=read(root/'final-audited.jsonl');pred=json.loads((root/'final-predictions.json').read_text())['native_verified'];detail=[];confusion=collections.Counter();direction_errors=0;literal_errors=0;identities=0
 for g,p in zip(rows,pred):
  links=matches(g,p)
  for i,e in enumerate(p['events']):
   state=g['events'][links[i]]['status'] if i in links else 'no-gold-event';confusion[state,e['status']]+=1
   if i in links:direction_errors+=e['direction']!=g['events'][links[i]]['direction']
   for key in ['trigger','status_evidence','target_evidence']:
    s=e.get(key)
    if s and g['text'][slice(*s['span'])]!=s['text']:literal_errors+=1
   if e.get('target_identity'):
    identities+=1;literal_errors+=e['target_identity'] not in g['text']
  m=evaluate([g],[p],strict=True);detail.append(dict(id=g['id'],app_id=g['app_id'],url=g['url'],gold_events=m['gold_events'],matched=m['matched'],performed_claims=m['performed_claims'],performed_correct=m['performed_correct'],false_performed=m['false_performed']))
 per_game={app:evaluate([r for r in rows if r['app_id']==app],[p for r,p in zip(rows,pred) if r['app_id']==app],strict=True) for app in sorted({r['app_id'] for r in rows})}
 # Diagnostic upper-bound control of the rule relation stage: gold events supplied,
 # not an end-to-end result or evidence of successful model detection.
 supplied=[]
 for row in rows:
  events=[dict(trigger={'span':e['trigger'],'text':row['text'][slice(*e['trigger'])]},status=e['status'],direction=e['direction']) for e in row['events']]
  supplied.append(dict(events=events,observations=attach_observations(row['text'],events)))
 reference_observations=sum(len(r['observations']) for r in rows)
 report=dict(posthoc=True,no_model_changes=True,per_game=per_game,status_confusion=[dict(reference=a,predicted=b,count=n) for (a,b),n in sorted(confusion.items())],direction_errors_on_matched=direction_errors,literal_integrity_errors=literal_errors,literal_identities=identities,
  observed_performed_recall=sum(d['performed_correct'] for d in detail)/sum(e['status']=='performed' for r in rows for e in r['events']),
  reference_observations=reference_observations,relation_rule_with_supplied_gold_events=evaluate(rows,supplied,strict=True),
  result_claims_default_disabled=True,documents=detail)
 write(root/'posthoc-review.json',report)
 # A blind human queue intentionally excludes provisional labels and predictions.
 queue=[]
 for r in rows:
  queue.append(dict(id=r['id'],url=r['url'],app_id=r['app_id'],source_sha256=r['source_sha256'],source_offset=r['source_offset'],paragraph_sha256=hashlib.sha256(r['text'].encode()).hexdigest(),paragraph_length=len(r['text']),questions=['Mark literal technical event triggers and their targets.','Was each action performed, suggested, denied, already present, or unknown?','Which result belongs to which action or joint procedure?','Is the described action corrective, diagnostic, or gameplay?'],independent_review=None))
 write(root/'human-review-queue.json',queue)
 print(json.dumps({k:v for k,v in report.items() if k not in ['per_game','documents']},indent=2))

if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('root',type=Path);run(p.parse_args().root)
