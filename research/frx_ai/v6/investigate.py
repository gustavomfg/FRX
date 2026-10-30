"""JSONL local research investigator. No process execution or repair interface."""
import argparse,json,re,sys,copy
from pathlib import Path
from bridge import read,digest,retriever,CompactSyntax,Incidents,action_key,context_match,prior4
from classifier import Classifier
from engine import Interpreter
from diagnostic import inspect

def parts(text):
 for m in re.finditer(r'[^\n].*?(?=\r?\n\s*\r?\n|\Z)',text,re.S):
  a,b=m.span()
  while a<b and text[a].isspace():a+=1
  while b>a and text[b-1].isspace():b-=1
  if 25<=b-a<=1300:yield a,b
def rebase(value,offset,source_hash):
 if isinstance(value,dict):
  if 'span' in value and 'text' in value:value['document_span']=[x+offset for x in value['span']];value['document_text_sha256']=source_hash
  for v in list(value.values()):rebase(v,offset,source_hash)
 elif isinstance(value,list):
  for v in value:rebase(v,offset,source_hash)
 return value
def candidate_status(case,history):
 result=[]
 for item in case.get('candidate_actions',[]):
  if not isinstance(item,dict) or not isinstance(item.get('action'),dict):raise ValueError('typed candidate required')
  relation=context_match(case['context'],item.get('context',{}));requirements=item.get('requirements',{});missing=[];conflicts=[]
  if not isinstance(requirements,dict) or len(requirements)>16:raise ValueError('bounded typed requirements required')
  for key,value in requirements.items():
   actual=case['context'].get(key)
   if actual is None:missing.append(key)
   elif actual!=value:conflicts.append(key)
  failed=any(action_key(h['action'])==action_key(item['action']) and h['outcome'] in ['not-resolved','worsened'] for h in history)
  source_ok=prior4('diagnosis').v3.source_url(item.get('url'))
  status='source-unverified' if not source_ok else 'context-incompatible' if conflicts or relation in ['different-game','incompatible'] else 'requirements-unknown' if missing or relation=='unknown' else 'previously-failed-same-exact-context' if failed else 'investigate-only'
  result.append(dict(action=item['action'],source=item.get('url'),status=status,missing_requirements=missing,conflicting_requirements=conflicts,efficacy_confirmed=False,execution_authorized=False))
 return result
