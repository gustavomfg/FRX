"""Small real-source diagnostic reference controls, not population RCA accuracy."""
import argparse
from pathlib import Path
import json,re
from data import read,write,digest
from diagnosis import diagnose,v3

def run(root,v3root):
 cases=[]
 for source in json.loads((v3root/'historical-proposed.json').read_text()):
  kind='verify-library-origin' if any(x['predicate']=='reported-library-not-found' for x in source['observations']) else 'graphics-environment'
  cases.append(dict(id='historical-'+str(len(cases)),context={'app_id':source['app_id']},symptom='A game fails with a loader or graphics error.',log=source['text'],expected=kind,url=source['url'],reference='agent-selected question grounded in this source; parser-selected historical demonstration'))
 old3=read(v3root/'final-candidates.jsonl')
 for source in old3:
  m=re.search(r'Unhandled exception:\s*unimplemented function[^\n]+',source['text'])
  if m:
   cases.append(dict(id='unimplemented-'+str(len(cases)),context={'app_id':source['app_id']},symptom='The game fails with an unimplemented function exception.',log=m[0],expected='verify-component-function-version',url=source['url'],reference='Real quoted exception; proposed next check is provisional agent interpretation.'))
 # Real reports without a recoverable execution log must remain unknown.
 for index in [19,47,83]:
  source=old3[index]
  cases.append(dict(id='insufficient-'+str(len(cases)),context={'app_id':source['app_id']},symptom=source['text'][:500],log='',expected='collect-relevant-log',url=source['url'],reference='Controlled symptom-only view of a real report; telemetry intentionally withheld, not a new independent real incident.'))
 summaries=[]
 for case in cases:
  a=v3.inspect(case);b=diagnose(case)
  def check(item):
   name=item['next_checks'][0]['id'] if item['next_checks'] else None
   return 'verify-library-origin' if name and name.startswith('origin:') else name
  summaries.append(dict(id=case['id'],url=case['url'],app_id=case['context']['app_id'],source_view_sha256=digest(case['log']),reference=case['reference'],expected=case['expected'],v3=check(a),v4=check(b),v3_match=check(a)==case['expected'],v4_match=check(b)==case['expected'],v4_false_confirmed_causes=len(b['confirmed_causes'])))
 write(root/'diagnostic-references.json',cases);write(root/'diagnostic-results.json',dict(n=len(cases),reference_quality='agent-provisional; selected historical demonstrations; not an independent representative diagnostic test',cases=summaries,v3_matches=sum(x['v3_match'] for x in summaries),v4_matches=sum(x['v4_match'] for x in summaries),false_confirmed_causes=sum(x['v4_false_confirmed_causes'] for x in summaries)))
 print('Diagnostic reference cases:',len(cases),'v3',sum(x['v3_match'] for x in summaries),'v4',sum(x['v4_match'] for x in summaries))

if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('root',type=Path);p.add_argument('v3',type=Path);a=p.parse_args();run(a.root,a.v3)
