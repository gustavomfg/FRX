"""Retrieve bounded event-group/result candidates before verifying any claim."""
import re,itertools,math
from bridge import children,evidence
from grounding import ancestors,clause

FEATURES=['distance','same_sentence','sentence_delta','before_fraction','size','dependency_fraction','explicit_after','explicit_before','joint_incomplete','game_conflict','actor_conflict','game_known','actor_known','negative','improved','regression','intervening_result','direct_result_path','unknown_scope']
def conflict(a,b):
 ga,gb=a.get('game_app_id'),b.get('game_app_id');game=bool(ga and gb and ga!=gb)
 actor=bool(a.get('actor_id') and b.get('actor_id') and a['actor_id']!=b['actor_id']) or a.get('actor_kind')=='third_party' and b.get('actor_kind')=='reporter' or b.get('actor_kind')=='third_party' and a.get('actor_kind')=='reporter'
 return game,bool(actor)
def joint_clusters(text,bundle):
 events=bundle['events'];words=bundle['parsed']['words'];clusters=[]
 for i,e in enumerate(events):
  if e['status']!='performed':continue
  attached=False
  if clusters:
   last=clusters[-1][-1];f=events[last];gap=text[f['trigger']['span'][1]:e['trigger']['span'][0]];game,actor=conflict(e['identity'],f['identity']);intervening=any(f['trigger']['span'][1]<=o['evidence']['span'][0]<e['trigger']['span'][0] for o in bundle['observations'])
   conjunct=words[e['node']]['rel']=='conj' and any(x['i']==f['node'] for x in ancestors(words,e['node']))
   if not game and not actor and not intervening and (conjunct or e['sentence']==f['sentence'] and re.search(r'(?i)\b(?:and|then)\b',gap) and not re.search(r'[.!?;]|\bbut\b',gap)):
    clusters[-1].append(i);attached=True
  if not attached:clusters.append([i])
 return clusters
def candidates(text,bundle):
 events=bundle['events'];words=bundle['parsed']['words'];clusters=joint_clusters(text,bundle);out=[]
 for oi,obs in enumerate(bundle['observations']):
  a,b=obs['evidence']['span'];near=[i for i,e in enumerate(events) if e['status']=='performed' and abs(e['sentence']-obs['sentence'])<=2 and abs(e['trigger']['span'][0]-a)<650 and e['node']!=obs['node']]
  options={tuple([i]) for i in near}
  for c in clusters:
   if set(c)<=set(near) and len(c)<=6:options.add(tuple(c))
  # A multi-sentence sequence since the previous result is a joint procedure,
  # not a collection of independent successful interventions.
  previous=max([o['evidence']['span'][1] for o in bundle['observations'] if o['evidence']['span'][1]<a],default=0)
  sequence=tuple(i for i in near if previous<=events[i]['trigger']['span'][0]<a)
  if 1<len(sequence)<=6:options.add(sequence)
  for group in sorted(options):
   chosen=[events[i] for i in group];left=min([a]+[e['trigger']['span'][0] for e in chosen]);right=max([b]+[e['trigger']['span'][1] for e in chosen]);span=text[left:right];games=[];actors=[]
   for e in chosen:
    g,h=conflict(e['identity'],obs['identity']);games.append(g);actors.append(h)
   incomplete=any(set(c)&set(group) and not set(c)<=set(group) for c in clusters)
   dependencies=sum(any(p['i']==obs['node'] for p in ancestors(words,e['node'])[1:]) for e in chosen)/len(chosen)
   before=sum(e['trigger']['span'][0]<a for e in chosen)/len(chosen);delta=chosen[0]['sentence']-obs['sentence'];negative=obs['outcome']=='persistent';direct=dependencies>0
   v=[min(abs(events[i]['trigger']['span'][0]-a) for i in group)/650,float(delta==0),delta,before,len(group)/6,dependencies,float(bool(re.search(r'(?i)\b(?:after|by|since)\b',span))),float(bool(re.search(r'(?i)\bbefore\b',span))),float(incomplete),float(any(games)),float(any(actors)),float(all(e['identity']['game_app_id'] for e in chosen) and obs['identity']['game_app_id'] is not None),float(all(e['identity']['actor_kind']=='reporter' for e in chosen)),float(negative),float(obs['outcome']=='improved'),float(obs['outcome']=='regression'),float(any(left<o['evidence']['span'][0]<right and o is not obs for o in bundle['observations'])),float(direct),float(any(e['identity']['game_app_id'] is None for e in chosen))]
   out.append(dict(observation=oi,event_indices=list(group),features=v,hard_conflict=bool(any(games) or any(actors)),incomplete_joint=incomplete,evidence=evidence(text,left,min(right,left+1300))))
 return out
