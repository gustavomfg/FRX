"""Event/claim metrics; unknowns do not become false successes."""
import numpy as np

def overlap(a,b):return max(0,min(a[1],b[1])-max(a[0],b[0]))

def evaluate(rows,predictions,strict=False):
 total_gold=0;total_pred=0;matched=0;status_correct=0;direction_correct=0;performed=0;performed_correct=0;false_performed=0;accepted=0;accepted_correct=0;exact=0;details=[]
 result_claims=0;correct_result=0;false_success=0
 for row,pred in zip(rows,predictions):
  gold=row['events'];events=pred['events'];total_gold+=len(gold);total_pred+=len(events);used=set();links={};local=dict(id=row['id'],app_id=row['app_id'],gold=len(gold),predicted=len(events),false_performed=0)
  for i,event in enumerate(events):
   s=event['trigger']['span']
   def quality(g):
    common=overlap(s,g['trigger'])
    return common/(max(s[1],g['trigger'][1])-min(s[0],g['trigger'][0])) if strict else common
   choices=[(quality(g),j) for j,g in enumerate(gold) if j not in used];best=max(choices,default=(0,-1));j=best[1] if best[0]>=(.5 if strict else 1) else -1
   same=False
   if j>=0:
    used.add(j);links[i]=j;matched+=1;g=gold[j];status_correct+=event['status']==g['status'];direction_correct+=event['direction']==g['direction'];exact+=s==g['trigger'];same=event['status']==g['status'] and event['direction']==g['direction']
   if event['status']!='unknown':accepted+=1;accepted_correct+=bool(j>=0 and same)
   if event['status']=='performed':
    performed+=1;correct=j>=0 and gold[j]['status']=='performed';performed_correct+=correct
    if not correct:false_performed+=1;local['false_performed']+=1
  for obs in pred.get('observations',[]):
   result_claims+=1;owners=sorted(links.get(i,-1) for i in obs['event_indices']);found=False
   for g in row.get('observations',[]):
    if overlap(obs['evidence']['span'],g['span']) and owners==sorted(g['event_indices']) and obs['outcome']==g['outcome']:found=True
   correct_result+=found
   if not found and obs['outcome']=='improved':false_success+=1
  details.append(local)
 return dict(match_criterion='character-IoU>=0.5' if strict else 'exploratory-any-overlap',documents=len(rows),gold_events=total_gold,predicted_events=total_pred,matched=matched,
  trigger_precision=matched/total_pred if total_pred else None,trigger_recall=matched/total_gold if total_gold else None,
  trigger_f1=2*matched/(total_pred+total_gold) if total_pred+total_gold else 0,
  exact_trigger_matches=exact,status_accuracy_on_matched=status_correct/matched if matched else None,
  direction_accuracy_on_matched=direction_correct/matched if matched else None,
  performed_claims=performed,performed_correct=performed_correct,performed_precision=performed_correct/performed if performed else None,
  false_performed=false_performed,accepted_status_claims=accepted,accepted_status_correct=accepted_correct,
  accepted_tuple_precision=accepted_correct/accepted if accepted else None,claim_coverage=accepted/total_gold if total_gold else 0,
  result_claims=result_claims,result_correct=correct_result,result_precision=correct_result/result_claims if result_claims else None,
  false_success=false_success,details=details)
