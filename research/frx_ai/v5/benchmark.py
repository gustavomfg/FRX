"""Fresh-process CPU benchmark; distinguish neural cache misses and cache hits."""
import argparse,json,time,resource,statistics,subprocess,sys
from pathlib import Path
from common import read,write
def child(root,kind):
 from syntax import Syntax
 from compact import CompactSyntax
 from events import extract
 from relations import reconstruct
 start=time.perf_counter();parser=Syntax(root) if kind=='stanza' else CompactSyntax(root);init=time.perf_counter()-start
 # Unique benchmark prefix forces a parser cache miss. These are known
 # development texts, not final data and not additional quality evaluation.
 texts=[r['text'] for r in read(root/'dev-known.jsonl')[:16]];times=[];cold=None;hit=[];tag=str(time.time_ns())
 for i,text in enumerate(texts):
  t='Benchmark '+tag+' '+str(i)+'. '+text;before=time.perf_counter();p=parser.parse(t);reconstruct(t,p,extract(t,p));elapsed=time.perf_counter()-before
  if cold is None:cold=elapsed
  else:times.append(elapsed)
  before=time.perf_counter();p=parser.parse(t);reconstruct(t,p,extract(t,p));hit.append(time.perf_counter()-before)
 def stats(x):
  x=sorted(x);return dict(median_ms=1000*statistics.median(x),p95_ms=1000*x[min(len(x)-1,int(.95*len(x)))])
 return dict(kind=kind,artifact_validation_seconds=init,cold_first_request_seconds=cold,uncached_warm=stats(times),cached=stats(hit),requests=len(texts),max_rss_mib=resource.getrusage(resource.RUSAGE_SELF).ru_maxrss/1024,quality_evaluation=False)
def run(root):
 if (root/'benchmark.json').exists():raise ValueError('preserve benchmark')
 result=[]
 for kind in ['stanza','compact']:
  raw=subprocess.check_output([sys.executable,__file__,str(root),'--child',kind],timeout=180);item=json.loads(raw);result.append(item);write(root/('benchmark-'+kind+'.json'),item)
 weights=sum(x['bytes'] for x in json.loads((root/'syntax-manifest.json').read_text())['files'])
 compact=json.loads((root/'compact-manifest.json').read_text());directory=Path(compact['directory']);cbytes=sum((directory/n).stat().st_size for n in compact['files'])
 write(root/'benchmark.json',dict(cpu_only=True,processes=result,stanza_model_bytes=weights,compact_installed_model_bytes=cbytes,limitations=['Warm process is not cold OS page-cache benchmark.','Python/Torch packages are not part of reported model bytes.','Disk parse cache timings are reported separately from inference.','No Rust integration or installer packaging benchmark.']))
 print(json.dumps(json.loads((root/'benchmark.json').read_text()),indent=2))
if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('root',type=Path);p.add_argument('--child',choices=['stanza','compact']);a=p.parse_args();print(json.dumps(child(a.root,a.child))) if a.child else run(a.root)
