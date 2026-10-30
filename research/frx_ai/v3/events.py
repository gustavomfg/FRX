"""Learn narrowly scoped speech-act/outcome heads, not causal diagnosis.

Inference emits unverified candidates. Literal ordering is not temporal proof.
"""
import argparse
from pathlib import Path
import time
import re
import numpy as np
from sklearn.feature_extraction.text import TfidfVectorizer
from sklearn.pipeline import make_pipeline
from sklearn.linear_model import LogisticRegression
from sklearn.preprocessing import StandardScaler
from sklearn.metrics import classification_report,confusion_matrix,f1_score
import joblib
from common import read,write,near,cached_encode,Retriever,preserve_fit

CLASSES={'modality':['unknown','suggested','tested'],'outcome':['unknown','positive','not-resolved','worsened','mixed']}
ACTION=re.compile(r'\b(?:enabl\w*|disabl\w*|install\w*|uninstall\w*|replac\w*|remov\w*|updat\w*|upgrad\w*|downgrad\w*|switch\w*|chang\w*|cop(?:y|ied|ying)|renam\w*|set(?:ting)?|PROTON_[A-Z0-9_]+|VKD3D_CONFIG|DXVK_[A-Z0-9_]+)\b',re.I)

def examples(rows,task):
 result=[]
 for r in rows:
  if task=='modality':span=r['action_span'];label=r['modality']
  else:
   span=r.get('result_span') or r['action_span'];label=r['outcome'] if r.get('result_span') else 'unknown'
  result.append(dict(id=r['id'],url=r['url'],app_id=r['app_id'],text=r['text'][span[0]:span[1]],
                     context=near(r['text'],span),label=label,span=span))
 return result

def metrics(rows,prob,classes,threshold=.8):
 y=np.array([classes.index(x['label']) for x in rows]);pred=prob.argmax(1);conf=prob.max(1)
 selected=(pred!=0)&(conf>=threshold);n=int(selected.sum());correct=int(((y==pred)&selected).sum())
 return dict(n=len(y),macro_f1=float(f1_score(y,pred,labels=range(len(classes)),average='macro',zero_division=0)),
  classes=classification_report(y,pred,labels=list(range(len(classes))),target_names=classes,zero_division=0,output_dict=True),
  confusion=confusion_matrix(y,pred,labels=list(range(len(classes)))).tolist(),accepted=n,correct=correct,
  precision=correct/n if n else None,coverage=n/len(y),
  false_tested=int(((pred==2)&(y!=2)&selected).sum()) if classes[2]=='tested' else None,
  false_success=int(((pred==1)&(y!=1)&selected).sum()) if classes[1]=='positive' else None)

def design(root,rows,mode,encoder=None):
 texts=[r['context'] if mode.endswith('context') else r['text'] for r in rows]
 return texts if mode=='tfidf' else cached_encode(root,'event-'+mode,encoder,texts)

def fit(root,old):
 preserve_fit(root);out=root/'events';out.mkdir(exist_ok=True)
 train=read(old/'train-events.jsonl')+read(old/'dev-events.jsonl');dev=read(old/'final-events.jsonl')
 assert not {r['app_id'] for r in train}&{r['app_id'] for r in dev}
 manifest={};records=[]
 original=Retriever(old/'runtime-frozen-control');trained=Retriever(old/'runtime-retrieval')
 for task in CLASSES:
  tr=examples(train,task);dv=examples(dev,task);y=np.array([CLASSES[task].index(r['label']) for r in tr]);best=-1
  for mode,enc in [('tfidf',None),('original-context',original),('frx-context',trained)]:
   x=design(root,tr,mode,enc);z=design(root,dv,mode,enc)
   for c in [.3,3.]:
    name=f'{task}-{mode}-C{c}';start=time.perf_counter()
    estimator=make_pipeline(TfidfVectorizer(ngram_range=(1,2),sublinear_tf=True,max_features=20000),LogisticRegression(C=c,max_iter=1500,class_weight='balanced')) if mode=='tfidf' else make_pipeline(StandardScaler(),LogisticRegression(C=c,max_iter=1500,class_weight='balanced'))
    estimator.fit(x,y);prob=estimator.predict_proba(z);m=metrics(dv,prob,CLASSES[task]);record=dict(name=name,family='event',seconds=time.perf_counter()-start,task=task,mode=mode,C=c,dev=m)
    records.append(record);write(out/(name+'.json'),record)
    if m['macro_f1']>best:
     best=m['macro_f1'];manifest[task]=dict(name=name,mode=mode,classes=CLASSES[task],threshold=.8)
     joblib.dump(estimator,out/(task+'.joblib'))
    print(name,round(m['macro_f1'],3),m['accepted'],m['precision'],flush=True)
 write(out/'selection.json',manifest);write(out/'experiments.json',records)

class EventHeads:
 def __init__(self,root,old):
  import json
  self.meta=json.loads((root/'events/selection.json').read_text());self.heads={k:joblib.load(root/'events'/(k+'.joblib')) for k in self.meta}
  self.encoders={}
  for k,m in self.meta.items():
   if m['mode']!='tfidf':
    path=old/('runtime-frozen-control' if m['mode'].startswith('original') else 'runtime-retrieval')
    if m['mode'] not in self.encoders:self.encoders[m['mode']]=Retriever(path)
 def predict(self,rows,task):
  m=self.meta[task];texts=[r['context'] if m['mode'].endswith('context') else r['text'] for r in rows]
  x=texts if m['mode']=='tfidf' else self.encoders[m['mode']].encode(texts)
  return self.heads[task].predict_proba(x)

if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('root',type=Path);p.add_argument('old',type=Path);a=p.parse_args();fit(a.root,a.old)
