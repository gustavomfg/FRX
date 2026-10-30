"""Second-stage learned edge classification with deterministic proposals."""
import os
os.environ.setdefault("OPENBLAS_NUM_THREADS","4")
os.environ.setdefault("OMP_NUM_THREADS","4")
import argparse
import hashlib
import json
import pathlib
import time
import numpy as np
import torch
from torch import nn
from transformers import AutoTokenizer
from model import RelationModel,tokenize,labels,predict,supervised_contrast
from train import read
from synthetic import generate,hard_generate
from pairs import training_pairs,inference_pairs,reduce_predictions,overlaps
from metrics import evaluate,risk_curve


def run(root,config):
    if (root/'frozen.json').exists():raise ValueError('research sealed')
    name=config['name'];out=root/'runs'
    if (out/(name+'.json')).exists():raise ValueError('preserve run')
    elapsed=sum(json.loads(p.read_text())['seconds'] for p in out.glob('*.json'))
    start=time.perf_counter();torch.set_num_threads(4);torch.set_num_interop_threads(1)
    torch.manual_seed(config.get('seed',17))
    real=read(root/'train-events.jsonl');dev=read(root/'dev-events.jsonl')
    training=training_pairs(real+generate(160,31)+hard_generate(160))
    pairs,groups=inference_pairs(dev)
    tokenizer=AutoTokenizer.from_pretrained(root/'pretrained',local_files_only=True)
    tokens=tokenize(training,tokenizer,'pair');dv=tokenize(pairs,tokenizer,'pair')
    gold=labels(training);weight=torch.tensor([r.get('edge_weight',1.) for r in training])
    model=RelationModel(root/'pretrained')
    # Four layers adjust, embedding table and lowest two layers stay frozen.
    for n,p in model.encoder.named_parameters():p.requires_grad=any(f'layer.{i}.' in n for i in range(2,6))
    optimizer=torch.optim.AdamW([{'params':[p for p in model.encoder.parameters() if p.requires_grad],'lr':3e-5},
        {'params':list(model.project.parameters())+list(model.heads.parameters()),'lr':8e-4}],weight_decay=.01)
    history=[];best=None;bestscore=-1
    for epoch in range(config.get('epochs',5)):
        if elapsed+time.perf_counter()-start>5400:raise RuntimeError('compute budget reached')
        model.train();total=0
        for idx in torch.randperm(len(training)).split(16):
            optimizer.zero_grad(set_to_none=True);logits,z=model(**{k:v[idx] for k,v in tokens.items()})
            loss=sum(w*(nn.functional.cross_entropy(logits[j],gold[idx,j],reduction='none')*weight[idx]).mean()
                for j,w in enumerate([1.,.6,.5,.3]))
            if config.get('contrast',0):loss+=config['contrast']*supervised_contrast(z,gold[idx,0])
            loss.backward();nn.utils.clip_grad_norm_(model.parameters(),1.);optimizer.step();total+=float(loss.detach())
        edge_logits=predict(model,dv);reduced,selected=reduce_predictions(edge_logits,groups)
        m=evaluate(dev,reduced,.5);score=(m['relation_macro_f1']+m['exact_all_accuracy'])/2
        m['witness_overlap']=sum(overlaps(pairs[i]['candidate_result_span'],r.get('result_span')) for i,r in zip(selected,dev))/len(dev)
        history.append(dict(epoch=epoch+1,loss=total/(len(training)/16),dev=m))
        print(name,epoch+1,round(m['relation_macro_f1'],3),m['accepted'],m['correct'],flush=True)
        if score>bestscore:
            bestscore=score;best={k:v.detach().clone() for k,v in model.state_dict().items()}
    model.load_state_dict(best);edge_logits=predict(model,dv);reduced,selected=reduce_predictions(edge_logits,groups)
    np.savez(out/(name+'.npz'),**{f'head{i}':p for i,p in enumerate(reduced)})
    result=dict(config=dict(config,mode='pair',pair=True),seconds=time.perf_counter()-start,
        train_n=len(training),real_train_n=len(real),epochs=history,selection_score=bestscore,curve=risk_curve(dev,reduced),
        train_hash=hashlib.sha256((root/'train-events.jsonl').read_bytes()).hexdigest(),dev_hash=hashlib.sha256((root/'dev-events.jsonl').read_bytes()).hexdigest(),
        witness_overlap=sum(overlaps(pairs[i]['candidate_result_span'],r.get('result_span')) for i,r in zip(selected,dev))/len(dev))
    torch.save(dict(state=model.state_dict(),config=result['config']),out/(name+'.pt'))
    (out/(name+'.json')).write_text(json.dumps(result,indent=2));print('completed',name,round(result['seconds'],1),flush=True)


if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('root',type=pathlib.Path);p.add_argument('config',type=pathlib.Path)
    a=p.parse_args();run(a.root,json.loads(a.config.read_text()))
