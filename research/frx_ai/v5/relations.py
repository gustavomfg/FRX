"""Deconstruct observations, then reconstruct bounded reported event groups.

Deterministic candidate generator, not scientific causal inference. A learned
verifier can subsequently reject candidate edges; it cannot invent evidence.
"""
import re
from common import evidence
from events import children,family,governing,canonical

RESULTS={'fix','resolve','solve','improve','eliminate','work','help','persist','crash','fail','start','stop','freeze','hang','break','improve','worsen','cause','introduce','drop','distort','stutter','change','remove','have','get','feel','result','appear','make'}

def observations(text,parsed):
 words=parsed['words'];out=[]
 for w in words:
  lemma=w['lemma'].lower()
  if any(m.start()<=w['span'][0]<m.end() for m in re.finditer(r'```[\s\S]*?(?:```|\Z)|~~[\s\S]*?~~',text)):continue
  if lemma not in RESULTS:continue
  if w['pos'] not in ['VERB','ADJ','NOUN']:continue
  if w['pos']=='NOUN' and lemma not in ['change']:continue
  direct=children(words,w['i']);chain=governing(words,w)
  controls=direct+[x for a in chain[1:] if a['lemma'].lower() in ['try','see','want','hope'] for x in children(words,a['i'])]
  sentence=next(s for s in parsed['sentences'] if s['i']==w['sentence']);raw=text[slice(*sentence['span'])]
  if '?' in raw or any(x['lemma'].lower() in ['should','could','would','will','might','may'] for x in controls if x['rel'].startswith('aux')):continue
  if any(x['lemma'].lower() in ['if','unless'] for x in direct):continue
  if w['rel'] in ['xcomp','ccomp'] and any(a['lemma'].lower() in ['try','hope','want'] and a['i']!=w['i'] for a in chain):continue
  terms=[x for x in family(words,w['i']) if x['sentence']==w['sentence']];branch=' '.join(x['text'] for x in terms).lower();negative_context=direct+[y for x in direct if x['rel'].startswith('nsubj') for y in family(words,x['i']) if y['pos']!='VERB'];negative=any(x['lemma'].lower() in ['not','never','nothing','none','no'] for x in negative_context)
  outcome=None
  if lemma in ['fix','resolve','solve','eliminate','improve','help','work']:outcome='persistent' if negative else 'improved'
  elif lemma in ['break','worsen','distort']:outcome='regression'
  elif lemma in ['crash','fail','freeze','hang','stutter','persist']:
   if any(a['lemma'].lower() in ['fix','resolve','stop','prevent'] for a in chain[1:]) or (w['head'] is not None and words[w['head']]['lemma'].lower() in ['fix','resolve','stop','prevent']):continue
   outcome='improved' if negative and re.search(r'(?i)\b(?:anymore|any more|longer|now)\b',branch) else 'persistent'
  elif lemma in ['cause','result'] and re.search(r'(?i)\b(?:crash|crashes|distort\w*|error|artifact\w*|regression|hang)\b',branch):outcome='regression'
  elif lemma=='stop':
   if any(x['lemma'].lower() in ['crash','hang','stutter','freeze'] for x in terms):outcome='improved'
  elif lemma=='start' and any(x['lemma'].lower() in ['work','run','load'] for x in direct):outcome='improved'
  elif lemma=='change' and negative:outcome='persistent'
  elif lemma=='remove' and negative and any(x['rel'].startswith('nsubj') for x in direct):outcome='persistent'
  elif lemma=='drop' and re.search(r'(?i)\b(?:fps|performance|framerate)\b',branch):outcome='regression'
  if outcome is None and lemma=='feel' and re.search(r'(?i)\b(?:sluggish|worse|slower|stutter|bad)\b',branch):outcome='regression'
  if outcome is None and lemma=='get' and re.search(r'(?i)\b(?:worse|slower|sluggish)\b',branch):outcome='regression'
  if outcome is None and lemma=='make' and re.search(r'(?i)\bno (?:difference|change)\b',branch):outcome='persistent'
  if outcome is None and lemma=='get' and re.search(r'(?i)\b(?:error|crash|hang)\b',branch):outcome='improved' if negative else 'persistent'
  if outcome is None and lemma=='have' and negative and re.search(r'(?i)\b(?:issue|issues|problem|problems|crash|crashes)\b',branch):outcome='improved'
  if outcome is None and lemma=='appear' and negative and re.search(r'(?i)\b(?:window|screen|game)\b',branch):outcome='persistent'
  if outcome is None:continue
  if lemma=='work' and any(a['lemma'].lower() in ['start','stop','fix','resolve'] for a in chain[1:]):continue
  if any(x['lemma'].lower() in ['worse','sluggish','slower'] for x in terms):outcome='regression'
  # Keep an atomic predicate/argument quote rather than the entire paragraph.
  relevant=[w]
  for x in direct:
   if x['rel'].split(':')[0] in ['nsubj','aux','advmod','obj','xcomp','compound','cop','det']:
    relevant.extend(y for y in family(words,x['i']) if y['pos']!='PUNCT' and canonical(y) not in ['enable','disable','install','uninstall','update','switch','try'])
  a=max(sentence['span'][0],min(x['span'][0] for x in relevant));b=min(sentence['span'][1],max(x['span'][1] for x in relevant))
  if b-a>240:a=w['span'][0];b=min(sentence['span'][1],w['span'][1]+100)
  item=dict(evidence=evidence(text,a,b),outcome=outcome,node=w['i'],sentence=w['sentence'],lemma=lemma,modality='tentative' if any(x['lemma'].lower() in ['seem','think','maybe'] for x in chain) else 'reported',causality_confirmed=False)
  if not any(p['outcome']==outcome and min(p['evidence']['span'][1],b)>max(p['evidence']['span'][0],a) for p in out):out.append(item)
 return out

