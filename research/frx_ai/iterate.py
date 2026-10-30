"""Iteration after inspecting development errors; final remains unopened."""
from experiment import *
from synthetic import generate
import joblib

def run(root):
    real=load(root/'train.jsonl');dev=load(root/'dev.jsonl');syn=generate();rows=real+syn;ys=[r['gold'] for r in rows]
    enc=Encoder(root/'encoder');texts=[representation(x) for x in rows+dev];emb=enc.encode(texts)
    word=FeatureUnion([('word',TfidfVectorizer(ngram_range=(1,3),sublinear_tf=True,max_features=14000)),('char',TfidfVectorizer(analyzer='char_wb',ngram_range=(3,5),sublinear_tf=True,max_features=14000))]);X=word.fit_transform(texts[:len(rows)]);D=word.transform(texts[len(rows):]);weights=np.array([3.0]*len(real)+[.5]*len(syn))
    results={};models={}
    for mode in ['augmented_lexical','augmented_encoder']:
      A,B=(X,D) if mode.endswith('lexical') else (emb[:len(rows)],emb[len(rows):])
      head=LogisticRegression(C=8,max_iter=1500,class_weight='balanced',random_state=42).fit(A,ys,sample_weight=weights)
      p=head.predict_proba(B);models[mode]=head
      results[mode]={str(t):metrics(dev,p,t,True) for t in [0,.4,.5,.6,.7,.8,.9]}
    # State is a separate learned task, not inferred from an entire comment's sentiment.
    state_head=LogisticRegression(C=8,max_iter=1500,class_weight='balanced',random_state=42).fit(emb[:len(rows)],[x['gold_state'] for x in rows],sample_weight=weights)
    state_pred=state_head.predict(emb[len(rows):]);results['state_head_accuracy']=float(np.mean(state_pred==[x['gold_state'] for x in dev]))
    out=root/'experiments';(out/'iteration2.json').write_text(json.dumps(results,indent=2));joblib.dump(dict(models=models,vector=word,state_head=state_head),out/'augmented-research-only.joblib')
    for name,m in results.items():
      if isinstance(m,dict): print(name,[(t,v['accepted'],v['correct'],v['inversions'],v['false_success']) for t,v in m.items()])
      else:print(name,m)
    # Development errors are inspectable; final is explicitly absent.
    for mode in ['augmented_lexical','augmented_encoder']:
      print(mode,'errors at .5',results[mode]['0.5']['errors'])
if __name__=='__main__':
 import sys;run(pathlib.Path(sys.argv[1]))
