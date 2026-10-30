"""Small learned pairwise reranker; topic relevance is never fix efficacy."""
import argparse
from pathlib import Path
import json
import time
import math
from collections import Counter
import numpy as np
from sklearn.feature_extraction.text import TfidfVectorizer
from sklearn.metrics.pairwise import cosine_similarity
from sklearn.preprocessing import StandardScaler
from sklearn.linear_model import LogisticRegression
import joblib
from common import read,write,words,Retriever,cached_encode,QUERIES,TRAIN_QUERIES,preserve_fit

def windows(text):
 # Bounded overlapping views reduce loss from the encoder's 192-token limit.
 tokens=text.split();return [' '.join(tokens[i:i+130]) for i in range(0,min(len(tokens),780),100)][:6] or [text]

def bm25(texts,queries):
 docs=[Counter(words(t)) for t in texts];n=len(docs);length=np.array([sum(d.values()) for d in docs]);avg=max(1,length.mean())
 df=Counter(w for d in docs for w in d);out=[]
 for query in queries:
  s=np.zeros(n)
  for w in set(words(query)):
   freq=np.array([d[w] for d in docs]);idf=math.log(1+(n-df[w]+.5)/(df[w]+.5))
   s+=idf*freq*2.2/(freq+1.2*(.25+.75*length/avg))
  out.append(s)
 return np.array(out)

def features(root,rows,queries,encoder):
 texts=[r['text'] for r in rows];qt=list(queries.values())
 d=cached_encode(root,'rank-doc',encoder,texts);q=cached_encode(root,'rank-query',encoder,qt);semantic=q@d.T
 chunks=[windows(t) for t in texts];flat=[s for group in chunks for s in group]
 e=cached_encode(root,'rank-windows',encoder,flat);allscore=q@e.T;maximum=[];start=0
 for group in chunks:
  maximum.append(allscore[:,start:start+len(group)].max(1));start+=len(group)
 maximum=np.stack(maximum,1);lex=bm25(texts,qt)
 vec=TfidfVectorizer(ngram_range=(1,2),sublinear_tf=True)
 try:
  x=vec.fit_transform(texts);tfidf=cosine_similarity(vec.transform(qt),x)
 except ValueError:tfidf=np.zeros_like(semantic)
 overlap=np.array([[len(set(words(t))&set(words(query)))/max(1,len(set(words(query)))) for t in texts] for query in qt])
 length=np.broadcast_to(np.log1p([len(t) for t in texts])/10,semantic.shape)
 feature=np.stack([semantic,tfidf,lex/10,overlap,maximum,length],axis=-1)
 return feature

def evaluate(rows,scores,queries=QUERIES):
 details=[];missing=[]
 for app in sorted({r['app_id'] for r in rows}):
  ids=[i for i,r in enumerate(rows) if r['app_id']==app]
  for qi,topic in enumerate(queries):
   relevant={i for i in ids if topic in rows[i].get('topics',[])}
   order=sorted(ids,key=lambda i:(-float(scores[qi,i]),i))
   record=dict(app_id=app,topic=topic,n_candidates=len(ids),relevant=len(relevant),top_urls=[rows[i]['url'] for i in order[:5]],max_score=float(scores[qi,order[0]]))
   if not relevant:missing.append(record);continue
   hits=[int(i in relevant) for i in order[:5]];ideal=sum(1/np.log2(k+2) for k in range(min(5,len(relevant))))
   record.update(p3=sum(i in relevant for i in order[:3])/min(3,len(order)),recall5=sum(hits)/len(relevant),
                 ndcg5=float(sum(h/np.log2(k+2) for k,h in enumerate(hits))/ideal),mrr=next(1/(k+1) for k,i in enumerate(order) if i in relevant))
   details.append(record)
 return dict(n_comments=len(rows),answerable=len(details),unanswerable=len(missing),
             summary={k:float(np.mean([r[k] for r in details])) for k in ['p3','recall5','ndcg5','mrr']},details=details,unanswerable_queries=missing)

def rrf(scores_a,scores_b,rows,alpha):
 out=np.zeros_like(scores_a)
 for app in {r['app_id'] for r in rows}:
  ids=[i for i,r in enumerate(rows) if r['app_id']==app]
  for qi in range(len(scores_a)):
   for s,w in [(scores_a,alpha),(scores_b,1-alpha)]:
    order=sorted(ids,key=lambda i:-float(s[qi,i]))
    for k,i in enumerate(order):out[qi,i]+=w/(60+k+1)
 return out