def assemble(text,parsed,events,observed,policy='structured'):
 words=parsed['words'];edges=[]
 for obs in observed:
  w=words[obs['node']];a,b=obs['evidence']['span'];same=[(i,e) for i,e in enumerate(events) if e['sentence']==obs['sentence'] and e['status'] in ['performed','preexisting'] and not (e.get('role')=='reproduction' and e['status']!='performed') and e['node']!=obs['node']];owners=[];mechanism=None
  # Explicit dependency path: operations attached to the result predicate.
  for i,e in same:
   route=governing(words,words[e['node']])[1:]
   if any(x['i']==obs['node'] for x in route) and (e['trigger']['span'][0]<a or words[e['node']]['rel'].split(':')[0] in ['advcl','obl','csubj','xcomp']):owners.append(i)
  if owners:
   selected={events[i]['node'] for i in owners}
   for i,e in same:
    if words[e['node']]['rel']=='conj' and words[e['node']]['head'] in selected:owners.append(i)
   mechanism='syntactic-reported-association'
  if not owners:
   prior=[(i,e) for i,e in same if e['trigger']['span'][1]<=a]
   # A previous observed outcome followed by a new action starts a new group.
   earlier_obs=[o['evidence']['span'][1] for o in observed if o is not obs and o['sentence']==obs['sentence'] and o['evidence']['span'][1]<a]
   cutoff=max(earlier_obs,default=-1);after=[(i,e) for i,e in prior if e['trigger']['span'][0]>=cutoff]
   if not after:after=prior
   if after:
    boundary=max([m.end() for m in re.finditer(r'(?i)\bthen\b|;',text[:a])],default=0)
    current=[(i,e) for i,e in after if e['trigger']['span'][0]>=boundary] or after
    owners=[i for i,e in current];mechanism='same-clause-reported-sequence'
  if not owners and policy!='same-clause':
   previous=[(i,e) for i,e in enumerate(events) if e['sentence']==obs['sentence']-1 and e['status']=='performed']
   sentence=next(s for s in parsed['sentences'] if s['i']==obs['sentence']);start=sentence['span'][0];prefix=text[start:max(start,a+1)]
   if previous and re.match(r'(?i)\s*(?:It|That|This|Everything|Nothing|None|The (?:game|crashes|crash|stuttering|freezes|freezing)|Performance|Frametime|FPS)\b',text[start:]):
    owners=[i for i,e in previous];mechanism='bounded-source-anaphora'
  if not owners:continue
  # Speculative or explicitly unresolved attribution stays an observation.
  sentence=next(s for s in parsed['sentences'] if s['i']==obs['sentence']);raw=text[slice(*sentence['span'])]
  if re.search(r"(?i)\b(?:not sure what|don't know what|cannot say|can't say|might have|could have)\b",raw):continue
  edge=dict(obs,event_indices=sorted(set(owners)),joint=len(set(owners))>1,individual_cause='unknown',relation=mechanism,execution_authorized=False,scope='community-reported-association; not independent causality')
  # One result/group, not several duplicate confirmations from nested verbs.
  if not any(e['event_indices']==edge['event_indices'] and e['outcome']==edge['outcome'] and min(e['evidence']['span'][1],b)>max(e['evidence']['span'][0],a) for e in edges):edges.append(edge)
 return edges

def reconstruct(text,parsed,events,policy='structured'):
 obs=observations(text,parsed);edges=assemble(text,parsed,events,obs,policy);temporal=[]
 for i,e in enumerate(events):
  for j,f in enumerate(events[i+1:],i+1):
   gap=text[e['trigger']['span'][1]:f['trigger']['span'][0]];m=re.search(r'(?i)\bthen\b',gap)
   if m and e['status']=='performed' and f['status']=='performed':
    a=e['trigger']['span'][1]+m.start();temporal.append(dict(before=i,after=j,cue=evidence(text,a,a+len(m[0])),causality_confirmed=False));break
 return dict(events=events,observations=edges,unattributed_observations=[o for o in obs if not any(x['node']==o['node'] for x in edges)],temporal=temporal)
