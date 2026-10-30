"""One-to-one claims and complete owner sets, with attribution scored separately."""
import math
from bridge import prior4
overlap=prior4('metrics').overlap
def mapping(row,pred):
 links={};used=set()
 for i,e in enumerate(pred['events']):
  a,b=e['trigger']['span'];options=[(overlap([a,b],g['trigger'])/(max(b,g['trigger'][1])-min(a,g['trigger'][0])),j) for j,g in enumerate(row['events']) if j not in used];best=max(options,default=(0,-1))
  if best[0]>=.5:links[i]=best[1];used.add(best[1])
 return links
def evaluate(rows,preds):
 n=dict(documents=len(rows),gold_events=0,triggers=0,matched=0,gold_performed=0,performed=0,performed_correct=0,false_performed=0,relations=0,relation_correct=0,gold_relations=0,false_success=0,negative_correct=0,joint_claims=0,joint_correct=0,game_claims=0,game_correct=0,gold_game_known=0,cross_game_errors=0,explicit_cross_game_errors=0,unsupported_game_attributions=0,actor_claims=0,actor_correct=0,gold_actor_known=0,actor_kind_claims=0,actor_kind_correct=0,literal_errors=0,temporal_claims=0,temporal_correct=0,gold_temporal=0,proposable_relations=0);details=[]
 for row,pred in zip(rows,preds):
  links=mapping(row,pred);gold=row['events'];n['gold_events']+=len(gold);n['triggers']+=len(pred['events']);n['matched']+=len(links);n['gold_performed']+=sum(e['status']=='performed' for e in gold);n['gold_game_known']+=sum(e.get('game_app_id') is not None for e in gold);n['gold_actor_known']+=sum(e.get('actor_id') is not None for e in gold)
  n['gold_relations']+=sum(bool(o['event_indices']) and o['outcome']!='unknown' for o in row.get('observations',[]));local=dict(id=row['id'],game=row['app_id'],false_performed=0,false_success=0,cross_game_errors=0)
  for i,e in enumerate(pred['events']):
   j=links.get(i);g=gold[j] if j is not None else {};identity=e.get('identity',{});a,b=e['trigger']['span'];n['literal_errors']+=row['text'][a:b]!=e['trigger']['text']
   if e['status']=='performed':
    n['performed']+=1;correct=g.get('status')=='performed';n['performed_correct']+=correct;n['false_performed']+=not correct;local['false_performed']+=not correct
   app=identity.get('game_app_id')
   if app is not None:
    n['game_claims']+=1;correct=g.get('game_app_id')==app;n['game_correct']+=correct
    if not correct:
     n['cross_game_errors']+=1;local['cross_game_errors']+=1
     if g.get('game_app_id') is not None or g.get('game_scope')=='other-game-unidentified' and app==row['app_id']:n['explicit_cross_game_errors']+=1
     else:n['unsupported_game_attributions']+=1
   actor=identity.get('actor_id')
   if actor is not None:n['actor_claims']+=1;n['actor_correct']+=actor==g.get('actor_id')
   kind=identity.get('actor_kind','unknown')
   if kind!='unknown':n['actor_kind_claims']+=1;n['actor_kind_correct']+=kind==g.get('actor_kind')
  used=set()
  for o in pred.get('observations',[]):
   n['relations']+=1;owners=sorted(links.get(i,-1) for i in o['event_indices']);same=[j for j,g in enumerate(row.get('observations',[])) if j not in used and g['event_indices'] and overlap(o['evidence']['span'],g['span'])>0 and owners==sorted(g['event_indices']) and o['outcome']==g['outcome']];correct=bool(same)
   if correct:used.add(same[0])
   n['relation_correct']+=correct;n['negative_correct']+=correct and o['outcome'] in ['persistent','regression'];n['joint_claims']+=bool(o.get('joint'));n['joint_correct']+=correct and bool(o.get('joint'))
   if not correct and o['outcome']=='improved':n['false_success']+=1;local['false_success']+=1
  n['gold_temporal']+=len(row.get('temporal',[]))
  for edge in pred.get('temporal',[]):
   n['temporal_claims']+=1;a=links.get(edge['before'],-1);b=links.get(edge['after'],-1);n['temporal_correct']+=any(g['before']==a and g['after']==b for g in row.get('temporal',[]))
  # End-to-end proposal recall: do not blame the verifier for missing spans,
  # observations or complete groups that were never generated.
  for g in row.get('observations',[]):
   if not g['event_indices'] or g['outcome']=='unknown':continue
   for p in pred.get('proposals',[]):
    owners=sorted(links.get(i,-1) for i in p['event_indices'])
    # Proposals refer to the detector's full observation list by node, exposed
    # separately for evaluation, never filled from gold inference input.
    observed=pred.get('detected_observations',[])
    if p['observation']<len(observed):
     o=observed[p['observation']]
     if owners==sorted(g['event_indices']) and o['outcome']==g['outcome'] and overlap(o['evidence']['span'],g['span']):n['proposable_relations']+=1;break
  details.append(local)
 def div(a,b):return n[a]/n[b] if n[b] else None
 n.update(performed_precision=div('performed_correct','performed'),performed_recall=div('performed_correct','gold_performed'),performed_f1=2*n['performed_correct']/max(1,n['performed']+n['gold_performed']),relation_precision=div('relation_correct','relations'),relation_recall=div('relation_correct','gold_relations'),relation_missed=n['gold_relations']-n['relation_correct'],game_precision=div('game_correct','game_claims'),game_recall=div('game_correct','gold_game_known'),actor_precision=div('actor_correct','actor_claims'),actor_recall=div('actor_correct','gold_actor_known'),details=details)
 return n
