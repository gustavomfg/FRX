from experiment import *
from factor import focus,represented
from synthetic import generate
import joblib

def run(root):
 real=load(root/'train.jsonl');dev=load(root/'dev.jsonl');syn=generate();rows=real+syn;enc=Encoder(root/'encoder');texts=[represented(x) for x in rows+dev]
 emb=enc.encode(texts);vec=FeatureUnion([('word',TfidfVectorizer(ngram_range=(1,3),max_features=12000,sublinear_tf=True)),('char',TfidfVectorizer(analyzer='char_wb',ngram_range=(3,5),max_features=12000,sublinear_tf=True))]);X=vec.fit_transform(texts[:len(rows)]);D=vec.transform(texts[len(rows):]);weights=[4.]*len(real)+[.5]*len(syn);ys=[x['gold'] for x in rows];res={};models={}
 for name,A,B in [('focused_lexical',X,D),('focused_encoder',emb[:len(rows)],emb[len(rows):])]:
  model=LogisticRegression(C=8,max_iter=1500,class_weight='balanced',random_state=42).fit(A,ys,sample_weight=weights);p=model.predict_proba(B);models[name]=model
  res[name]={str(t):metrics(dev,p,t,False) for t in [0,.4,.5,.6,.7,.8,.9]}
  print(name,[(t,m['accepted'],m['correct'],m['inversions'],m['false_success']) for t,m in res[name].items()])
  print('errors .5',res[name]['0.5']['errors'])
 (root/'experiments/focused-results.json').write_text(json.dumps(res,indent=2));joblib.dump(dict(models=models,vector=vec),root/'experiments/focused-research-only.joblib')
 (root/'experiments/focused-dev.json').write_text(json.dumps([dict(id=x['id'],text=focus(x),gold=x['gold'],state=x['gold_state']) for x in dev],indent=2))
if __name__=='__main__':
 import sys;run(pathlib.Path(sys.argv[1]))
