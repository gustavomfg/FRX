"""Train a compact dual encoder on provisional TRAIN topic annotations.

Positive query/document edges are topical, never assertions that a fix works.
Query wording differs between training and development/final evaluation.
"""
import os
os.environ.setdefault('OMP_NUM_THREADS','4')
os.environ.setdefault('OPENBLAS_NUM_THREADS','4')
import argparse
import json
import pathlib
import time
import numpy as np
import torch
from transformers import AutoModel,AutoTokenizer
from retrieval import QUERIES,measure
from train import read
from budget import spent

TRAIN_QUERIES={
    'startup':'The game does not launch or fails to start.',
    'crash':'Crashing and freezing during the game.',
    'performance':'Low FPS and stuttering in the game.',
    'graphics':'Visual artifacts or missing geometry and textures.',
    'controls':'Mouse keyboard or controller input issues.',
    'dependency':'Missing libraries and failed component installation.',
    'audio':'Crackling or missing audio and sound.',
    'network':'Problems with connecting to multiplayer online.',
}


def pool(model,tokens):
    h=model(**tokens).last_hidden_state;m=tokens['attention_mask'].float()
    z=(h*m[...,None]).sum(1)/m.sum(1).clamp(min=1)[:,None]
    return torch.nn.functional.normalize(z,dim=1)


def encode(model,tokenizer,texts):
    model.eval();out=[]
    with torch.inference_mode():
        for start in range(0,len(texts),16):
            tokens=tokenizer(texts[start:start+16],padding=True,truncation=True,max_length=192,return_tensors='pt')
            out.append(pool(model,tokens).numpy())
    return np.concatenate(out)


def run(root,name,lr):
    if (root/'frozen.json').exists():raise ValueError('research sealed')
    out=root/'retriever-runs';out.mkdir(exist_ok=True)
    if (out/(name+'.json')).exists():raise ValueError('preserve completed run')
    start=time.perf_counter();torch.set_num_threads(4);torch.set_num_interop_threads(1);torch.manual_seed(17)
    train=[x for x in read(root/'retrieval-train.jsonl') if x['topics']]
    dev=read(root/'retrieval-dev.jsonl')
    assert not {r['app_id'] for r in train}&{r['app_id'] for r in dev}
    tokenizer=AutoTokenizer.from_pretrained(root/'pretrained',local_files_only=True)
    model=AutoModel.from_pretrained(root/'pretrained',local_files_only=True)
    frozen=measure(dev,encode(model,tokenizer,[r['text'] for r in dev]),encode(model,tokenizer,list(QUERIES.values())),.5)
    for n,p in model.named_parameters():p.requires_grad=any(f'layer.{i}.' in n for i in [4,5])
    optimizer=torch.optim.AdamW([p for p in model.parameters() if p.requires_grad],lr=lr,weight_decay=.01)
    qtok=tokenizer(list(TRAIN_QUERIES.values()),padding=True,truncation=True,max_length=192,return_tensors='pt')
    docs=tokenizer([r['text'] for r in train],padding='max_length',truncation=True,max_length=192,return_tensors='pt')
    positive=torch.tensor([[topic in r['topics'] for r in train] for topic in TRAIN_QUERIES],dtype=torch.float32)
    history=[];best=None;bestscore=-1
    for epoch in range(4):
        used=spent(root)
        if used+time.perf_counter()-start>5300:
            if best is None:raise RuntimeError('not enough compute budget for retrieval training')
            break
        model.train();losses=[]
        for idx in torch.randperm(len(train)).split(16):
            optimizer.zero_grad(set_to_none=True)
            q=pool(model,qtok);d=pool(model,{k:v[idx] for k,v in docs.items()})
            scores=q@d.T/.08;p=positive[:,idx];valid=p.sum(1)>0
            qloss=-(torch.log_softmax(scores,dim=1)*p).sum(1)[valid]/p.sum(1)[valid]
            dloss=-(torch.log_softmax(scores.T,dim=1)*p.T).sum(1)/p.T.sum(1).clamp(min=1)
            loss=(qloss.mean()+dloss.mean())/2;loss.backward();torch.nn.utils.clip_grad_norm_(model.parameters(),1.)
            optimizer.step();losses.append(float(loss.detach()))
        emb=encode(model,tokenizer,[r['text'] for r in dev]);qe=encode(model,tokenizer,list(QUERIES.values()))
        result=measure(dev,emb,qe,.5);score=result['summary']['semantic']['ndcg5']
        history.append(dict(epoch=epoch+1,loss=float(np.mean(losses)),dev=result))
        print(name,epoch+1,result['summary']['semantic'],flush=True)
        if score>bestscore:bestscore=score;best={k:v.detach().clone() for k,v in model.state_dict().items()}
    torch.save(dict(state=best,lr=lr),out/(name+'.pt'))
    (out/(name+'.json')).write_text(json.dumps(dict(name=name,seconds=time.perf_counter()-start,
        frozen=frozen,epochs=history,selection_score=bestscore,real_comments=len(train)),indent=2))


if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('root',type=pathlib.Path);p.add_argument('name');p.add_argument('--lr',type=float,required=True)
    a=p.parse_args();run(a.root,a.name,a.lr)
