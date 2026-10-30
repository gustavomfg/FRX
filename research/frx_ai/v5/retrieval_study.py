"""Known v3 cohort control: raw comments versus literal incident summaries.

Same preserved encoder, same topic queries, same per-AppID candidates. These
are not independent new retrieval data and summaries are not confirmed causes.
"""
import argparse,json,importlib.util,time
from pathlib import Path
import numpy as np
from common import read,write,HERE
from lab import retriever
from syntax import Syntax
from events import extract
from relations import reconstruct

def evaluate(rows,scores,topics):
 out=[]
 for app in sorted({r['app_id'] for r in rows}):
  ids=[i for i,r in enumerate(rows) if r['app_id']==app]
  for j,topic in enumerate(topics):
   gold={i for i in ids if topic in rows[i].get('topics',[])}
   if not gold:continue
   order=sorted(ids,key=lambda i:(-float(scores[j,i]),i));hit=[int(i in gold) for i in order[:5]];ideal=sum(1/np.log2(k+2) for k in range(min(5,len(gold))))
   out.append(dict(app_id=app,topic=topic,ndcg5=float(sum(h/np.log2(k+2) for k,h in enumerate(hit))/ideal),mrr=next(1/(k+1) for k,i in enumerate(order) if i in gold),p3=sum(i in gold for i in order[:3])/min(3,len(order)),recall5=sum(hit)/len(gold)))
 return dict(summary={k:float(np.mean([r[k] for r in out])) for k in ['ndcg5','mrr','p3','recall5']},details=out)
def run(root,old,v3root):
 dest=root/'incident-retrieval.json'
 if dest.exists():raise ValueError('preserve readout')
 spec=importlib.util.spec_from_file_location('known_v3_queries',HERE.parent/'v3/common.py');m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m)
 labels={r['id']:r for r in json.loads((v3root/'final-annotations.json').read_text())};rows=[dict(r,topics=labels[r['id']]['topics']) for r in read(v3root/'final-candidates.jsonl')];parser=Syntax(root);summaries=[];start=time.perf_counter()
 for row in rows:
  text=row['text'];views=[]
  for unit in m.units(text)[:20]:
   t=text[slice(*unit)]
   if len(t)>1100:continue
   p=parser.parse(t);r=reconstruct(t,p,extract(t,p))
   for e in r['events']:
    views.append(e['status']+' '+e['direction']+' '+e['trigger']['text']+' '+ ' '.join(a['text'] for a in e['arguments']))
   views.extend(o['outcome']+' '+o['evidence']['text'] for o in r['observations']+r['unattributed_observations'])
  summaries.append(' ; '.join(views) or text[:900])
 encoder=retriever(old);q=encoder.encode(list(m.QUERIES.values()));raw=q@encoder.encode([r['text'] for r in rows]).T;structured=q@encoder.encode(summaries).T
 variants={'comments':raw,'structured-incidents':structured,'mean-score-hybrid':(raw+structured)/2};result={k:evaluate(rows,s,m.QUERIES) for k,s in variants.items()}
 write(dest,dict(status='known-data representation control; not new independent validation',seconds=time.perf_counter()-start,results=result,encoder_changed=False,labels='old agent provisional',summary_policy='literal actions/statuses/observations; no guessed diagnosis',runtime_selection='preserve original v2 comment retrieval regardless of this exploratory result'))
 print(json.dumps({k:v['summary'] for k,v in result.items()},indent=2))
if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('root',type=Path);p.add_argument('old',type=Path);p.add_argument('v3',type=Path);a=p.parse_args();run(a.root,a.old,a.v3)
