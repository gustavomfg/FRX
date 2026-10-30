"""POST-EVALUATION conservative assembly of the preserved v2 retrieval baseline.

The frozen v3 challenger and its independent results are unchanged. This wrapper
uses the preregistered v2 baseline after the challenger failed; the combined
investigation workflow is not independently validated by that retrieval test.
Unreliable event-head predictions are available only with an explicit flag.
"""
import argparse
from pathlib import Path
import json
import sys
import hashlib
from common import Retriever
from investigation import inspect
from prototype import Investigator
from seal import verify
from memory import CaseMemory

class CaseReview:
 def __init__(self,root,old,experimental_events=False,memory=None):
  verify(root);self.encoder=Retriever(old/'runtime-retrieval')
  self.events=Investigator(root,old) if experimental_events else None
  self.memory=CaseMemory(memory) if memory else None
 def analyze(self,case):
  result=inspect(case);query=case.get('symptom','')
  if not isinstance(query,str) or not query.strip() or len(query)>1000:raise ValueError('bounded symptom text required')
  if self.memory:
   # Validate the complete input before recording anything. Neural predictions
   # and text-extracted outcome candidates never enter this journal.
   stored=self.memory.exchange(case['context'],case.get('attempts',[]))
   case=dict(case,attempts=stored);result=inspect(case)
  rows=[case['evidence'][s['index']] for s in result['sources']];ranked=[]
  if rows:
   similarity=(self.encoder.encode([query])@self.encoder.encode([r['text'] for r in rows]).T)[0]
   for i in sorted(range(len(rows)),key=lambda i:-float(similarity[i])):
    row=rows[i];ranked.append(dict(url=row['url'],semantic_similarity=float(similarity[i]),
       source_sha256=hashlib.sha256(row['text'].encode()).hexdigest(),meaning='topical relevance; efficacy unknown'))
  result.update(symptom=query,ranked_evidence=ranked,retrieval_model='preserved-frx-v2-int8',
    assembly_status='post-evaluation conservative baseline; no new whole-pipeline accuracy claim',
    case_origin=case.get('origin','caller-supplied-unverified'),
    automatic_interpretation_approved=False,event_candidates=[])
  for hypothesis in result['hypotheses']:
   result['graph']['nodes'].append(dict(hypothesis,type='investigable-hypothesis'))
   for support in hypothesis['supports']:result['graph']['edges'].append(dict(source=support,target=hypothesis['id'],relation='supports-investigation; not-causal-proof'))
  if self.events:result['event_candidates']=self.events.analyze(case)['event_candidates']
  else:result['limitations'].append('Experimental language heads failed independent evaluation; supplied structured attempts are preserved without inferring outcomes from prose.')
  return result

def main():
 parser=argparse.ArgumentParser();parser.add_argument('root',type=Path);parser.add_argument('old',type=Path);parser.add_argument('--experimental-events',action='store_true');parser.add_argument('--memory',type=Path);args=parser.parse_args()
 model=CaseReview(args.root,args.old,args.experimental_events,args.memory)
 for line in sys.stdin:
  try:
   if len(line)>300000:raise ValueError('request size bound')
   output=model.analyze(json.loads(line))
  except (ValueError,TypeError,KeyError) as error:output=dict(status='invalid-request',error=str(error),execution_authorized=False)
  print(json.dumps(output,ensure_ascii=False),flush=True)

if __name__=='__main__':main()
