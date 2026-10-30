"""One independent v3 evaluation after selection and agent annotation."""
import argparse
from pathlib import Path
import json
import hashlib
import numpy as np
from common import read,write,near,Retriever,cached_encode,QUERIES
from events import EventHeads,CLASSES,metrics
from ranking import features,evaluate,rrf,scores_selected
from seal import verify

def run(root,old):
 verify(root);dest=root/'final-results.json'
 if dest.exists():raise ValueError('preserve first final evaluation')
 candidates=read(root/'final-candidates.jsonl');ann=json.loads((root/'final-annotations.json').read_text());labels={r['id']:r for r in ann}
 if set(labels)!={r['id'] for r in candidates}:raise ValueError('all reserved comments must be annotated')
 rows=[dict(r,**{k:v for k,v in labels[r['id']].items() if k!='id'}) for r in candidates]
 heads=EventHeads(root,old);event_results={};predictions={}
 for task,spanfield,label in [('modality','action_span','modality'),('outcome','observation_span','observation_outcome')]:
  units=[dict(label=r[label],text=r['text'][r[spanfield][0]:r[spanfield][1]],context=near(r['text'],r[spanfield])) for r in rows]
  prob=heads.predict(units,task);event_results[task]=metrics(units,prob,CLASSES[task]);predictions[task]=prob.tolist()
 frx=Retriever(old/'runtime-retrieval');original=Retriever(old/'runtime-frozen-control')
 f=features(root,rows,QUERIES,frx);o=features(root,rows,QUERIES,original);selection=json.loads((root/'ranking/selection.json').read_text())
 scores={name:s for name,s in [('tfidf',f[:,:,1]),('bm25',f[:,:,2]),('original-int8',o[:,:,0]),('original-window-max',o[:,:,4]),('frx-v2',f[:,:,0]),('frx-window-max',f[:,:,4])]}
 original_score=o[:,:,4 if selection['name']=='original-window-max' else 0]
 scores['selected']=scores_selected(f,rows,selection,original_score)
 retrieval={name:evaluate(rows,s) for name,s in scores.items()}
 baseline=retrieval['frx-v2'];chosen=retrieval['selected'];before={(r['app_id'],r['topic']):r for r in baseline['details']};apps=sorted({r['app_id'] for r in chosen['details']});rng=np.random.default_rng(308);diff=[]
 for _ in range(4000):
  sample=[r for app in rng.choice(apps,len(apps),replace=True) for r in chosen['details'] if r['app_id']==app]
  diff.append([float(np.mean([r[k]-before[(r['app_id'],r['topic'])][k] for r in sample])) for k in ['p3','recall5','ndcg5','mrr']])
 intervals={k:np.quantile(np.array(diff)[:,i],[.025,.975]).tolist() for i,k in enumerate(['p3','recall5','ndcg5','mrr'])}
 result=dict(status='first-v3-new-game-evaluation; provisional-agent-labels',n=len(rows),games=apps,
  annotation_sha256=hashlib.sha256((root/'final-annotations.json').read_bytes()).hexdigest(),event_results=event_results,
  retrieval_selection=selection,retrieval=retrieval,selected_minus_v2_game_cluster_intervals=intervals,
  automatic_interpretation_approved=False,execution_authorized=False)
 with dest.open('x') as file:json.dump(result,file,indent=2)
 write(root/'final-event-probabilities.json',predictions)
 print(json.dumps(dict(events=event_results,retrieval={k:v['summary'] for k,v in retrieval.items()},intervals=intervals),indent=2))

if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('root',type=Path);p.add_argument('old',type=Path);a=p.parse_args();run(a.root,a.old)