class Investigator:
 def __init__(self,root,old,memory=None,verify=True):
  if verify:
   from seal import verify as verify_seal
   verify_seal(root,old)
  self.root=root;self.search=retriever(old);self.catalog=json.loads((root/'catalog.json').read_text());self.selection=json.loads((root/'selection.json').read_text());classifier=Classifier(root/self.selection['model'])
  self.interpreter=Interpreter(CompactSyntax(root),self.catalog,classifier,self.selection['threshold'],mode='trained-structured-verifier');self.memory=memory
 def analyze(self,case):
  if not isinstance(case,dict) or not isinstance(case.get('context'),dict):raise ValueError('structured context required')
  if not re.fullmatch(r'[1-9]\d{1,9}',str(case['context'].get('app_id',''))):raise ValueError('explicit investigation AppID required; source events can remain unknown')
  if not isinstance(case.get('symptom'),str) or not 1<=len(case['symptom'])<=1000:raise ValueError('bounded symptom required')
  for key in ['evidence','attempts','candidate_actions','facts']:
   if not isinstance(case.get(key,[]),list) or len(case.get(key,[]))>64:raise ValueError('bounded structured collection required')
  if len(json.dumps(case['context']))>10000:raise ValueError('context bound')
  clean=copy.deepcopy(case);history=[];rejected=[]
  for i,a in enumerate(clean.get('attempts',[])):
   if not isinstance(a,dict) or not isinstance(a.get('context'),dict):raise ValueError('attempt context required')
   if a['context']==clean['context'] and context_match(clean['context'],a['context'])=='compatible':history.append(a)
   else:rejected.append(dict(attempt=i,reason='different-or-incomplete-environment'))
  clean['attempts']=history
  mem=None
  if self.memory:
   # Namespace includes every supplied environment field, including vendor,
   # component hashes, unknown values and symptom. No legacy migration guess.
   proposals=clean.get('candidate_actions',[]);namespace=digest(json.dumps([clean['context'],clean['symptom']],sort_keys=True));mem=Incidents(Path(self.memory)/namespace);clean,extra,_=mem.exchange(dict(clean,candidate_actions=[]));clean['candidate_actions']=proposals;rejected+=extra
  base=inspect(dict(clean,candidate_actions=[]));base['candidate_actions']=candidate_status(clean,base['attempts']);base['context_rejections']=rejected;sources=[];seen=set()
  for i,s in enumerate(clean.get('evidence',[])):
   if not isinstance(s,dict) or not isinstance(s.get('text'),str) or not 1<=len(s['text'])<=16000:raise ValueError('bounded evidence required')
   if not prior4('diagnosis').v3.source_url(s.get('url')):rejected.append(dict(source=i,reason='unapproved-url'));continue
   if str(s.get('origin','')).startswith('synthetic'):rejected.append(dict(source=i,reason='synthetic-not-real-evidence'));continue
   key=(s['url'].split('#')[0],digest(re.sub(r'(?m)^\s*> ?', '',s['text']).strip()))
   if key in seen:continue
   seen.add(key);environment=dict(s.get('context',{}),app_id=s.get('app_id'));match=context_match(clean['context'],environment);sources.append(dict(s,applicability=match,independence_group=s['url'].split('#')[0]))
  if sources:
   scores=(self.search.encode([clean['symptom']])@self.search.encode([s['text'] for s in sources]).T)[0];sources=[dict(s,topic_score=float(score)) for s,score in sorted(zip(sources,scores),key=lambda x:-float(x[1]))]
  reports=[]
  for s in sources[:8]:
   for a,b in list(parts(s['text']))[:3]:
    row=dict(text=s['text'][a:b],app_id=s.get('app_id'),source_thread_app_id=s.get('app_id'),url=s['url'],document_id=s.get('document_id',digest(s['text'])),reporter_id=s.get('reporter_id'),source_sha256=digest(s['text']));result=self.interpreter.analyze(row);result.pop('proposals',None);result=rebase(result,a,digest(s['text']));result.update(source=s['url'],source_thread_app_id=s.get('app_id'),source_environment_applicability=s['applicability'],current_case_effect='unknown',source_content_authenticated=False);reports.append(result)
  base.update(status='experimental-frx-v6-investigation',case_origin=case.get('origin','caller-unverified'),ranked_evidence=[dict(url=s['url'],topic_score=s['topic_score'],applicability=s['applicability'],use='analogy-only; no transferred outcome' if s['applicability'] in ['incompatible','different-game'] else 'research evidence; not confirmed local result',independence_group=s['independence_group']) for s in sources],event_reports=reports,automatic_approval=False,execution_authorized=False,local_runtime_verified=False,diagnostic_model='explicit evidence checks, not a trained root-cause model')
  base['limitations']+=['Source thread AppID is never used as event identity.','Trained association scores are uncalibrated and labels provisional.','Compatible context is not evidence a procedure works locally.','No community prediction enters caller-attempt memory.']
  if mem:base['incident_id']=mem.store(clean,base)
  return base
def main():
 p=argparse.ArgumentParser();p.add_argument('root',type=Path);p.add_argument('old',type=Path);p.add_argument('--memory',type=Path);a=p.parse_args();engine=Investigator(a.root,a.old,a.memory)
 for line in sys.stdin:
  try:
   if len(line)>300000:raise ValueError('request bound')
   out=engine.analyze(json.loads(line))
  except (ValueError,TypeError,KeyError) as e:out=dict(status='invalid-request',error=str(e),execution_authorized=False)
  print(json.dumps(out,ensure_ascii=False),flush=True)
if __name__=='__main__':main()
