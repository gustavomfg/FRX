"""Predeclared comparisons. Training/dev only; final is never opened here."""
import argparse,json,pathlib,time,hashlib,resource
import numpy as np
from sklearn.linear_model import LogisticRegression
from sklearn.feature_extraction.text import TfidfVectorizer
from sklearn.pipeline import FeatureUnion
from sklearn.metrics import confusion_matrix
from core import representation,state_of,guards,extra_features,LABELS
from encoder import Encoder

def load(path):return [json.loads(l) for l in path.read_text().splitlines() if l.strip()]
def metrics(rows,prob,threshold,guard=False):
    pred=prob.argmax(1); conf=prob.max(1); accepted=[i for i,x in enumerate(rows) if pred[i]!=0 and conf[i]>=threshold and (not guard or not guards(x))]
    correct=[i for i in accepted if pred[i]==rows[i]['gold'] and state_of(rows[i])==rows[i]['gold_state']]
    # Report state separately as well as full action-outcome correctness.
    inversions=[i for i in accepted if (pred[i]==1 and rows[i]['gold'] in (2,3)) or (pred[i] in (2,3) and rows[i]['gold']==1)]
    return dict(accepted=len(accepted),correct=len(correct),precision=len(correct)/len(accepted) if accepted else None,coverage=len(accepted)/len(rows),known_recall=len(correct)/max(1,sum(x['gold']!=0 for x in rows)),inversions=len(inversions),false_success=int(sum(pred[i]==1 and rows[i]['gold']!=1 for i in accepted)),errors=[dict(id=rows[i]['id'],gold=LABELS[rows[i]['gold']],predicted=LABELS[pred[i]],state=state_of(rows[i]),gold_state=rows[i]['gold_state'],confidence=round(float(conf[i]),3)) for i in accepted if i not in correct])
def main(root):
    tr=load(root/'train.jsonl');dev=load(root/'dev.jsonl');out=root/'experiments';out.mkdir(exist_ok=True);Y=np.array([x['gold'] for x in tr]);start=time.time()
    # Predeclared four comparisons, constant regularization/seed, no final tuning.
    texts=[representation(x,False) for x in tr+dev];local=[representation(x) for x in tr+dev]
    vec=TfidfVectorizer(ngram_range=(1,2),min_df=1,sublinear_tf=True,max_features=10000)
    X=vec.fit_transform(texts[:len(tr)]);D=vec.transform(texts[len(tr):]);baseline=LogisticRegression(C=2,max_iter=1000,class_weight='balanced',random_state=42).fit(X,Y)
    probs={'e1_whole_comment_tfidf':baseline.predict_proba(D)}
    enc=Encoder(root/'encoder');emb=enc.encode(local);eX=np.hstack([emb,np.array([extra_features(x) for x in tr+dev])*.25]);head=LogisticRegression(C=2,max_iter=1000,class_weight='balanced',random_state=42).fit(eX[:len(tr)],Y)
    probs['e2_local_encoder']=head.predict_proba(eX[len(tr):])
    vector=FeatureUnion([('word',TfidfVectorizer(ngram_range=(1,2),sublinear_tf=True,max_features=8000)),('char',TfidfVectorizer(analyzer='char_wb',ngram_range=(3,5),sublinear_tf=True,max_features=12000))]);TX=vector.fit_transform(local[:len(tr)]);TD=vector.transform(local[len(tr):]);linear=LogisticRegression(C=2,max_iter=1000,class_weight='balanced',random_state=42).fit(TX,Y)
    probs['e3_local_word_char']=linear.predict_proba(TD)
    # Product-of-experts agreement with deterministic action/trust gates.
    p=(probs['e2_local_encoder']*probs['e3_local_word_char'])**.5;p/=p.sum(1,keepdims=True);probs['e4_hybrid']=p
    results={}
    for name,prob in probs.items():
        results[name]={str(t):metrics(dev,prob,t,name=='e4_hybrid') for t in [.0,.5,.6,.7,.8,.9]}
    (out/'dev-results.json').write_text(json.dumps(results,indent=2));(out/'dev-context.json').write_text(json.dumps([dict(id=x['id'],local=representation(x),state=state_of(x)) for x in dev],indent=2))
    # Data artifacts only; never deserialize untrusted pickles in prototype.
    import joblib
    joblib.dump({'encoder_head':head,'vectorizer':vector,'linear':linear},out/'research-only.joblib')
    np.savez(out/'dev-probs.npz',**probs)
    print(json.dumps({n:{t:{k:v for k,v in m.items() if k!='errors'} for t,m in table.items()} for n,table in results.items()},indent=2))
    print('elapsed',time.time()-start,'peak_rss_KiB',resource.getrusage(resource.RUSAGE_SELF).ru_maxrss)
if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('root',type=pathlib.Path);main(p.parse_args().root)
