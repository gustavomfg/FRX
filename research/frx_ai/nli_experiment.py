from experiment import *
from nli import NLI
import joblib

def run(root):
 tr=load(root/'train.jsonl');dev=load(root/'dev.jsonl');path=root/'experiments/nli-train-dev.npy';start=time.time()
 if path.exists():f=np.load(path)
 else:f=NLI(root/'nli').features(tr+dev);np.save(path,f)
 zero=f[len(tr):].reshape(len(dev),5,3)[:,:,1].copy();zero/=zero.sum(1,keepdims=True).clip(.0001)
 head=LogisticRegression(C=2,max_iter=1500,class_weight='balanced',random_state=42).fit(f[:len(tr)],[x['gold'] for x in tr]);p=head.predict_proba(f[len(tr):]);results={}
 for name,probs in [('nli_zero_shot',zero),('nli_trained_head',p)]:
  results[name]={str(t):metrics(dev,probs,t,True) for t in [0,.4,.5,.6,.7,.8,.9]};print(name,[(t,m['accepted'],m['correct'],m['inversions'],m['false_success']) for t,m in results[name].items()]);print(results[name]['0.5']['errors'])
 (root/'experiments/nli-results.json').write_text(json.dumps(results,indent=2));joblib.dump(head,root/'experiments/nli-head-research.joblib');print('seconds',time.time()-start,'peakRSS',resource.getrusage(resource.RUSAGE_SELF).ru_maxrss)
if __name__=='__main__':
 import sys;run(pathlib.Path(sys.argv[1]))
