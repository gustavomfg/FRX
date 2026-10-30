"""Controlled lightweight baselines on the same span-annotated relation task."""
import os
os.environ.setdefault("OPENBLAS_NUM_THREADS", "1")
import argparse
import hashlib
import json
import pathlib
import time
import joblib
import numpy as np
from sklearn.feature_extraction.text import TfidfVectorizer
from sklearn.pipeline import FeatureUnion
from sklearn.linear_model import LogisticRegression
from model import labels, representation
from train import read
from metrics import evaluate, risk_curve
from synthetic import generate


def features(rows, mode):
    output=[]
    for r in rows:
        a,c=representation(r,"marked" if mode!="whole" else "whole")
        if mode=="action":
            c=a
        elif mode=="local":
            start,end=r["action_span"]
            c=r["text"][max(0,start-120):min(len(r["text"]),end+450)]
        output.append(f"action {a} context {c}")
    return output


def fit(train, dev, mode, c=1., synthetic=0):
    training=train+generate(synthetic,31)
    vec=FeatureUnion([("word",TfidfVectorizer(ngram_range=(1,3),min_df=1,sublinear_tf=True)),
                      ("char",TfidfVectorizer(analyzer="char_wb",ngram_range=(3,5),min_df=2,sublinear_tf=True))])
    x=vec.fit_transform(features(training,mode)); v=vec.transform(features(dev,mode));y=labels(training).numpy()
    models=[];logits=[]
    for j,n in enumerate([5,10,3,2]):
        model=LogisticRegression(C=c,max_iter=500,class_weight="balanced",random_state=17).fit(x,y[:,j])
        # decision_function binary gives one logit; embed into full class space.
        raw=model.decision_function(v)
        if raw.ndim==1:raw=np.stack([-raw/2,raw/2],1)
        out=np.full((len(dev),n),-50.)
        out[:,model.classes_]=raw
        logits.append(out);models.append(model)
    return logits,(vec,models,mode)


def main(root):
    out=root/"baselines";out.mkdir(exist_ok=True)
    train=read(root/"train-events.jsonl");dev=read(root/"dev-events.jsonl")
    for mode in ["whole","local","action"]:
        for synthetic in [0,320]:
            for c in [1.,10.]:
                name=f"B-{mode}-s{synthetic}-C{c:g}"
                if (out/(name+".json")).exists():continue
                start=time.perf_counter();logits,model=fit(train,dev,mode,c,synthetic)
                joblib.dump(model,out/(name+".joblib"))
                np.savez(out/(name+".npz"),**{f"head{i}":p for i,p in enumerate(logits)})
                result=dict(name=name,seconds=time.perf_counter()-start,curve=risk_curve(dev,logits),
                            train_hash=hashlib.sha256((root/"train-events.jsonl").read_bytes()).hexdigest(),
                            dev_hash=hashlib.sha256((root/"dev-events.jsonl").read_bytes()).hexdigest())
                (out/(name+".json")).write_text(json.dumps(result,indent=2))
                m=result["curve"]["0.0"];print(name,round(m["relation_macro_f1"],3),round(m["exact_all_accuracy"],3),flush=True)


if __name__=="__main__":
    p=argparse.ArgumentParser();p.add_argument("root",type=pathlib.Path);main(p.parse_args().root)
