"""Publish offsets, provenance and metrics, never source bodies or weights."""
import argparse,json,copy
from pathlib import Path
from bridge import HERE,read,write,sha

def clean(value):
 if isinstance(value,dict):return {k:clean(v) for k,v in value.items() if k not in ['text','source_text','reporter_login','directory','quote']}
 if isinstance(value,list):return [clean(v) for v in value]
 return value

def publish(root):
 dest=HERE/'snapshot';dest.mkdir(exist_ok=True)
 for name in ['development-reserved','development-gold','development-gold-v2','acceptance-reserved','identity-challenge-reserved','context-challenge-reserved','acceptance-gold']:
  write(dest/(name+'.json'),clean(read(root/(name+'.jsonl'))))
 for name in ['catalog','compact-manifest','syntax-manifest','reservation','identity-challenge-reservation','context-challenge-reservation','selection','frozen','label-lock','offset-repair','frozen-original-before-offset-repair','final-results','posthoc-metrics','split-audit','dependency-versions','pair-data-audit','development-annotation-erratum','prefreeze-development','neural-decoding','diagnostic-replay','diagnostic-probes','replay-labels','efficiency','efficiency-v5-paired','no-torch-inference','reproduction','preservation','runtime-smoke']:
  file=root/(name+'.json')
  if file.exists():write(dest/file.name,clean(json.loads(file.read_text())))
 model=root/'verifiers/L1-structured-real-text.json'
 # Even the tiny verifier lives outside Git, so all distribution is explicit.
 write(dest/'artifact-reference.json',dict(classifier=dict(path=str(model.relative_to(root)),bytes=model.stat().st_size,sha256=sha(model)),old_v2_model='preserved runtime-retrieval; see frozen.json hashes',weights_in_git=False))
 fitted=[]
 for name in ['L1-structured-real-text','L2-structured-contrast','L3-semantic-contrast']:
  source=json.loads((root/'prefreeze-development.json').read_text())['results'][name];fitted.append(dict(id=name,family='multiclass-logistic',learned=True,seed=606,result=clean(source)))
 for name in ['N1-real-text','N2-contrast','N3-structured-contrast']:
  meta=json.loads((root/'neural'/(name+'.json')).read_text());score=json.loads((root/'neural-decoding.json').read_text())['results'][name];fitted.append(dict(id=name,family='MiniLM-cross-encoder',learned=True,metadata=clean(meta),result=clean(score)))
 write(dest/'experiments.json',dict(exploratory_fitted_configurations=6,reproduction_fits=2,actual_fit_executions=8,unrun_configurations=['N4-seed-control'],fitted=fitted,other_controls=['initial vs expanded structural extraction','completed-change vs preexisting/purpose/temporal constraints','complete-group vs individual proximity','frozen v2 semantic features','existing small NLI verifier (not trained here)','six final comparators','known incident replay','Torch-blocked inference','identical-input deterministic reproduction'],limits='Threshold points, grammar revisions, test cases and replay steps are not counted as additional trained models.'))
 queue=[]
 errors=json.loads((root/'posthoc-errors.json').read_text())['records'][0]['errors']
 for row in errors:
  reasons=sorted({e['kind'] for e in row['failures']});queue.append(dict(id=row['id'],url=row['url'],review_priority='human review required',reasons=reasons,source_body_in_git=False))
 write(dest/'human-review-queue.json',queue)
 manifest={p.name:sha(p) for p in dest.glob('*.json') if p.name!='manifest.json'};write(dest/'manifest.json',dict(files=manifest,source_bodies=False,weights=False,labels='agent-provisional; no independent human review'))
 print('Published',len(manifest),'source-free metadata files')
if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('root',type=Path);publish(p.parse_args().root)
