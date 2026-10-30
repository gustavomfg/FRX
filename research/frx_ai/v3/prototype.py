"""Local JSONL investigation prototype. No APIs, downloads or action execution."""
import argparse
from pathlib import Path
import sys
import json
import hashlib
import numpy as np
from common import Retriever,units,near
from investigation import inspect
from events import EventHeads,ACTION
from prepare import RESULT
from ranking import windows,bm25,scores_selected
from sklearn.feature_extraction.text import TfidfVectorizer
from sklearn.metrics.pairwise import cosine_similarity

def rank_features(rows,query,encoder):
 texts=[r['text'] for r in rows];q=encoder.encode([query]);d=encoder.encode(texts);semantic=q@d.T
 spans=[windows(t) for t in texts];all_windows=encoder.encode([x for group in spans for x in group]);full=q@all_windows.T;maximum=[];start=0
 for group in spans:maximum.append(float(full[:,start:start+len(group)].max()));start+=len(group)
 vec=TfidfVectorizer(ngram_range=(1,2),sublinear_tf=True)
 try:x=vec.fit_transform(texts);lexical=cosine_similarity(vec.transform([query]),x)
 except ValueError:lexical=np.zeros_like(semantic)
 from common import words
 overlap=np.array([[len(set(words(t))&set(words(query)))/max(1,len(set(words(query)))) for t in texts]])
 length=np.array([[np.log1p(len(t))/10 for t in texts]])
 return np.stack([semantic,lexical,bm25(texts,[query])/10,overlap,np.array([maximum]),length],-1)

class Investigator:
 def __init__(self,root,old):
  frozen=json.loads((root/'frozen.json').read_text())
  for name,digest in frozen['artifacts'].items():
   path=root/name
   if path.parent.resolve() not in [(root/'events').resolve(),(root/'ranking').resolve()]:raise ValueError('artifact directory')
   if hashlib.sha256(path.read_bytes()).hexdigest()!=digest:raise ValueError('changed research artifact')
  self.selection=json.loads((root/'ranking/selection.json').read_text());self.encoder=Retriever(old/'runtime-retrieval');self.heads=EventHeads(root,old)
  self.original=Retriever(old/'runtime-frozen-control') if self.selection['name'].startswith('original-') else None

 def analyze(self,case):
  result=inspect(case);symptom=case.get('symptom','')
  if not isinstance(symptom,str) or len(symptom)>1000:raise ValueError('symptom size/type')
  allowed=[case['evidence'][s['index']] for s in result['sources']];ranked=[];events=[]
  if allowed:
   x=rank_features(allowed,symptom,self.encoder);original=None
   if self.original:
    original_features=rank_features(allowed,symptom,self.original)
    original=original_features[:,:,4 if self.selection['name']=='original-window-max' else 0]
   scores=scores_selected(x,allowed,self.selection,original)[0]
   for i in sorted(range(len(allowed)),key=lambda i:-float(scores[i])):
    row=allowed[i];ranked.append(dict(url=row['url'],score=float(scores[i]),score_means='topical-relevance-only',source_sha256=hashlib.sha256(row['text'].encode()).hexdigest()))
   # Preserve multiple event units; never assign a nearby outcome as a cause.
   for source in allowed[:8]:
    text=source['text'];candidates=units(text);candidates=[s for s in candidates if ACTION.search(text[s[0]:s[1]]) or RESULT.search(text[s[0]:s[1]])][:12]
    if not candidates:continue
    rows=[dict(text=text[a:b],context=near(text,[a,b])) for a,b in candidates]
    modality=self.heads.predict(rows,'modality');outcome=self.heads.predict(rows,'outcome')
    for k,(a,b) in enumerate(candidates):
     events.append(dict(source=source['url'],span=[a,b],quote=text[a:b],quote_sha256=hashlib.sha256(text[a:b].encode()).hexdigest(),
       speech_act_candidate=self.heads.meta['modality']['classes'][int(modality[k].argmax())],
       outcome_candidate=self.heads.meta['outcome']['classes'][int(outcome[k].argmax())],
       speech_act_score=float(modality[k].max()),outcome_score=float(outcome[k].max()),
       action_result_relation='unverified',causality_confirmed=False,accepted_automatically=False))
  result.update(symptom=symptom,retrieval_model=self.selection['name'],ranked_evidence=ranked,event_candidates=events,
    learned_components=['preserved-fine-tuned-evidence-encoder','trained-speech-act-and-observation-heads'],
    deterministic_components=['segmentation','context-constraints','supplied-log-patterns','history','graph-serialization'],
    automatic_interpretation_approved=False)
  return result

def main():
 p=argparse.ArgumentParser();p.add_argument('root',type=Path);p.add_argument('old',type=Path);a=p.parse_args();model=Investigator(a.root,a.old)
 for line in sys.stdin:
  try:
   if len(line)>300000:raise ValueError('request size bound')
   output=model.analyze(json.loads(line))
  except (ValueError,TypeError,KeyError) as e:output=dict(status='invalid-request',error=str(e),execution_authorized=False)
  print(json.dumps(output,ensure_ascii=False),flush=True)

if __name__=='__main__':main()
