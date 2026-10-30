"""A small learned event-group/observation linker with inspectable features.

This learns source-reported associations, not intervention effects. Every
candidate is a literal observation and a complete syntactic event group.
"""
import itertools,math,re
import numpy as np
from common import evidence
from events import governing,canonical
from relations import observations

FEATURES=['sentence_delta','same_sentence','before_fraction','distance','dependency_fraction','performed_fraction','preexisting_fraction','unknown_fraction','group_size','after_by_with','source_anaphora','then_between','intervening_groups','result_negative','result_improved','result_regression']

def groups(text,parsed,events):
 words=parsed['words'];parts={}
 for i,e in enumerate(events):
  w=words[e['node']];chain=governing(words,w);controller=e['node']
  trial=next((a for a in chain if canonical(a) in ['try','attempt']),None)
  nominal=next((a for a in chain if a['lemma'].lower()=='do' and a['tag']=='VBD'),None)
  if trial:controller=trial['i']
  elif nominal:controller=nominal['i']
  else:
   for a in chain[1:]:
    if any(x['node']==a['i'] for x in events) and w['rel']=='conj':controller=a['i']
  boundary=len(re.findall(r'(?i)\bthen\b',text[:e['trigger']['span'][0]]));key=(e['sentence'],controller,boundary)
  parts.setdefault(key,[]).append(i)
 return list(parts.values())

def choices(text,parsed,events,obs):
 pool=groups(text,parsed,events);a,b=obs['evidence']['span'];valid=[]
 for group in pool:
  if len(group)>6:continue
  if any(events[i]['node']==obs['node'] for i in group):continue
  delta=events[group[0]]['sentence']-obs['sentence'];distance=min(abs(events[i]['trigger']['span'][0]-a) for i in group)
  if abs(delta)>2 or distance>600:continue
  valid.append((distance,group))
 return [g for _,g in sorted(valid,key=lambda x:x[0])[:6]]

def features(text,parsed,events,obs,group):
 chosen=[events[i] for i in group];words=parsed['words'];a,b=obs['evidence']['span'];dist=min(abs(e['trigger']['span'][0]-a) for e in chosen);left=min(a,min(e['trigger']['span'][0] for e in chosen));right=max(b,max(e['trigger']['span'][1] for e in chosen));gap=text[left:right]
 sentence=next(s for s in parsed['sentences'] if s['i']==obs['sentence']);start=sentence['span'][0];delta=chosen[0]['sentence']-obs['sentence'];ancestor=sum(any(p['i']==obs['node'] for p in governing(words,words[e['node']])[1:]) for e in chosen)/len(chosen)
 earlier=[g for g in groups(text,parsed,events) if g!=group and any(left<events[i]['trigger']['span'][0]<right for i in g)]
 return [max(-2,min(2,delta)),float(delta==0),sum(e['trigger']['span'][0]<a for e in chosen)/len(chosen),min(600,dist)/600,ancestor,sum(e['status']=='performed' for e in chosen)/len(chosen),sum(e['status']=='preexisting' for e in chosen)/len(chosen),sum(e['status'] in ['unknown','suggested','negated'] for e in chosen)/len(chosen),len(chosen)/6,float(bool(re.search(r'(?i)\b(?:after|by|with)\b',gap))),float(bool(re.match(r'(?i)\s*(?:It|That|This|Everything|Nothing|None|The game)\b',text[start:]))),float(bool(re.search(r'(?i)\bthen\b',gap))),min(5,len(earlier))/5,float(obs['outcome']=='persistent'),float(obs['outcome']=='improved'),float(obs['outcome']=='regression')]

def supported(events,group):
 return all(events[i]['status'] in ['performed','preexisting'] and not(events[i].get('role')=='reproduction' and events[i]['status']!='performed') for i in group)

def predict_score(model,x):
 a=np.asarray(x,dtype=float)
 if model['kind']=='linear':
  z=float(a@np.array(model['coefficients'])+model['intercept']);return 1/(1+math.exp(-max(-40,min(40,z))))
 node=0;tree=model['tree']
 while tree['left'][node]!=tree['right'][node]:node=tree['left'][node] if a[tree['feature'][node]]<=tree['threshold'][node] else tree['right'][node]
 return tree['positive'][node]

def link(text,parsed,events,model,threshold=.5,margin=0.):
 out=[];rejected=[]
 for obs in observations(text,parsed):
  scored=[]
  for group in choices(text,parsed,events,obs):
   if not supported(events,group):continue
   score=predict_score(model,features(text,parsed,events,obs,group));scored.append((score,group))
  scored.sort(key=lambda x:-x[0])
  if not scored or scored[0][0]<threshold or len(scored)>1 and scored[0][0]-scored[1][0]<margin:
   rejected.append(obs);continue
  score,owners=scored[0]
  out.append(dict(obs,event_indices=sorted(owners),joint=len(owners)>1,individual_cause='unknown',relation='learned-reported-association',model_score=score,score_meaning='uncalibrated association score; not causal or safety probability',execution_authorized=False,scope='community report; no independent causality'))
 return out,rejected

def train_rows(rows,parser,extract):
 examples=[];missed=[]
 for row in rows:
  text=row['text'];parsed=parser.parse(text);events=extract(text,parsed);mapping={}
  for i,e in enumerate(events):
   a,b=e['trigger']['span'];hits=[]
   for j,g in enumerate(row['events']):
    x,y=g['trigger'];iou=max(0,min(b,y)-max(a,x))/(max(b,y)-min(a,x))
    if iou>=.5:hits.append((iou,j))
   if hits:mapping[i]=max(hits)[1]
  for oi,gold in enumerate(row.get('observations',[])):
   if not gold['event_indices'] or gold['outcome']=='unknown':continue
   a,b=gold['span'];words=[w for w in parsed['words'] if min(w['span'][1],b)>max(w['span'][0],a)];head=next((w for w in words if w['pos']=='VERB'),words[0] if words else None)
   if head is None:continue
   obs=dict(evidence=evidence(text,a,b),outcome=gold['outcome'],node=head['i'],sentence=head['sentence']);candidates=[]
   for group in choices(text,parsed,events,obs):
    if not supported(events,group):continue
    owners=sorted(mapping.get(i,-1) for i in group);label=owners==sorted(gold['event_indices']);candidates.append(dict(x=features(text,parsed,events,obs,group),y=int(label),id=row['id'],app_id=row['app_id'],observation=oi,origin=row.get('origin','real-agent-provisional')))
   if any(e['y'] for e in candidates):examples+=candidates
   else:missed.append(dict(id=row['id'],observation=oi,reason='correct full owner group absent from proposals'))
 return examples,missed
