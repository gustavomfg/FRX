"""Evidence retrieval evaluation. Topical relevance is not a verified solution.

The original known audit is development material. New final topics may only
be read after the model/retrieval freeze. No classifier labels rank evidence.
"""
import argparse
import json
import pathlib
import re
import importlib.util
import time
import numpy as np
from sklearn.feature_extraction.text import TfidfVectorizer
from sklearn.metrics.pairwise import cosine_similarity
def legacy_module(name):
    spec=importlib.util.spec_from_file_location('frx_ai_v1_'+name,pathlib.Path(__file__).resolve().parents[1]/(name+'.py'))
    module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module)
    return module


Encoder=legacy_module('encoder').Encoder
clean=legacy_module('core').clean

QUERIES={
    "startup":"The game closes when loading and never reaches the main menu.",
    "crash":"The application freezes or exits unexpectedly during gameplay.",
    "performance":"Uneven frame pacing, low speed and pauses while playing.",
    "graphics":"The picture has flashing surfaces, broken shapes or absent textures.",
    "controls":"Buttons, mouse movement or the gamepad do not respond correctly.",
    "dependency":"A necessary software library is missing or fails to install.",
    "audio":"Sound is absent, distorted or crackles during playback.",
    "network":"Online play cannot connect or multiplayer is unavailable.",
}


def ranks(scores,ids):
    return sorted(ids,key=lambda i:(-float(scores[i]),i))


def measure(samples,emb,qemb,alpha):
    texts=[clean(x['text']) for x in samples]
    vec=TfidfVectorizer(ngram_range=(1,2),sublinear_tf=True)
    x=vec.fit_transform(texts);q=vec.transform(list(QUERIES.values()))
    lexical=cosine_similarity(q,x);semantic=qemb@emb.T
    results=[]
    for app in sorted({s['app_id'] for s in samples}):
        ids=[i for i,s in enumerate(samples) if s['app_id']==app]
        for qi,topic in enumerate(QUERIES):
            relevant={i for i in ids if topic in samples[i]['topics']}
            if not relevant:continue
            lexorder=ranks(lexical[qi],ids);semorder=ranks(semantic[qi],ids)
            fusion={i:alpha/(60+semorder.index(i)+1)+(1-alpha)/(60+lexorder.index(i)+1) for i in ids}
            tokens=set(re.findall(r'\w+',list(QUERIES.values())[qi].lower()))
            overlap={i:len(tokens&set(re.findall(r'\w+',texts[i].lower()))) for i in ids}
            row=dict(app_id=app,topic=topic,relevant=len(relevant))
            for name,order in [('overlap',ranks(overlap,ids)),('tfidf',lexorder),('semantic',semorder),('hybrid',ranks(fusion,ids))]:
                top=order[:3];hits=[int(i in relevant) for i in order[:5]]
                dcg=sum(hit/np.log2(k+2) for k,hit in enumerate(hits));ideal=sum(1/np.log2(k+2) for k in range(min(5,len(relevant))))
                row[name]=dict(p3=sum(i in relevant for i in top)/len(top),ndcg5=float(dcg/ideal),
                    mrr=next((1/(k+1) for k,i in enumerate(order) if i in relevant),0),top_urls=[samples[i]['url'] for i in top])
            results.append(row)
    summary={name:{metric:float(np.mean([r[name][metric] for r in results])) for metric in ['p3','ndcg5','mrr']} for name in ['overlap','tfidf','semantic','hybrid']}
    return dict(n_comments=len(samples),n_answerable_queries=len(results),n_total_queries=len({s['app_id'] for s in samples})*len(QUERIES),alpha=alpha,summary=summary,details=results)


def samples_old(old):
    rows=[json.loads(x) for x in (old/'final.jsonl').read_text().splitlines()]
    topics=json.loads((old/'retrieval-topics.json').read_text());unique={}
    for row in rows:
        if row['url'] not in unique:unique[row['url']]=dict(row,topics=[])
        unique[row['url']]['topics']=sorted(set(unique[row['url']]['topics'])|set(topics[row['id']]))
    return list(unique.values())


def main(root,old,final=False):
    if final and not (root/'frozen.json').exists():raise ValueError('freeze required')
    if final:
        samples=[json.loads(x) for x in (root/'final-events.jsonl').read_text().splitlines()]
        alphas=[json.loads((root/'frozen.json').read_text())['retrieval_alpha']]
    else:
        samples=samples_old(old);alphas=[0.,.25,.5,.75,1.]
    encoder=Encoder(old/'encoder');start=time.perf_counter()
    emb=encoder.encode([clean(s['text']) for s in samples]);qemb=encoder.encode(list(QUERIES.values()))
    results=[measure(samples,emb,qemb,a) for a in alphas]
    out=root/('retrieval-final.json' if final else 'retrieval-dev.json')
    if final and out.exists():raise ValueError('preserve first final result')
    out.write_text(json.dumps(dict(status='new-final' if final else 'known-v1-audit-development',seconds=time.perf_counter()-start,results=results),indent=2))
    for r in results:print(r['alpha'],r['summary'],flush=True)


if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('root',type=pathlib.Path);p.add_argument('old',type=pathlib.Path);p.add_argument('--final',action='store_true')
    a=p.parse_args();main(a.root,a.old,a.final)
