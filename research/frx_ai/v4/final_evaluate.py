"""One new-game test, comparing the same audited event schema and source units."""
import argparse
from pathlib import Path
import json,hashlib
import numpy as np
from data import read,write
from native import NativeEvents
from proof import verify
from proposals import propose
from metrics import evaluate
from study import legacy_status
from seal import verify as verify_seal

def run(root,old,v3root):
 verify_seal(root);dest=root/'final-results.json'
 if dest.exists():raise ValueError('preserve first independent test')
 rows=read(root/'final-audited.jsonl');source=read(root/'final-paragraphs.jsonl')
 if [r['id'] for r in rows]!=[r['id'] for r in source]:raise ValueError('complete reserved cohort required')
 for row,original in zip(rows,source):
  if row['text']!=original['text'] or row['source_sha256']!=original['source_sha256']:raise ValueError('annotated source mismatch')
 runtime=NativeEvents(root/'runtime-events');variants=dict(native_verified=runtime.predict(rows),native_ungated=runtime.predict(rows,verified=False),fixed_lexical_verifier=[propose(r['text']) for r in rows],training_lexicon_verifier=[propose(r['text'],json.loads((root/'lexicon.json').read_text())) for r in rows])
 supplied=[]
 for row in rows:
  events=[item for e in row['events'] if (item:=verify(row['text'],e['trigger'])) is not None];supplied.append(dict(events=events,observations=[]))
 variants['supplied_spans_verifier']=supplied;variants['supplied_spans_v3_learned']=legacy_status(rows,old,v3root)
 metrics={name:evaluate(rows,pred,strict=True) for name,pred in variants.items()};apps=sorted({r['app_id'] for r in rows});rng=np.random.default_rng(401);bootstrap=[]
 # Paired game-cluster intervals: false-performed reduction on identical supplied triggers.
 for _ in range(3000):
  indices=[i for app in rng.choice(apps,len(apps),replace=True) for i,r in enumerate(rows) if r['app_id']==app]
  picked=[rows[i] for i in indices];a=evaluate(picked,[variants['supplied_spans_verifier'][i] for i in indices],strict=True);b=evaluate(picked,[variants['supplied_spans_v3_learned'][i] for i in indices],strict=True)
  bootstrap.append((b['false_performed']-a['false_performed'])/max(1,sum(len(r['events']) for r in picked)))
 m=metrics['native_verified'];n=m['performed_claims'];k=m['performed_correct'];wilson=None
 if n:
  p=k/n;z=1.96;den=1+z*z/n;center=(p+z*z/(2*n))/den;half=z*np.sqrt(p*(1-p)/n+z*z/(4*n*n))/den;wilson=[float(center-half),float(center+half)]
 result=dict(status='first-v4-new-game-evaluation; provisional-agent-labels',games=apps,paragraphs=len(rows),metrics=metrics,
  performed_wilson_descriptive=wilson,supplied_trigger_false_performed_reduction_cluster_interval=np.quantile(bootstrap,[.025,.975]).tolist(),
  final_annotation_sha256=hashlib.sha256((root/'final-audited.jsonl').read_bytes()).hexdigest(),automatic_approval=False,
  limitations=['Agent labels are not independent human validation.','Event-enriched bounded paragraphs; no population prevalence claim.','Supplied-span controls do not measure detection.','Result associations are experimental and disabled by default.'])
 with dest.open('x') as f:json.dump(result,f,indent=2)
 write(root/'final-predictions.json',variants)
 print(json.dumps({k:{a:v for a,v in m.items() if a!='details'} for k,m in metrics.items()},indent=2))

if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('root',type=Path);p.add_argument('old',type=Path);p.add_argument('v3',type=Path);a=p.parse_args();run(a.root,a.old,a.v3)
