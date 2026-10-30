"""One-shot final measurement. Never use these outputs for tuning."""
from experiment import *
from nli import NLI
import joblib,subprocess,math

def wilson(k,n):
 if not n:return [0,1]
 z=1.96;p=k/n;den=1+z*z/n;center=(p+z*z/(2*n))/den;delta=z*math.sqrt(p*(1-p)/n+z*z/(4*n*n))/den;return [center-delta,center+delta]
def run(root):
 out=root/'experiments/final-results.json'
 if out.exists():raise RuntimeError('Final already evaluated; preserve first result, no re-tuning.')
 freeze=json.loads((root/'frozen-selection.json').read_text())
 for f,digest in freeze['hashes'].items():assert hashlib.sha256((root/f).read_bytes()).hexdigest()==digest
 tr=load(root/'train.jsonl');dv=load(root/'dev.jsonl');rows=load(root/'final.jsonl');assert not {x['thread'] for x in rows}&{x['thread'] for x in tr+dv}
 start=time.time();features=NLI(root/'nli').features(rows);np.save(root/'experiments/final-nli.npy',features)
 report=joblib.load(root/'experiments/reportability-research.joblib')['report_nli'];p=report.predict_proba(features)[:,1];ids=np.flatnonzero(p>=.5);correct=int(sum(rows[i]['gold']!=0 for i in ids))
 res={'n':len(rows),'threads':len({x['thread'] for x in rows}),'known_reports':sum(x['gold']!=0 for x in rows),'report_detection':dict(accepted=len(ids),correct=correct,precision=correct/len(ids) if len(ids) else None,wilson95=wilson(correct,len(ids)),coverage=len(ids)/len(rows),errors=[rows[i]['id'] for i in ids if rows[i]['gold']==0])}
 zero=features.reshape(len(rows),5,3)[:,:,1].copy();zero/=zero.sum(1,keepdims=True).clip(.0001);res['outcome_nli']=metrics(rows,zero,.7,True);res['outcome_nli']['wilson95']=wilson(res['outcome_nli']['correct'],res['outcome_nli']['accepted'])
 rust=subprocess.run([str(root/'rust-target/debug/frx-ai-rule-baseline')],input=''.join(json.dumps(x)+'\n' for x in rows),text=True,capture_output=True,check=True);pred=[json.loads(l) for l in rust.stdout.splitlines()];lm={'positive':1,'not-resolved':2,'worsened':3,'mixed':4};a=[(r,y) for r,y in zip(rows,pred) if y];res['existing_rules']={'accepted':len(a),'outcome_correct':sum(lm[y['outcome']]==r['gold'] for r,y in a),'action_outcome_correct':sum(lm[y['outcome']]==r['gold'] and ('disable' if y['disabled'] else 'enable')==r['gold_state'] for r,y in a)}
 res['seconds']=time.time()-start;res['peakRSS_KiB']=resource.getrusage(resource.RUSAGE_SELF).ru_maxrss;res['automatic_acceptance']=False
 out.write_text(json.dumps(res,indent=2));print(json.dumps(res,indent=2))
if __name__=='__main__':
 import sys;run(pathlib.Path(sys.argv[1]))
