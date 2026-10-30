"""JSONL process interface, suitable for a future Rust child-process adapter."""
import argparse
import json
import pathlib
import sys
from runtime import Runtime


def main():
    p=argparse.ArgumentParser();p.add_argument('--model',type=pathlib.Path,required=True)
    p.add_argument('--retrieval',type=pathlib.Path)
    args=p.parse_args();model=Runtime(args.model);retriever=None
    if args.retrieval:
        from retrieval_runtime import Retriever
        retriever=Retriever(args.retrieval)
    for line in sys.stdin:
        try:
            if len(line)>600_000:raise ValueError('request size limit')
            result=model.analyze(json.loads(line))
            if retriever and result['candidates']:
                request=json.loads(line);items={x['url']:x for x in request.get('evidence',[]) if isinstance(x,dict) and x.get('url') in {c['source'] for c in result['candidates']}}
                keys=list(items);vectors=retriever.encode([str(request.get('symptom',''))[:500]]+[items[k]['text'] for k in keys])
                scores=vectors[1:]@vectors[0]
                result['retrieved_sources']=[dict(url=keys[i],semantic_similarity=float(scores[i]),causality_confirmed=False) for i in sorted(range(len(keys)),key=lambda i:-scores[i])]
        except (ValueError,TypeError,KeyError) as e:
            result=dict(status='invalid-research-request',error=str(e),execution_authorized=False)
        print(json.dumps(result,ensure_ascii=False,allow_nan=False),flush=True)


if __name__=='__main__':main()
