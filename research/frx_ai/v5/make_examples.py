"""Materialize seven requested demonstration kinds from known cached sources."""
import argparse
from pathlib import Path
from common import read,jsonl
from replay_study import make
def run(root,old4):
 snapshots,_=make(root,old4);cases=[s['input'] for s in snapshots];rows=read(root/'final-audited.jsonl')
 for i,name,symptom in [(12,'failed-reported-procedure','Audio desynchronization after trying several changes.'),(17,'reported-composed-procedure','Windowed mode after a combined system replacement.')]:
  r=rows[i];cases.append(dict(id=name,context=dict(app_id=r['app_id']),symptom=symptom,evidence=[dict(app_id=r['app_id'],url=r['url'],text=r['text'])],origin='heldout-public-source-demonstration; not-local-telemetry'))
 jsonl(root/'examples.jsonl',cases);print('Prepared',len(cases),'known-source/control examples; not new independent cases')
if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('root',type=Path);p.add_argument('old4',type=Path);a=p.parse_args();run(a.root,a.old4)
