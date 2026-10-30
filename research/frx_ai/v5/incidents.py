"""Structured research incidents, contextual attempt memory and milestone replay.

No system tools or repairs exist here. Stored effects are source/caller reports,
never verified local benchmarks or transferable causal facts.
"""
from pathlib import Path
import json,os,tempfile
from common import digest,prior4

Memory=prior4('context_memory').ContextMemory
intake=prior4('run').intake
FIELDS=['app_id','game_version','gpu','driver','proton','technology','configuration']

def context_match(a,b):
 if a.get('app_id')!=b.get('app_id'):return 'different-game'
 for k in FIELDS[1:]:
  if a.get(k) is not None and b.get(k) is not None and a[k]!=b[k]:return 'incompatible'
 if any(a.get(k) is None or b.get(k) is None for k in ['gpu','driver','proton']):return 'unknown'
 if any(a.get(k)!=b.get(k) for k in FIELDS):return 'unknown'
 return 'compatible'

def action_key(value):return digest(json.dumps(value,sort_keys=True,ensure_ascii=False))
def incident_state(attempts):
 outcomes={x.get('outcome','unknown') for x in attempts}
 if 'positive' in outcomes and outcomes&{'not-resolved','worsened'}:return 'conflicting-or-sequential-results'
 if 'mixed' in outcomes:return 'partial-reported-improvement'
 if 'worsened' in outcomes:return 'reported-worsening'
 if 'positive' in outcomes:return 'reported-resolved; not independently verified'
 if 'not-resolved' in outcomes:return 'reported-persistent'
 return 'unknown'

class Incidents:
 def __init__(self,directory):
  self.directory=Path(directory);self.directory.mkdir(parents=True,exist_ok=True);self.directory.chmod(0o700);self.memory=Memory(self.directory/'attempts')
 def exchange(self,case):
  clean,rejected=intake(case);clean,namespace=self.memory.exchange(clean);return clean,rejected,namespace
 def store(self,case,analysis):
  record=dict(schema='frx-incident-1',context=case['context'],symptom=case['symptom'],origin=case.get('origin','caller-unverified'),attempts=analysis['attempts'],state=incident_state(analysis['attempts']),observations=analysis['observations'],source_groups=analysis.get('evidence_thread_groups',[]),checks=[x['id'] for x in analysis['next_checks']],local_measurement_verified=False,causality_confirmed=False)
  # Identity includes scope and source hashes; community duplicates are not votes.
  identity=digest(json.dumps([record['context'],record['symptom'],record['source_groups']],sort_keys=True,ensure_ascii=False));file=self.directory/(identity+'.json')
  with tempfile.NamedTemporaryFile('w',dir=self.directory,delete=False) as f:
   json.dump(record,f,ensure_ascii=False);name=f.name
  os.chmod(name,0o600);os.replace(name,file);return identity
 def related(self,case,encoder,limit=4):
  records=[]
  for file in sorted(self.directory.glob('*.json'))[:200]:
   try:r=json.loads(file.read_text())
   except (ValueError,OSError):continue
   if r.get('schema')!='frx-incident-1':continue
   match=context_match(case['context'],r['context']);records.append(dict(id=file.stem,scope=match,record=r))
  if not records:return []
  q=encoder.encode([case['symptom']]);texts=[r['record']['symptom']+' '+r['record']['state']+' '+' '.join(o.get('predicate','') for o in r['record']['observations']) for r in records];scores=(q@encoder.encode(texts).T)[0]
  order=sorted(range(len(records)),key=lambda i:-float(scores[i]));out=[]
  for i in order[:limit]:
   r=records[i];out.append(dict(id=r['id'],context_scope=r['scope'],relevance_score=float(scores[i]),source_state=r['record']['state'],may_reuse_attempt_history=r['scope']=='compatible',use='contextual-research-case' if r['scope']=='compatible' else 'analogy-only; do not transfer outcomes',current_case_result_confirmed=False))
  return out

def candidates(case,history):
 out=[]
 for value in case.get('candidate_actions',[]):
  known=context_match(case['context'],value.get('context',{}));key=action_key(value.get('action',{}));failed=[a for a in history if action_key(a['action'])==key and a['outcome'] in ['not-resolved','worsened']]
  verified_url=prior4('diagnosis').v3.source_url(value.get('url',''))
  if not verified_url:known='source-unverified'
  out.append(dict(action=value.get('action'),source=value.get('url'),status='source-unverified' if not verified_url else 'previously-failed-same-exact-context' if failed and known=='compatible' else 'context-incompatible' if known in ['different-game','incompatible'] else 'context-unknown' if known=='unknown' else 'investigate-only',execution_authorized=False,efficacy_confirmed=False))
 return out

class Replay:
 """Read only immutable, explicitly supplied incident tool-cache entries."""
 def __init__(self,snapshot):
  self.snapshot=snapshot;self.state=dict(snapshot['input']);self.visited=[]
 def reveal(self,check):
  if check in self.visited:return False
  item=self.snapshot.get('tool_cache',{}).get(check)
  if not item:return False
  self.visited.append(check)
  if item['kind']=='log':self.state['log']=item['text']
  elif item['kind']=='fact':self.state['facts']=self.state.get('facts',[])+item['values']
  elif item['kind']=='context':self.state['context']=dict(self.state['context'],**item['values'])
  else:raise ValueError('no executable replay tools')
  self.state['checked_items']=self.visited.copy();return True
