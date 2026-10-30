"""Refutation audit, never a selector or a modification of locked labels."""
import argparse,json
from pathlib import Path
from common import read,write
from assess import legacy
def run(root):
 rows=read(root/'final-audited.jsonl');allpred=json.loads((root/'final-predictions.json').read_text());out={}
 for name,preds in allpred.items():
  records=[];correct_current=claims_current=joint_correct=joint_claims=0;other_game_correct=0;unique=set();literal_errors=0
  for row,pred in zip(rows,preds):
   mapping={};used=set()
   for i,e in enumerate(pred['events']):
    a,b=e['trigger']['span'];options=[(legacy.overlap([a,b],g['trigger'])/(max(b,g['trigger'][1])-min(a,g['trigger'][0])),j) for j,g in enumerate(row['events']) if j not in used];best=max(options,default=(0,-1));j=best[1] if best[0]>=.5 else -1
    if j>=0:mapping[i]=j;used.add(j)
    if e['trigger']['text']!=row['text'][a:b]:literal_errors+=1
    other=j>=0 and row['events'][j].get('scope','').startswith('other-')
    if e['status']=='performed' and not other:
     claims_current+=1;good=j>=0 and row['events'][j]['status']=='performed';correct_current+=good
     if not good:records.append(dict(id=row['id'],trigger=e['trigger']['span'],gold_status=row['events'][j]['status'] if j>=0 else 'unmatched',type='false-performed'))
   for o in pred.get('observations',[]):
    owners=sorted(mapping.get(i,-1) for i in o['event_indices']);same=[j for j,g in enumerate(row['observations']) if legacy.overlap(o['evidence']['span'],g['span']) and owners==sorted(g['event_indices']) and o['outcome']==g['outcome']]
    other=any(i>=0 and row['events'][i].get('scope','').startswith('other-') for i in owners)
    if same:
     unique.add((row['id'],same[0]));other_game_correct+=other
    if o.get('joint'):joint_claims+=1;joint_correct+=bool(same)
    if not same:records.append(dict(id=row['id'],span=o['evidence']['span'],outcome=o['outcome'],type='unmatched-result-association'))
  gold_current=sum(e['status']=='performed' and not e.get('scope','').startswith('other-') for r in rows for e in r['events'])
  out[name]=dict(current_game_performed_correct=correct_current,current_game_performed_claims=claims_current,current_game_gold_performed=gold_current,current_game_performed_recall=correct_current/max(1,gold_current),literal_trigger_errors=literal_errors,unique_correct_observation_groups=len(unique),correct_result_links_to_explicit_other_games=other_game_correct,joint_groups_correct=joint_correct,joint_groups_claimed=joint_claims,review_items=records)
 result=dict(status='post-final refutation audit; no model changes or label corrections',results=out,review='agent provisional; not human adjudication',interpretation='Two linguistically correct result links can belong to another game inside a valid thread. They are not usable current-game evidence. run.py now prevents per-event AppID promotion without claiming to solve coreference.')
 write(root/'posthoc-review.json',result);print(json.dumps(out,indent=2))
if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('root',type=Path);run(p.parse_args().root)
