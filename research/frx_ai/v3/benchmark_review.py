"""Small CPU benchmark on known historical demonstrations, without networking."""
import argparse
from pathlib import Path
import json
import resource
import time
import numpy as np
from case_review import CaseReview
from common import write

def run(root,old):
 start=time.perf_counter();model=CaseReview(root,old);initialization=time.perf_counter()-start
 sources=json.loads((root/'historical-proposed.json').read_text());requests=[]
 for row in sources:
  # These are public reported logs; they are not the local machine's telemetry.
  requests.append(dict(context={'app_id':row['app_id']},symptom='The game fails with a graphics or library error.',log=row['text'],evidence=[{k:row[k] for k in ['app_id','url','text']}]))
 times=[]
 for i in range(24):
  before=time.perf_counter();result=model.analyze(requests[i%len(requests)]);times.append(time.perf_counter()-before)
  assert result['execution_authorized'] is False and not result['confirmed_causes']
 memory={line.split(':')[0]:int(line.split(':',1)[1].strip().split()[0]) for line in Path('/proc/self/status').read_text().splitlines() if line.startswith(('VmRSS:','VmHWM:'))}
 summary=dict(n=24,distinct_known_sources=len(sources),init_seconds=initialization,
  median_seconds=float(np.median(times)),p95_seconds=float(np.quantile(times,.95)),
  peak_rss_kib=resource.getrusage(resource.RUSAGE_SELF).ru_maxrss,
  proc_memory_kib=memory,
  torch_loaded='torch' in __import__('sys').modules,scope='CPU four threads, repeated known public demonstrations, no network; not production latency or game performance')
 write(root/'benchmark-review.json',summary);print(json.dumps(summary,indent=2))

if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('root',type=Path);p.add_argument('old',type=Path);a=p.parse_args();run(a.root,a.old)
