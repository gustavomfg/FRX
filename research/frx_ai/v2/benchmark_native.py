"""Measure a fresh local ONNX process, not a torch training process."""
import argparse
import json
import pathlib
import resource
import time
import numpy as np


def main(root,directory):
    cold=time.perf_counter()
    from runtime import Runtime
    model=Runtime(directory);startup=time.perf_counter()-cold
    rows=[json.loads(x) for x in (root/'dev-events.jsonl').read_text().splitlines()]
    samples=[rows[i] for i in [0,12,34]]
    summary={}
    for provided,n in [(True,30),(False,12)]:
        latencies=[]
        for i in range(n+2):
            row=samples[i%len(samples)];item={k:row[k] for k in ['app_id','url','text']}
            if provided:item['action_span']=row['action_span']
            request=dict(context={'app_id':row['app_id']},symptom='The game closes while loading.',evidence=[item])
            start=time.perf_counter();result=model.analyze(request)
            assert not result['execution_authorized']
            if i>=2:latencies.append((time.perf_counter()-start)*1000)
        summary['provided_action' if provided else 'automatic_proposals']=dict(n=n,median_ms=float(np.median(latencies)),p95_ms=float(np.quantile(latencies,.95)))
    summary.update(model=model.meta['model_name'],cold_seconds=startup,
        peak_rss_kib=resource.getrusage(resource.RUSAGE_SELF).ru_maxrss,
        model_bytes=(directory/model.meta['model_file']).stat().st_size,
        tokenizer_bytes=(directory/'tokenizer.json').stat().st_size,
        torch_loaded='torch' in __import__('sys').modules,
        caveat='Three real development comments repeated; no network, game or renderer measurement.')
    (root/('benchmark-'+directory.name+'.json')).write_text(json.dumps(summary,indent=2));print(json.dumps(summary))


if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('root',type=pathlib.Path);p.add_argument('model',type=pathlib.Path);a=p.parse_args();main(a.root,a.model)
