"""Reproduce preserved v2 retrieval on the now-known v3 cohort. No fitting."""
import argparse,json,sys
from pathlib import Path
import numpy as np

V3=Path(__file__).resolve().parents[1]/'v3'
sys.path.append(str(V3))
from common import read,write,Retriever,QUERIES
from ranking import evaluate,bm25

def run(root,old,v3root):
 raw=read(v3root/'final-candidates.jsonl');labels={r['id']:r for r in json.loads((v3root/'final-annotations.json').read_text())}
 rows=[dict(r,**{k:v for k,v in labels[r['id']].items() if k!='id'}) for r in raw];texts=[r['text'] for r in rows];queries=list(QUERIES.values());results={}
 for name,directory in [('frx-v2','runtime-retrieval'),('original-int8','runtime-frozen-control')]:
  engine=Retriever(old/directory);scores=engine.encode(queries)@engine.encode(texts).T;results[name]=evaluate(rows,scores)
 results['bm25']=evaluate(rows,bm25(texts,queries))
 prior=json.loads((v3root/'final-results.json').read_text())['retrieval'];consistent=all(abs(results[name]['summary'][k]-prior[name]['summary'][k])<1e-10 for name in ['frx-v2','original-int8','bm25'] for k in results[name]['summary'])
 result=dict(status='replay of known v3 data; no new independent retrieval test',model_changed=False,identical_rank_metrics_to_v3=consistent,results=results)
 write(root/'retrieval-replay.json',result);print(json.dumps({k:v['summary'] for k,v in results.items()},indent=2))
 if not consistent:raise ValueError('preserved retrieval results changed; inspect without tuning')

if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('root',type=Path);p.add_argument('old',type=Path);p.add_argument('v3',type=Path);a=p.parse_args();run(a.root,a.old,a.v3)
