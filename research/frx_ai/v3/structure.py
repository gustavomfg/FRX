"""Learn a tiny diagnostic dependency graph; all cases are explicitly synthetic."""
import argparse
from pathlib import Path
import itertools
import time
import hashlib
import json
import numpy as np
from common import write
from simulation import generate,COST

STATES=np.array(list(itertools.product([0,1],repeat=7)),dtype=int)

def mutual_information(y,x,i,j):
 result=0.
 for c in range(4):
  z=x[y==c];counts=np.ones((2,2))
  for a,b in z[:,[i,j]]:counts[a,b]+=1
  p=counts/counts.sum();result+=len(z)/len(y)*np.sum(p*np.log(p/(p.sum(1)[:,None]*p.sum(0)[None,:])))
 return result

def spanning_tree(y,x):
 edges=sorted([(mutual_information(y,x,i,j),i,j) for i in range(7) for j in range(i)],reverse=True)
 groups=list(range(7));selected=[]
 def find(i):
  while groups[i]!=i:i=groups[i]
  return i
 for weight,i,j in edges:
  if find(i)!=find(j):groups[find(i)]=find(j);selected.append((i,j))
 parent={0:None};todo=[0]
 while todo:
  at=todo.pop()
  for i,j in selected:
   child=j if i==at else i if j==at else None
   if child is not None and child not in parent:parent[child]=at;todo.append(child)
 return [parent[i] for i in range(7)]

def fit(y,x,kind):
 counts=np.bincount(y,minlength=4)+1.;prior=counts/counts.sum();parents=[None]*7
 if kind=='tree':parents=spanning_tree(y,x)
 duplicate={}
 if kind=='learned-dedup':
  for i in range(7):
   for j in range(i):
    if np.array_equal(x[:,i],x[:,j]):duplicate[i]=j;break
 joint=np.ones((4,len(STATES)))
 for c in range(4):
  z=x[y==c]
  for i,parent in enumerate(parents):
   if i in duplicate:
    joint[c]*=(STATES[:,i]==STATES[:,duplicate[i]])
   elif parent is None:
    p=(z[:,i].sum()+1)/(len(z)+2);joint[c]*=np.where(STATES[:,i],p,1-p)
   else:
    for value in [0,1]:
     mask=z[:,parent]==value;p=(z[mask,i].sum()+1)/(mask.sum()+2)
     state_mask=STATES[:,parent]==value;joint[c,state_mask]*=np.where(STATES[state_mask,i],p,1-p)
  joint[c]/=joint[c].sum()
 return dict(kind=kind,prior=prior.tolist(),joint=joint.tolist(),parents=parents,duplicates=duplicate)

def posterior(model,observed):
 mask=np.ones(len(STATES),dtype=bool)
 for j,v in observed.items():mask&=STATES[:,j]==v
 joint=np.array(model['joint']);mass=np.array(model['prior'])*joint[:,mask].sum(1)
 if mass.sum()==0:return np.array(model['prior']),mask
 return mass/mass.sum(),mask

def entropy(p):return float(-(p*np.log2(p.clip(1e-12))).sum())

def evaluate(model,data,policy):
 _,y,x=data;records=[];joint=np.array(model['joint']);prior=np.array(model['prior']);duplicates={int(k):v for k,v in model['duplicates'].items()}
 for idx,cause in enumerate(y):
  observed={};cost=0
  for _ in range(5):
   p,mask=posterior(model,observed)
   if observed and p.max()>=.9:break
   available=[j for j in range(7) if j not in observed and j not in duplicates]
   if policy=='static':j=available[0]
   else:
    gain=[];normal=(prior[:,None]*joint[:,mask]).sum()
    for j in available:
     positive=(prior[:,None]*joint[:,mask&(STATES[:,j]==1)]).sum()/normal;after=0
     for value,prob in [(1,positive),(0,1-positive)]:after+=prob*entropy(posterior(model,observed|{j:value})[0])
     gain.append((entropy(p)-after)/COST[j])
    j=available[int(np.argmax(gain))]
   observed[j]=int(x[idx,j]);cost+=COST[j]
  p,_=posterior(model,observed);records.append(dict(correct=int(p.argmax()==cause),cost=cost,inspections=len(observed),
   brier=float(((p-np.eye(4)[cause])**2).sum()),false_confident=int(p.max()>=.9 and p.argmax()!=cause)))
 return dict(n=len(y),accuracy=float(np.mean([r['correct'] for r in records])),cost=float(np.mean([r['cost'] for r in records])),
  inspections=float(np.mean([r['inspections'] for r in records])),brier=float(np.mean([r['brier'] for r in records])),
  false_confident=sum(r['false_confident'] for r in records),records=records)

def run(root):
 out=root/'structure';out.mkdir(exist_ok=True)
 if (out/'final.json').exists():raise ValueError('preserve first structure final')
 start=time.perf_counter();_,y,x=generate(1200,17);dev=generate(300,29);models={};development={};records=[];selection=None
 for name in ['naive','learned-dedup','tree']:
  tick=time.perf_counter();model=fit(y,x,name);models[name]=model;development[name]={p:evaluate(model,dev,p) for p in ['static','eig']}
  records.append(dict(name=name,family='learned-structure',seconds=time.perf_counter()-tick))
  base=development[name]['static']
  for policy,m in development[name].items():
   if m['accuracy']>=base['accuracy']-.02 and m['false_confident']<=base['false_confident']+5:
    if selection is None or m['cost']<selection['dev_cost']:selection=dict(model=name,policy=policy,dev_cost=m['cost'])
 write(out/'models.json',models);write(out/'development.json',development);write(out/'experiments.json',records)
 write(out/'frozen.json',dict(selection=selection,final_seeds_unseen=[131,137],model_sha256=hashlib.sha256((out/'models.json').read_bytes()).hexdigest(),code_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest()))
 # Generate reserved synthetic cases only after the above freeze.
 final={}
 for split,data in [('synthetic-final',generate(600,131)),('noise-shift-final',generate(600,137,True))]:
  final[split]={name:{p:evaluate(model,data,p) for p in ['static','eig']} for name,model in models.items()}
 result=dict(synthetic_only=True,prior_only_accuracy=float(np.mean(generate(600,131)[1]==np.argmax(models['naive']['prior']))),
  selection=selection,results=final,seconds=time.perf_counter()-start)
 with (out/'final.json').open('x') as file:json.dump(result,file,indent=2)
 print(json.dumps(dict(selection=selection,summary={s:{n:{p:{k:v for k,v in m.items() if k!='records'} for p,m in ps.items()} for n,ps in values.items()} for s,values in final.items()}),indent=2))

if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('root',type=Path);run(p.parse_args().root)
