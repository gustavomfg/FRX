"""Shared constrained decoding for PyTorch research and native ONNX runtime."""
import numpy as np
from data import VERBS,STATUSES,DIRECTIONS
from proof import verify,quoted,attach_observations,temporal_edges

def decode(row,positions,prob,verified=False,threshold=.6,whole_word=False,head_projection=False):
 text=row['text'];candidates=[];active=[]
 for j,(a,b) in enumerate(positions):
  yes=a<b and prob[0][j,1]>=threshold
  if yes and active and a-positions[active[-1]][1]<=1:active.append(j)
  else:
   if active:candidates.append(active)
   active=[j] if yes else []
 if active:candidates.append(active)
 if head_projection:
  expanded=[]
  for group in candidates:
   a=positions[group[0]][0];b=positions[group[-1]][1]
   while a>0 and (text[a-1].isalnum() or text[a-1]=='_'):a-=1
   while b<len(text) and (text[b].isalnum() or text[b]=='_'):b+=1
   heads=list(VERBS.finditer(text[a:b]))
   if not heads:expanded.append(group)
   for head in heads:
    first=a+head.start();last=a+head.end();indices=[j for j,(x,y) in enumerate(positions) if x<y and min(y,last)>max(x,first)]
    if indices:expanded.append(indices)
  candidates=expanded
 events=[];seen=set()
 for group in candidates:
  a=positions[group[0]][0];b=positions[group[-1]][1]
  if whole_word:
   while a>0 and (text[a-1].isalnum() or text[a-1]=='_'):a-=1
   while b<len(text) and (text[b].isalnum() or text[b]=='_'):b+=1
   group=[j for j,(x,y) in enumerate(positions) if x<y and min(y,b)>max(x,a)]
  if (a,b) in seen:continue
  seen.add((a,b))
  if quoted(text,a):continue
  item=verify(text,[a,b])
  if item is None:continue
  if not verified:
   item['status']=STATUSES[int(prob[1][group].mean(0).argmax())];item['direction']=DIRECTIONS[int(prob[2][group].mean(0).argmax())];item['reliability']='learned-unverified'
  item['proposal_score']=float(prob[0][group,1].mean());events.append(item)
 return dict(events=events,observations=attach_observations(text,events) if verified else [],temporal=temporal_edges(text,events) if verified else [])
