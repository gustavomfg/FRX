"""Post-freeze resource measurement; does not select or modify the model."""
import argparse,json,sys,time
from pathlib import Path
import numpy as np
from prototype import Intelligence
from data import read,write

def run(root,old):
 start=time.perf_counter();engine=Intelligence(root,old);init=time.perf_counter()-start
 cases=read(root/'demos.jsonl');elapsed=[];events=[]
 for i in range(24):
  before=time.perf_counter();result=engine.analyze(cases[i%len(cases)]);elapsed.append(time.perf_counter()-before)
  events.append(sum(len(p['events']) for p in result['event_paragraphs']))
  assert result['execution_authorized'] is False and not result['confirmed_causes']
 memory={line.split(':')[0]:int(line.split(':',1)[1].strip().split()[0]) for line in Path('/proc/self/status').read_text().splitlines() if line.startswith(('VmRSS:','VmHWM:'))}
 files=[root/'runtime-events'/engine.events.meta['model_file'],old/'runtime-retrieval'/engine.search.meta['model_file']]
 summary=dict(requests=24,distinct_known_cases=len(cases),initialization_seconds=init,median_seconds=float(np.median(elapsed)),p95_seconds=float(np.quantile(elapsed,.95)),proc_memory_kib=memory,torch_loaded='torch' in sys.modules,model_bytes=sum(p.stat().st_size for p in files),scope='4 CPU threads; 6 known public-source demonstrations repeated; no network, no game metrics; post-freeze measurement only')
 write(root/'benchmark.json',summary);print(json.dumps(summary,indent=2))

if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('root',type=Path);p.add_argument('old',type=Path);a=p.parse_args();run(a.root,a.old)
