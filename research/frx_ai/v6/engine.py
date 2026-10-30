"""Narrative relation verification; no repairs, safety approval or causal claims."""
import json,math
from grounding import deconstruct
from pairs import candidates,verify_literal,deterministic_score,reported_edge

class Interpreter:
 def __init__(self,parser,catalog,verifier=None,threshold=1.5,mode='structural',guard=True):self.parser=parser;self.catalog=catalog;self.verifier=verifier;self.threshold=threshold;self.mode=mode;self.guard=guard
 def analyze(self,row):
  b=deconstruct(row,self.parser,self.catalog,self.guard);pairs=candidates(row['text'],b);pool={};claims=[]
  if self.verifier is not None:learned=self.verifier(row,b,pairs)
  else:learned=None
  for pi,p in enumerate(pairs):
   verdict,reason=verify_literal(row['text'],b,p)
   if verdict=='candidate':
    score=learned[pi] if learned is not None else -p['features'][0] if self.mode=='nearest' else deterministic_score(p)
    p=dict(p,score=float(score));pool.setdefault(p['observation'],[]).append(p)
   else:claims.append(dict(observation=p['observation'],event_indices=p['event_indices'],verdict=verdict,reason=reason))
  out=[]
  for oi,ps in pool.items():
   ps.sort(key=lambda p:(-p['score'],-len(p['event_indices'])))
   p=ps[0]
   if p['score']>=self.threshold:out.append(reported_edge(b,p,self.mode,p['score']))
   else:claims.append(dict(observation=oi,event_indices=p['event_indices'],verdict='not_demonstrated',reason='insufficient-verifier-support',score=p['score']))
  return dict(events=b['events'],observations=out,temporal=b['temporal'],detected_observations=b['observations'],unattributed_observations=[o for i,o in enumerate(b['observations']) if not any(x['node']==o['node'] for x in out)],verification_rejections=claims,proposals=pairs,game_mentions=b['game_mentions'],automatic_approval=False,execution_authorized=False)
