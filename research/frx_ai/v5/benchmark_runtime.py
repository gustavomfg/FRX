"""Full retrieval+syntax process, uncached controlled requests; no quality claim."""
import argparse,time,json,resource,statistics
from pathlib import Path
from common import read,write
from lab import Intelligence
from run import guarded
def run(root,old):
 if (root/'benchmark-runtime.json').exists():raise ValueError('preserve runtime benchmark')
 start=time.perf_counter();engine=Intelligence(root,old);init=time.perf_counter()-start;times=[];nonce=str(time.time_ns())
 for i,row in enumerate(read(root/'dev-known.jsonl')[:12]):
  case=dict(context=dict(app_id=row['app_id']),symptom='A game crashes during startup.',evidence=[dict(app_id=row['app_id'],url=row['url'],text='Benchmark '+nonce+' '+str(i)+'. '+row['text'])],origin='controlled-benchmark-of-known-public-text; not-local-telemetry')
  before=time.perf_counter();result=guarded(engine,case);times.append(time.perf_counter()-before)
  if result['execution_authorized']:raise ValueError('research must never execute')
 ordered=sorted(times[1:]);meta=json.loads((old/'runtime-retrieval/runtime.json').read_text());model=old/'runtime-retrieval'/meta['model_file']
 out=dict(cpu_only=True,requests=len(times),init_seconds=init,first_request_seconds=times[0],uncached_warm_median_ms=1000*statistics.median(times[1:]),uncached_warm_p95_ms=1000*ordered[min(len(ordered)-1,int(.95*len(ordered)))],max_rss_mib=resource.getrusage(resource.RUSAGE_SELF).ru_maxrss/1024,retrieval_model_bytes=model.stat().st_size,quality_evaluation=False,limitations=['Known development texts, controlled nonce to force parser cache misses.','Cold process does not imply cold OS page cache.','Does not include Python environment installation size.'])
 write(root/'benchmark-runtime.json',out);print(json.dumps(out,indent=2))
if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('root',type=Path);p.add_argument('old',type=Path);a=p.parse_args();run(a.root,a.old)
