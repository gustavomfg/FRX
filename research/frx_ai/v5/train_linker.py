"""Train compact source-association models; no held-out final access."""
import argparse,time,json
from pathlib import Path
import numpy as np
from sklearn.linear_model import LogisticRegression
from sklearn.tree import DecisionTreeClassifier
from common import read,write
from syntax import Syntax
from events import extract
from relations import reconstruct
from linker import FEATURES,train_rows,link
from relation_data import synthetic_relations
from assess import evaluate

def linear(model):return dict(kind='linear',coefficients=model.coef_[0].tolist(),intercept=float(model.intercept_[0]))
def tree(model):
 t=model.tree_;v=t.value[:,0,:];positive=(v[:,1]/v.sum(1)).tolist()
 return dict(kind='tree',tree=dict(left=t.children_left.tolist(),right=t.children_right.tolist(),feature=t.feature.tolist(),threshold=t.threshold.tolist(),positive=positive))

def run(root,family_name="linker"):
 if (root/'frozen.json').exists():raise ValueError('sealed')
 dest=root/(family_name+'-development.json')
 if dest.exists():raise ValueError('preserve completed family')
 start=time.perf_counter();parser=Syntax(root);train=read(root/'train-known.jsonl')+(read(root/'extra-audited.jsonl') if family_name.endswith('audited') else []);dev=read(root/'dev-known.jsonl');real,miss=train_rows(train,parser,extract);synth,synth_missed=train_rows(synthetic_relations(),parser,extract)
 write(root/(family_name+'-training.json'),dict(real_examples=real,unproposable_real=miss,synthetic_examples=synth,unproposable_synthetic=synth_missed))
 print('Association examples real/synthetic',len(real),len(synth),'unproposable',len(miss),flush=True)
 parsed=[parser.parse(r['text']) for r in dev];events=[extract(r['text'],p) for r,p in zip(dev,parsed)];base=[reconstruct(r['text'],p,e) for r,p,e in zip(dev,parsed,events)];all_results={};selected=None;best=-1
 for name,extra,kind,dependencies in [('R1-real-linear',False,'linear',True),('R2-contrast-linear',True,'linear',True),('R3-contrast-tree',True,'tree',True),('R4-without-dependencies',True,'tree',False)]:
  before=time.perf_counter();examples=real+(synth if extra else []);x=np.array([r['x'] for r in examples]);y=np.array([r['y'] for r in examples])
  if not dependencies:x[:,4]=0
  est=LogisticRegression(C=.2,class_weight='balanced',max_iter=1000).fit(x,y) if kind=='linear' else DecisionTreeClassifier(max_depth=3,min_samples_leaf=4,class_weight='balanced',random_state=52).fit(x,y)
  model=linear(est) if kind=='linear' else tree(est)
  if not dependencies:
   # The runtime model explicitly ignores dependency features too.
   def zero_dependency(item):
    if model['kind']=='tree':
     # Training with a constant dependency column means it cannot be split on.
     assert 4 not in model['tree']['feature']
   zero_dependency(model)
  comparisons=[];winner=None;winner_score=-1
  for threshold in [.4,.5,.6,.7,.8,.9]:
   for margin in [0.,.1]:
    pred=[]
    for row,p,e,b in zip(dev,parsed,events,base):
     edges,rejected=link(row['text'],p,e,model,threshold,margin);pred.append(dict(b,observations=edges,unattributed_observations=rejected))
    m=evaluate(dev,pred);precision=m['result_precision'] or 0;recall=m['result_recall'] or 0;f1=2*precision*recall/(precision+recall) if precision+recall else 0
    eligible=m['result_correct']>=4 and precision>=.8;score=(10 if eligible else 0)+f1-2*m['false_success']/max(1,m['result_claims'])
    comparisons.append(dict(threshold=threshold,margin=margin,metrics=m))
    if score>winner_score:winner_score=score;winner=dict(model=model,threshold=threshold,margin=margin,metrics=m,eligible=eligible);winning_predictions=pred
  all_results[name]=dict(seconds=time.perf_counter()-before,training_examples=len(examples),trained_kind=kind,synthetic_added=extra,dependency_features=dependencies,comparisons=comparisons,selection=winner)
  print(name,'best',winner['metrics']['result_correct'],winner['metrics']['result_claims'],'false-success',winner['metrics']['false_success'],'eligible',winner['eligible'],flush=True)
  if winner_score>best:best=winner_score;selected=dict(name=name,**winner);write(root/(family_name+'-selected-predictions.json'),winning_predictions)
 write(dest,dict(seconds=time.perf_counter()-start,models=all_results));write(root/(family_name+'-selection.json'),selected)

if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('root',type=Path);p.add_argument('--name',default='linker');a=p.parse_args();run(a.root,a.name)