def verify_literal(text,bundle,pair):
 if pair['hard_conflict']:return 'contradicted','different-explicit-game-or-actor'
 if pair['incomplete_joint']:return 'not_demonstrated','individual-credit-from-joint-procedure'
 obs=bundle['observations'][pair['observation']];chosen=[bundle['events'][i] for i in pair['event_indices']];words=bundle['parsed']['words']
 for e in chosen:
  first=e['identity'];second=obs['identity']
  if first.get('game_app_id') and second.get('unlinked_name_candidates') or second.get('game_app_id') and first.get('unlinked_name_candidates'):return 'not_demonstrated','unresolved-game-context-switch'
  for key in ['gpu','driver','proton']:
   a=first.get('technical_context',{}).get(key);b=second.get('technical_context',{}).get(key)
   if a and b and a['text'].casefold()!=b['text'].casefold():return 'contradicted','different-explicit-technical-context'
 predicate=words[obs['node']]
 if predicate['rel'] in ['xcomp','advcl'] and any(predicate['head']==e['node'] for e in bundle['events']) and any(x['lemma'].lower()=='to' for x in children(words,predicate['i'])):return 'not_demonstrated','purpose-not-an-observed-result'
 if all(e['trigger']['span'][0]>obs['evidence']['span'][0] for e in chosen) and not re.search(r'(?i)\b(?:after|by|once|when|since)\b',pair['evidence']['text']):return 'not_demonstrated','result-first-without-temporal-link'
 if any(e['status']!='performed' for e in chosen):return 'not_demonstrated','execution-not-supported'
 for e in chosen:
  route=ancestors(words,e['node']);marks=[x for p in route[:2] for x in children(words,p['i']) if x['rel']=='mark']
  if any(x['lemma'].lower()=='before' for x in marks) and any(p['i']==obs['node'] for p in route):return 'contradicted','result-before-action'
 if any(e['identity']['actor_kind']=='quoted' for e in chosen) and obs['identity']['actor_kind']!='quoted':return 'not_demonstrated','quote-attribution-unresolved'
 return 'candidate','needs-relation-verification'
def deterministic_score(pair):
 x=dict(zip(FEATURES,pair['features']))
 # Explicit structural score, not a probability. Unknown game identity is
 # permitted for a narrative relation but never current-game applicability.
 return (3*x['dependency_fraction']+2*x['explicit_after']+x['same_sentence']+x['before_fraction']-.6*x['distance']-2*x['intervening_result']-.3*x['size'])
def reported_edge(bundle,pair,method,score):
 obs=bundle['observations'][pair['observation']];owners=pair['event_indices'];games={bundle['events'][i]['identity']['game_app_id'] for i in owners};game=next(iter(games)) if len(games)==1 and None not in games else None
 return dict(obs,event_indices=owners,joint=len(owners)>1,relation=method,verification='supported-as-source-reported-association',score=float(score),score_is_calibrated_probability=False,game_app_id=game,individual_cause='unknown',causality_confirmed=False,current_environment_verified=False,execution_authorized=False,automatic_approval=False)
