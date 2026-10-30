"""Learn observation likelihoods and inspection policies in a declared simulator.

All causes, priors, noise and costs below are SYNTHETIC experimental assumptions.
None are estimates for real Proton games. No repair actions are performed.
"""
import argparse
from pathlib import Path
import time
import numpy as np
from common import write,preserve_fit

CAUSES=['missing-runtime','graphics-stack','feature-setting','input-runtime']
CHECKS=['loader-log','vulkan-log','version-history','feature-configuration','input-log','generic-crash-log','duplicate-loader-log']
COST=np.array([1.,1.1,1.8,1.3,1.,.8,1.])
# Conditional probabilities only define synthetic data generation, not inference.
BASE=np.array([[.92,.10,.15,.12,.08,.58,.92],[.09,.88,.82,.28,.07,.76,.09],
               [.08,.23,.20,.90,.09,.65,.08],[.07,.08,.12,.11,.93,.35,.07]])

def generate(n,seed,shift=False):
 rng=np.random.default_rng(seed);gpu=rng.integers(0,3,n);cause=rng.choice(4,n,p=[.3,.3,.25,.15]);p=BASE[cause].copy()
 p[:,1]=np.clip(p[:,1]+np.where(gpu==0,.05,-.03),.02,.98)
 if shift:p=.55*p+.45*.5
 x=(rng.random(p.shape)<p).astype(int);x[:,-1]=x[:,0] # duplicated source, not independent evidence
 return gpu,cause,x

def learn(gpu,y,x,alpha,contextual):
 groups=3 if contextual else 1;prior=[];likelihood=[]
 for g in range(groups):
  mask=gpu==g if contextual else np.ones(len(y),dtype=bool);counts=np.bincount(y[mask],minlength=4)+alpha
  prior.append(counts/counts.sum());q=[]
  for c in range(4):
   samples=x[mask&(y==c)];q.append((samples.sum(0)+alpha)/(len(samples)+2*alpha))
  likelihood.append(q)
 return np.array(prior),np.array(likelihood)

def posterior(prior,likelihood,observed):
 log=np.log(prior)
 for j,value in observed.items():log+=np.log(likelihood[:,j] if value else 1-likelihood[:,j])
 p=np.exp(log-log.max());return p/p.sum()

def entropy(p):return float(-(p*np.log2(p.clip(1e-12))).sum())

def next_check(prior,likelihood,observed,available):
 p=posterior(prior,likelihood,observed);scores=[]
 for j in available:
  positive=p@likelihood[:,j];after=0
  for value,prob in [(1,positive),(0,1-positive)]:after+=prob*entropy(posterior(prior,likelihood,observed|{j:value}))
  scores.append((entropy(p)-after)/COST[j])
 return available[int(np.argmax(scores))]

def evaluate(model,data,policy,seed=33):
 priors,like=model;gpu,y,x=data;rng=np.random.default_rng(seed);records=[]
 for i,cause in enumerate(y):
  group=int(gpu[i]) if len(priors)>1 else 0;prior=priors[group];q=like[group];observed={};cost=0
  allowed=list(range(len(CHECKS)))
  if policy in ['eig-dedup','static-dedup']:allowed.remove(6)
  for _ in range(5):
   p=posterior(prior,q,observed)
   if observed and p.max()>=.9:break
   available=[j for j in allowed if j not in observed]
   if not available:break
   if policy.startswith('eig'):j=next_check(prior,q,observed,available)
   elif policy=='random':j=int(rng.choice(available))
   else:j=available[0]
   observed[j]=int(x[i,j]);cost+=COST[j]
  p=posterior(prior,q,observed);records.append(dict(correct=int(p.argmax()==cause),inspections=len(observed),cost=cost,
    brier=float(((p-np.eye(4)[cause])**2).sum()),confident=bool(p.max()>=.9)))
 return dict(n=len(y),accuracy=float(np.mean([r['correct'] for r in records])),
  inspections=float(np.mean([r['inspections'] for r in records])),cost=float(np.mean([r['cost'] for r in records])),
  brier=float(np.mean([r['brier'] for r in records])),false_confident=int(sum(r['confident'] and not r['correct'] for r in records)),records=records)

def run(root):
 preserve_fit(root);out=root/'simulation';out.mkdir(exist_ok=True);train=generate(1200,17);dev=generate(300,29);test=generate(600,43);shift=generate(600,71,True);records=[]
 for name,alpha,contextual in [('pooled-a1',1.,False),('context-a1',1.,True),('context-a5',5.,True)]:
  start=time.perf_counter();model=learn(*train,alpha,contextual);results={}
  for split,data in [('dev',dev),('synthetic-test',test),('shifted-noise-test',shift)]:
   results[split]={policy:evaluate(model,data,policy) for policy in ['static','random','eig','eig-dedup','static-dedup']}
  record=dict(name=name,family='simulation',seconds=time.perf_counter()-start,alpha=alpha,contextual=contextual,
   synthetic_only=True,results=results);write(out/(name+'.json'),record);records.append(record)
  print(name,{p:{k:round(v,3) if isinstance(v,float) else v for k,v in m.items() if k!='records'} for p,m in results['synthetic-test'].items()},flush=True)
 write(out/'experiments.json',[{k:v for k,v in r.items() if k!='results'} for r in records])
 write(out/'assumptions.json',dict(synthetic_only=True,causes=CAUSES,checks=CHECKS,costs=COST.tolist(),generator=BASE.tolist(),train_seed=17,dev_seed=29,test_seed=43,shift_seed=71,
  explanation='Closed-world single-fault toy; duplicate sensor shares the same outcome; no real-game likelihood or safety guarantees.'))

if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('root',type=Path);run(p.parse_args().root)
