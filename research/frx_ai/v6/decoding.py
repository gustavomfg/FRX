"""Replay unchanged fitted weights after separately recorded structural controls."""
import argparse,json,time
from pathlib import Path
import numpy as np
from sklearn.linear_model import LogisticRegression
from bridge import read,write,sha,immutable,CompactSyntax,retriever
from classifier import Classifier
from learning import choose,linear
from engine import Interpreter
from evaluate import evaluate
def run(root,old,name='decoded-development'):
 immutable(root);dest=root/(name+'.json')
 if dest.exists():raise ValueError('preserve decoding study')
 rows=read(root/'development-gold-v2.jsonl');parser=CompactSyntax(root);catalog=json.loads((root/'catalog.json').read_text());results={};start=time.perf_counter()
 for name in ['L1-structured-real-text','L2-structured-contrast','L3-semantic-contrast']:
  path=root/'verifiers'/(name+'.json');verifier=Classifier(path,retriever(old) if name.startswith('L3') else None);record,pred=choose(root,rows,parser,catalog,verifier,name,[.3,.4,.5,.6,.7,.8]);record['model_sha256']=sha(path);results[name]=record;write(root/(name+'-decoded-predictions.json'),pred)
 for threshold in [-.01,.5,1.5,2.5]:
  engine=Interpreter(parser,catalog,threshold=threshold);pred=[engine.analyze(r) for r in rows];results['structural-'+str(threshold)]=dict(selection=dict(name='structural',threshold=threshold,metrics=evaluate(rows,pred)))
 # Paired deterministic refit uses the exact stored claim matrices, independent
 # of parser cache/RNG initialization. It is a reproduction, not a new model search.
 data=[r for r in read(root/'training-pairs.jsonl') if not r['source_origin'].startswith('explicit-synthetic')];x=np.array([r['x'] for r in data]);y=np.array([r['y'] for r in data]);again=linear(LogisticRegression(C=.5,class_weight='balanced',max_iter=1500,random_state=607).fit(x,y));original=json.loads((root/'verifiers/L1-structured-real-text.json').read_text());delta=float(np.max(np.abs(np.array(again['coefficients'])-np.array(original['coefficients']))));reproduction=dict(max_coefficient_difference=delta,identical_coefficients=delta==0,training_pairs_sha256=sha(root/'training-pairs.jsonl'),seed=607,not_new_model_selection=True)
 write(dest,dict(results=results,reproduction=reproduction,seconds=time.perf_counter()-start))
 for k,r in results.items():
  m=r['selection']['metrics'];print(k,m['relation_correct'],m['relations'],m['false_success'],r['selection'].get('eligible'),flush=True)
if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('root',type=Path);p.add_argument('old',type=Path);p.add_argument('--name',default='decoded-development');a=p.parse_args();run(a.root,a.old,a.name)
