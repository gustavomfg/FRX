"""Executable local v4 investigation. No fixes, shell execution or online inference."""
import argparse
from pathlib import Path
import json,sys,hashlib,re,importlib.util
import numpy as np
from data import authored_paragraphs,VERBS
from native import NativeEvents
from diagnosis import diagnose
from context_memory import ContextMemory

def retriever(directory):
 file=Path(__file__).resolve().parents[1]/'v2/retrieval_runtime.py';spec=importlib.util.spec_from_file_location('frx_v2_retrieval_runtime',file);module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module);return module.Retriever(directory)

def source_spans(result,offset,source_hash):
 for event in result['events']:
  for key in ['trigger','status_evidence','target_evidence']:
   item=event.get(key)
   if item:item.update(source_span=[v+offset for v in item['span']],source_sha256=source_hash)
 for obs in result.get('observations',[]):
  item=obs['evidence'];item.update(source_span=[v+offset for v in item['span']],source_sha256=source_hash)
 for edge in result.get('temporal',[]):
  item=edge['cue'];item.update(source_span=[v+offset for v in item['span']],source_sha256=source_hash)
 return result

class Intelligence:
 def __init__(self,root,old,memory=None,experimental_relations=False):
  from seal import verify
  verify(root);self.events=NativeEvents(root/'runtime-events');self.search=retriever(old/'runtime-retrieval')
  preserved=json.loads((Path(__file__).resolve().parents[1]/'v2/snapshot/frozen.json').read_text())['retrieval_runtime']
  if self.search.meta!=preserved:raise ValueError('preserved v2 retrieval model changed')
  self.memory=ContextMemory(memory) if memory else None;self.experimental_relations=experimental_relations
 def analyze(self,case):
  query=case.get('symptom','')
  if not isinstance(query,str) or not query.strip() or len(query)>1000:raise ValueError('bounded symptom text required')
  base=diagnose(case)
  if self.memory:
   case,namespace=self.memory.exchange(case);base=diagnose(case);base['memory_namespace']=namespace
  selected=[case['evidence'][r['index']] for r in base['sources']];ranked=[]
  if selected:
   scores=(self.search.encode([query])@self.search.encode([r['text'] for r in selected]).T)[0]
   order=sorted(range(len(selected)),key=lambda i:-float(scores[i]));selected=[selected[i] for i in order]
   ranked=[dict(url=case['evidence'][base['sources'][i]['index']]['url'],score=float(scores[i]),score_meaning='topical relevance only; not causal or safety probability') for i in order]
  units=[];metadata=[]
  for source in selected[:8]:
   text=source['text'];paragraphs=authored_paragraphs(text);paragraphs.sort(key=lambda s:-len(VERBS.findall(text[s[0]:s[1]])))
   for a,b in paragraphs[:4]:
    units.append(dict(text=text[a:b]));metadata.append(dict(source=source['url'],app_id=source['app_id'],source_offset=a,source_sha256=hashlib.sha256(text.encode()).hexdigest(),scope='source-thread attribution; event-level game/build identity not independently verified'))
    if len(units)>=16:break
   if len(units)>=16:break
  extracted=self.events.predict(units) if units else [];paragraphs=[]
  for row,meta,result in zip(units,metadata,extracted):
   result=source_spans(result,meta['source_offset'],meta['source_sha256'])
   result['experimental_result_associations']=result.pop('observations') if self.experimental_relations else []
   result.update(meta,source_paragraph=row['text'],automatic_interpretation_approved=False,
                 causal_effect_established=False,local_benchmark=False,execution_authorized=False)
   paragraphs.append(result)
  base.update(status='experimental-v4-investigation',case_origin=case.get('origin','caller-supplied-unverified'),ranked_evidence=ranked,event_paragraphs=paragraphs,
   automatic_interpretation_approved=False,result_association_mode='experimental' if self.experimental_relations else 'disabled-pending-validation',
   technology_detection='explicit supplied documentary facts only; GPU/DLL presence is insufficient',
   limitations=base['limitations']+['Neural event proposals and deterministic literal verification have limited coverage.','Community event assertions never enter local attempt memory.','English bounded paragraphs are the evaluated scope.'])
  return base

def main():
 p=argparse.ArgumentParser();p.add_argument('root',type=Path);p.add_argument('old',type=Path);p.add_argument('--memory',type=Path);p.add_argument('--experimental-relations',action='store_true');a=p.parse_args();engine=Intelligence(a.root,a.old,a.memory,a.experimental_relations)
 for line in sys.stdin:
  try:
   if len(line)>300000:raise ValueError('request size bound')
   out=engine.analyze(json.loads(line))
  except (ValueError,TypeError,KeyError) as e:out=dict(status='invalid-request',error=str(e),execution_authorized=False)
  print(json.dumps(out,ensure_ascii=False),flush=True)

if __name__=='__main__':main()
