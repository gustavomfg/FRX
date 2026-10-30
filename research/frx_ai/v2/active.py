"""Simulated pool-based active learning on provisional TRAIN annotations only.

Pool labels are revealed only after selection. Development measures benefit;
this is not an independent active-learning benchmark or a human annotation.
"""
import argparse
import hashlib
import json
import pathlib
import time
import numpy as np
from scipy.special import softmax
from sklearn.feature_extraction.text import TfidfVectorizer
from sklearn.metrics.pairwise import cosine_similarity
from baselines import fit, features
from train import read
from metrics import risk_curve


def main(root):
    train=read(root/"train-events.jsonl");dev=read(root/"dev-events.jsonl")
    dest=root/"active";dest.mkdir(exist_ok=True)
    strategies={"random":None,"uncertainty":(1,0,0),"diversity":(0,1,0),
                "uncertain-diverse":(.5,.5,0),"rare-balanced":(.4,.4,.2)}
    for seed in [17,29]:
        order=sorted(range(len(train)),key=lambda i:hashlib.sha256((str(seed)+train[i]["id"]).encode()).hexdigest())
        initial=order[:45];pool=order[45:]
        base=[train[i] for i in initial]
        pool_logits,_=fit(base,[train[i] for i in pool],"local",10.,0)
        p=softmax(pool_logits[0],axis=1)
        uncertainty=-(p*np.log(p.clip(1e-9))).sum(1)/np.log(5)
        rare=p[:,3]+p[:,4]
        # Vocabulary learned without development/final examples.
        vec=TfidfVectorizer(ngram_range=(1,2),sublinear_tf=True).fit(features(train,"local"))
        x=vec.transform(features(train,"local"));similarity=cosine_similarity(x,x)
        for strategy,weights in strategies.items():
            start=time.perf_counter();chosen=[];available=list(range(len(pool)))
            rng=np.random.default_rng(seed)
            for _ in range(min(25,len(pool))):
                if weights is None:j=int(rng.choice(available))
                else:
                    diversity=1-similarity[np.array(pool)[:,None],np.array(initial+[pool[j] for j in chosen])[None,:]].max(1)
                    score=weights[0]*uncertainty+weights[1]*diversity+weights[2]*rare
                    j=max(available,key=lambda k:score[k])
                available.remove(j);chosen.append(j)
            ids=initial+[pool[j] for j in chosen]
            logits,_=fit([train[i] for i in ids],dev,"local",10.,0)
            name=f"A-{seed}-{strategy}"
            result=dict(name=name,seconds=time.perf_counter()-start,strategy=strategy,weights=weights,
                        initial_n=len(initial),acquired_n=len(chosen),selected_ids=[train[i]["id"] for i in ids],
                        curve=risk_curve(dev,logits))
            (dest/(name+".json")).write_text(json.dumps(result,indent=2))
            m=result["curve"]["0.0"];print(name,round(m["relation_macro_f1"],3),round(m["exact_all_accuracy"],3),flush=True)
    # Compact queue for an actual independent reviewer, not fabricated review.
    top=np.argsort(-uncertainty)[:15]
    queue=[dict(id=train[pool[int(i)]]["id"],url=train[pool[int(i)]]["url"],
                app_id=train[pool[int(i)]]["app_id"],action_span=train[pool[int(i)]]["action_span"],
                reason="uncertain provisional relation; independent blind review requested",
                annotation_status="independent human review pending") for i in top]
    (dest/"human-review-queue.json").write_text(json.dumps(queue,indent=2))


if __name__=="__main__":
    p=argparse.ArgumentParser();p.add_argument("root",type=pathlib.Path);main(p.parse_args().root)