def pair_data(rows,features,queries):
 x=[]
 for app in sorted({r['app_id'] for r in rows}):
  ids=[i for i,r in enumerate(rows) if r['app_id']==app]
  for qi,topic in enumerate(queries):
   positive=[i for i in ids if topic in rows[i].get('topics',[])];negative=[i for i in ids if topic not in rows[i].get('topics',[])]
   for a in positive:
    for b in negative:x.extend([features[qi,a]-features[qi,b],features[qi,b]-features[qi,a]])
 return np.array(x),np.tile([1,0],len(x)//2)

def fit(root,old):
 preserve_fit(root);out=root/'ranking';out.mkdir(exist_ok=True)
 train=read(old/'retrieval-train.jsonl')+read(old/'retrieval-dev.jsonl');dev=read(old/'final-events.jsonl')
 assert not {r['app_id'] for r in train}&{r['app_id'] for r in dev}
 frx=Retriever(old/'runtime-retrieval');original=Retriever(old/'runtime-frozen-control')
 a=features(root,train,TRAIN_QUERIES,frx);b=features(root,dev,QUERIES,frx)
 original_features=features(root,dev,QUERIES,original)
 results={name:evaluate(dev,s) for name,s in [('original-int8',original_features[:,:,0]),('original-window-max',original_features[:,:,4]),('frx-v2',b[:,:,0]),('tfidf',b[:,:,1]),('bm25',b[:,:,2]),('frx-window-max',b[:,:,4])]}
 for alpha in [.25,.5,.75]:results[f'rrf-{alpha}']=evaluate(dev,rrf(b[:,:,0],b[:,:,2],dev,alpha))
 records=[];best=max(v['summary']['ndcg5'] for v in results.values());selection=max(results,key=lambda k:results[k]['summary']['ndcg5']);manifest=dict(kind='baseline',name=selection)
 for mode,cols in [('hybrid',[0,1,2,3]),('window',[0,1,2,3,4,5])]:
  scaler=StandardScaler().fit(a[:,:,cols].reshape(-1,len(cols)))
  at=scaler.transform(a[:,:,cols].reshape(-1,len(cols))).reshape(a.shape[0],a.shape[1],len(cols));bv=scaler.transform(b[:,:,cols].reshape(-1,len(cols))).reshape(b.shape[0],b.shape[1],len(cols))
  x,y=pair_data(train,at,TRAIN_QUERIES)
  for c in [.03,.3,3.]:
   start=time.perf_counter();est=LogisticRegression(C=c,fit_intercept=False,max_iter=1500);est.fit(x,y)
   scores=bv@est.coef_[0];name=f'pairwise-{mode}-C{c}';m=evaluate(dev,scores);results[name]=m
   rec=dict(name=name,family='ranker',seconds=time.perf_counter()-start,dev=m['summary'],pairs=len(y),C=c,columns=cols);records.append(rec)
   if m['summary']['ndcg5']>best:
    best=m['summary']['ndcg5'];manifest=dict(kind='pairwise',name=name,columns=cols,weights=est.coef_[0].tolist(),mean=scaler.mean_.tolist(),scale=scaler.scale_.tolist())
   print(name,m['summary'],flush=True)
 write(out/'development.json',results);write(out/'experiments.json',records);write(out/'selection.json',manifest)
 print('Selected ranker:',manifest['name'],best,flush=True)

def scores_selected(features,rows,selection,original_scores=None):
 name=selection['name']
 if selection['kind']=='pairwise':
  x=features[:,:,selection['columns']];return ((x-np.array(selection['mean']))/np.array(selection['scale']))@np.array(selection['weights'])
 if name.startswith('original-'):
  if original_scores is None:raise ValueError('original encoder scores required')
  return original_scores
 if name.startswith('rrf-'):return rrf(features[:,:,0],features[:,:,2],rows,float(name[4:]))
 return features[:,:,[ 'frx-v2','tfidf','bm25','unused','frx-window-max'].index(name)]

if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('root',type=Path);p.add_argument('old',type=Path);a=p.parse_args();fit(a.root,a.old)
