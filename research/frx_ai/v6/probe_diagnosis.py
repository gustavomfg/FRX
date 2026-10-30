"""Post-freeze diagnostic probes; known-source milestones, not learned RCA."""
import argparse,json
from pathlib import Path
from bridge import read,write,sha,prior4
from diagnostic import inspect

def run(root):
 rows=read(root/'acceptance-gold.jsonl');cases=[]
 # Declare milestones before either baseline or v6 is run. None of these
 # outputs is used to change inference, labels, thresholds or model selection.
 for i,expected in [(57,'verify-component-function-version'),(66,'collect-relevant-log'),(92,'collect-relevant-log')]:
  row=rows[i];case=dict(context=dict(app_id=row['app_id']),symptom='Investigate the reported startup crash.',evidence=[],origin='known-public-acceptance-source diagnostic probe; no local telemetry')
  if i==57:case['log']='\n'.join(line.lstrip('> ') for line in row['text'].splitlines() if 'Unhandled exception:' in line)
  cases.append(dict(id=row['id'],url=row['url'],source_sha256=row['source_sha256'],case=case,expected=expected))
 write(root/'diagnostic-probe-reservation.json',cases)
 results=[]
 for entry in cases:
  variants={}
  for name,fn in [('v5',prior4('diagnosis').diagnose),('v6',inspect)]:
   out=fn(entry['case']);variants[name]=dict(next_checks=out['next_checks'],top1=bool(out['next_checks']) and out['next_checks'][0]['id']==entry['expected'],confirmed_causes=out['confirmed_causes'],cause_probability=out.get('cause_probability'),observations=out['observations'])
  results.append(dict(id=entry['id'],url=entry['url'],expected=entry['expected'],variants=variants))
 write(root/'diagnostic-probes.json',dict(status='post-freeze known-source probes; provisional agent milestones, no training or independent RCA validation',records=results,summary={name:sum(r['variants'][name]['top1'] for r in results) for name in ['v5','v6']},probes=len(results),learned_diagnostic_model=False,execution_authorized=False))
 print({name:sum(r['variants'][name]['top1'] for r in results) for name in ['v5','v6']})
if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('root',type=Path);run(p.parse_args().root)
