"""Restricted learned task: is there an attributable reported result? No polarity claim."""
from experiment import *
import joblib

def run(root):
 tr=load(root/'train.jsonl');dev=load(root/'dev.jsonl');enc=Encoder(root/'encoder');f=enc.encode([representation(x) for x in tr+dev]);nli=np.load(root/'experiments/nli-train-dev.npy');Y=[int(x['gold']!=0) for x in tr];res={};models={}
 for name,features in [('report_encoder',f),('report_nli',nli),('report_both',np.hstack([f,nli]))]:
  model=LogisticRegression(C=2,max_iter=1500,class_weight='balanced',random_state=42).fit(features[:len(tr)],Y);p=model.predict_proba(features[len(tr):])[:,1];models[name]=model
  res[name]={}
  for t in [.5,.6,.7,.8,.9]:
   ids=np.flatnonzero(p>=t);correct=int(sum(dev[i]['gold']!=0 for i in ids));res[name][str(t)]=dict(accepted=len(ids),correct=correct,precision=correct/len(ids) if len(ids) else None)
 print(json.dumps(res,indent=2));(root/'experiments/reportability.json').write_text(json.dumps(res,indent=2));joblib.dump(models,root/'experiments/reportability-research.joblib')
if __name__=='__main__':
 import sys;run(pathlib.Path(sys.argv[1]))
