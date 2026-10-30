"""Warm local inference timing, no network or game process."""
import json,pathlib,statistics,sys,time,resource
from prototype import ResearchAI
root=pathlib.Path(sys.argv[1]);request=json.loads((root/'demo-input.jsonl').read_text());start=time.perf_counter();ai=ResearchAI(root);startup=time.perf_counter()-start;ai.analyze(request);times=[]
for _ in range(20):
 start=time.perf_counter();ai.analyze(request);times.append((time.perf_counter()-start)*1000)
result={'startup_seconds':startup,'warm_samples':20,'p50_ms':statistics.median(times),'p95_ms':sorted(times)[18],'peakRSS_KiB':resource.getrusage(resource.RUSAGE_SELF).ru_maxrss,'threads':4,'device':'CPU','models_bytes':sum(p.stat().st_size for sub in ['encoder','nli'] for p in (root/sub).glob('*') if p.is_file())}
(root/'experiments/benchmark.json').write_text(json.dumps(result,indent=2));print(result)
