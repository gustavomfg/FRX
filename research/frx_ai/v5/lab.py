"""Read-only local investigation: trained retrieval, events, groups and unknowns."""
import argparse,json,re,sys,importlib.util
from pathlib import Path
from common import prior4,digest
from syntax import Syntax
from compact import CompactSyntax
from events import extract
from relations import reconstruct
from linker import link
from incidents import Incidents,candidates,intake,incident_state

v4diagnosis=prior4('diagnosis')

def retriever(old):
 file=Path(__file__).resolve().parents[1]/'v2/retrieval_runtime.py';spec=importlib.util.spec_from_file_location('v5_preserved_v2',file);m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m);return m.Retriever(old/'runtime-retrieval')

def rebase(value,offset,source_hash):
 if isinstance(value,dict):
  if 'span' in value and 'text' in value:value['source_span']=[x+offset for x in value['span']];value['source_sha256']=source_hash
  for x in list(value.values()):rebase(x,offset,source_hash)
 elif isinstance(value,list):
  for x in value:rebase(x,offset,source_hash)
 return value

class Intelligence:
 def __init__(self,root,old,memory=None,check_seal=True):
  if check_seal:
   from seal import verify
   verify(root)
  self.root=root;self.selection=json.loads((root/'selection.json').read_text());self.parser=Syntax(root) if self.selection['parser']=='stanza' else CompactSyntax(root);self.search=retriever(old);self.history=Incidents(memory) if memory else None
  self.linker=json.loads((root/'linker-selection.json').read_text()) if self.selection.get('linker') else None
 def analyze(self,case):
  if not isinstance(case,dict) or not isinstance(case.get('symptom'),str) or not case['symptom'].strip() or len(case['symptom'])>1000:raise ValueError('bounded symptom required')
  clean,rejected=intake(case)
  if self.history:clean,extra,namespace=self.history.exchange(clean);rejected+=extra
  base=v4diagnosis.diagnose(dict(clean,candidate_actions=[]));sources=[clean['evidence'][r['index']] for r in base['sources']];ranked=[];reports=[]
  if sources:
   scores=(self.search.encode([clean['symptom']])@self.search.encode([x['text'] for x in sources]).T)[0];order=sorted(range(len(sources)),key=lambda i:-float(scores[i]))
   sources=[sources[i] for i in order];ranked=[dict(url=sources[j]['url'],score=float(scores[i]),meaning='topic relevance, not efficacy or safety') for j,i in enumerate(order)]
  for source in sources[:8]:
   text=source['text'];parts=[]
   # One bounded authored unit per paragraph, never interpreting a code block
   # as an executed procedure. Full source offsets remain available.
   start=0
   for match in re.finditer(r'\r?\n\s*\r?\n|\Z',text):
    end=match.start();a=start;b=end
    while a<b and text[a].isspace():a+=1
    while b>a and text[b-1].isspace():b-=1
    if 25<=b-a<=1100 and not text[a:b].startswith(('>','```','<img','![','# Compatibility','# System')):parts.append((a,b))
    start=match.end()
   parts.sort(key=lambda p:-len(re.findall(r'(?i)\b(?:tried|installed|disabled|enabled|changed|using|updated|removed)\b',text[slice(*p)])))
   for a,b in parts[:3]:
    unit=text[a:b];parsed=self.parser.parse(unit);events=extract(unit,parsed);result=reconstruct(unit,parsed,events)
    if self.linker:
     edges,unknown=link(unit,parsed,events,self.linker['model'],self.linker['threshold'],self.linker['margin']);result.update(observations=edges,unattributed_observations=unknown)
    result=rebase(result,a,digest(text));result.update(source=source['url'],app_id=source['app_id'],source_offset=a,source_sha256=digest(text),scope='source thread; individual game/build/hardware attribution may be incomplete',automatic_interpretation_approved=False,causal_effect_established=False,local_benchmark=False,execution_authorized=False);reports.append(result)
    if len(reports)>=16:break
   if len(reports)>=16:break
  base.update(status='experimental-frx-v5-investigation',case_origin=clean.get('origin','caller-unverified'),ranked_evidence=ranked,event_reports=reports,context_intake_rejections=rejected,attempt_state=incident_state(base['attempts']),candidate_actions=candidates(clean,base['attempts']),automatic_interpretation_approved=False,technology_detection='explicit attributed game facts only; no inference from GPU or DLL presence',local_measurement_verified=False)
  # The grammar and trained linker report narrative associations. They do not
  # turn those associations into confirmed root causes or repair commands.
  for report in reports:
   for obs in report['observations']:
    if obs['outcome'] in ['persistent','regression']:
     base['hypotheses'].append(dict(id='reported-attempt-'+str(len(base['hypotheses'])),status='source-reported-association',statement='A source associates this procedure with '+obs['outcome']+'.',source=report['source'],evidence=obs['evidence'],individual_cause='unknown',confirmed=False,current_computer_effect='unknown'))
  if self.history:
   base['related_incidents']=self.history.related(clean,self.search);base['incident_id']=self.history.store(clean,base)
  else:base['related_incidents']=[]
  base['limitations']+=['All language annotations are provisional, not independently human validated.','Event and result interpretation remain experimental.','A failed attempt does not prove its hypothesized cause false.','Community events do not enter local caller-attempt memory.']
  return base

def main():
 p=argparse.ArgumentParser();p.add_argument('root',type=Path);p.add_argument('old',type=Path);p.add_argument('--memory',type=Path);a=p.parse_args();engine=Intelligence(a.root,a.old,a.memory)
 for line in sys.stdin:
  try:
   if len(line)>300000:raise ValueError('request size bound')
   result=engine.analyze(json.loads(line))
  except (ValueError,KeyError,TypeError) as e:result=dict(status='invalid-request',error=str(e),execution_authorized=False)
  print(json.dumps(result,ensure_ascii=False),flush=True)

if __name__=='__main__':main()
