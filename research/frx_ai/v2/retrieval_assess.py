"""Same-task frozen/trained retrieval comparison; final labels read after freeze."""
import argparse
import hashlib
import json
import pathlib
import numpy as np
import torch
from transformers import AutoModel,AutoTokenizer
from retrieval import QUERIES,measure
from retrieval_train import encode
from train import read


def main(root,final=False):
    torch.set_num_threads(4)
    tokenizer=AutoTokenizer.from_pretrained(root/'pretrained',local_files_only=True)
    model=AutoModel.from_pretrained(root/'pretrained',local_files_only=True)
    if final:
        frozen=json.loads((root/'frozen.json').read_text());name=frozen.get('retrieval_name')
        for filename,digest in frozen['code_sha256'].items():
            if hashlib.sha256((pathlib.Path(__file__).parent/filename).read_bytes()).hexdigest()!=digest:raise ValueError('frozen code changed')
        samples=read(root/'final-events.jsonl');alphas=[frozen['retrieval_alpha']]
    else:
        samples=read(root/'retrieval-dev.jsonl');alphas=[0.,.25,.5,.75,1.]
        names=list((root/'retriever-runs').glob('*.json'))
        name=max(names,key=lambda p:json.loads(p.read_text())['selection_score']).stem if names else None
        if name:
            result=json.loads((root/'retriever-runs'/(name+'.json')).read_text())
            baseline=result['frozen']['summary']['semantic']['ndcg5']
            if result['selection_score']<baseline:name=None
    baseline=measure(samples,encode(model,tokenizer,[r['text'] for r in samples]),encode(model,tokenizer,list(QUERIES.values())),.5)
    if name:
        path=root/'retriever-runs'/(name+'.pt')
        if final and hashlib.sha256(path.read_bytes()).hexdigest()!=frozen['retrieval_sha256']:raise ValueError('retrieval checkpoint changed')
        checkpoint=torch.load(path,map_location='cpu',weights_only=True);model.load_state_dict(checkpoint['state'])
    emb=encode(model,tokenizer,[r['text'] for r in samples]);qe=encode(model,tokenizer,list(QUERIES.values()))
    results=[measure(samples,emb,qe,a) for a in alphas]
    winner=max(results,key=lambda r:(r['summary']['hybrid']['ndcg5'],r['summary']['hybrid']['p3']))
    summary=dict(model_name=name or 'frozen-MiniLM',frozen_same_tokenization=baseline,
        results=results,selected_alpha=winner['alpha'],evaluation='held-out-new-games' if final else 'development')
    if final:
        # Paired bootstrap over games for semantic and hybrid gains.
        apps=sorted({x['app_id'] for x in winner['details']});rng=np.random.default_rng(919);diffs=[]
        before={(x['app_id'],x['topic']):x for x in baseline['details']}
        for _ in range(2000):
            picked=rng.choice(apps,len(apps),replace=True)
            rows=[x for app in picked for x in winner['details'] if x['app_id']==app]
            diffs.append([np.mean([x[method]['p3']-before[(x['app_id'],x['topic'])]['semantic']['p3'] for x in rows]) for method in ['semantic','hybrid']])
        summary['game_bootstrap_gain_vs_frozen_p3']=np.quantile(diffs,[.025,.975],axis=0).tolist()
        from retrieval_runtime import Retriever
        native=Retriever(root/'runtime-retrieval')
        if native.meta!=frozen['retrieval_runtime']:raise ValueError('frozen retrieval runtime changed')
        summary['native_runtime']=measure(samples,native.encode([r['text'] for r in samples]),native.encode(list(QUERIES.values())),alphas[0])
    dest=root/('retrieval-final-v2.json' if final else 'retrieval-selection.json')
    if final and dest.exists():raise ValueError('preserve first final retrieval evaluation')
    with dest.open('x' if final else 'w') as f:json.dump(summary,f,indent=2)
    print('retrieval',summary['model_name'],'alpha',winner['alpha'],winner['summary'])


if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('root',type=pathlib.Path);p.add_argument('--final',action='store_true')
    a=p.parse_args();main(a.root,a.final)
