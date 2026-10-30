"""Bounded CPU relation fine-tuning and ablations. All evaluation here is DEV.

Final files are intentionally never loaded by this module.
"""
import os
os.environ.setdefault("OMP_NUM_THREADS", "4")
os.environ.setdefault("OPENBLAS_NUM_THREADS", "4")
import argparse
import hashlib
import json
import pathlib
import random
import resource
import time
import numpy as np
import torch
from torch import nn
from transformers import AutoTokenizer
from model import RelationModel, labels, tokenize, predict, supervised_contrast
from metrics import evaluate, risk_curve
from synthetic import generate, hard_generate
from budget import spent


def read(path):
    return [json.loads(x) for x in path.read_text().splitlines()]


def run(root, config):
    if (root/'frozen.json').exists():raise ValueError('research sealed; use a new protocol/root for further training')
    out = root / "runs"; out.mkdir(exist_ok=True)
    name = config["name"]
    if (out / (name + ".json")).exists():
        raise ValueError("preserve completed run; use a distinct name")
    elapsed = spent(root)
    if name=='E17-expanded-seed29' and elapsed>4500:
        (root/'skipped-E17.json').write_text(json.dumps(dict(reason='reserve compute for independent retrieval experiments',elapsed_seconds=elapsed)))
        print('E17 not started: reserve remaining budget for retrieval',flush=True)
        return
    if elapsed >= 5400:
        raise ValueError("90 minute compute budget exhausted")
    start = time.perf_counter()
    torch.set_num_threads(4); torch.set_num_interop_threads(1)
    seed = config.get("seed", 17)
    random.seed(seed); np.random.seed(seed); torch.manual_seed(seed)
    train_file=root / config.get("train_file","train-events.jsonl")
    train = read(train_file)
    if config.get("subset_ids"):
        train = [r for r in train if r["id"] in config["subset_ids"]]
    dev_file=root / config.get('dev_file','dev-events.jsonl')
    dev = read(dev_file)
    assert not {r["app_id"] for r in train} & {r["app_id"] for r in dev}
    synthetic=generate(config.get('synthetic',0),seed=31)+hard_generate(config.get('hard',0))
    if config.get('goal_pairs',False):
        from goals import synthetic_goals
        synthetic=synthetic_goals(synthetic)
    train+=synthetic
    if config.get("weak", False):
        train += read(root/"weak-events.jsonl")
    tokenizer = AutoTokenizer.from_pretrained(root / "pretrained", local_files_only=True)
    mode = config.get("mode", "marked")
    tr = tokenize(train, tokenizer, mode); dv = tokenize(dev, tokenizer, mode)
    y = labels(train)
    outcome_weights=None
    if config.get('outcome_class_balance',False):
        counts=torch.bincount(y[:,0],minlength=5).float().clamp(min=1)
        outcome_weights=len(train)/(5*counts)
    sample_weight = torch.tensor([config.get("weak_weight",.25) if r["origin"]=="weak-supervised-real" else config.get("synthetic_weight",1.) if r["origin"]=="synthetic-training-only" else 1. for r in train])
    sample_weight*=torch.tensor([r.get('edge_weight',1.) for r in train])
    model = RelationModel(root / "pretrained")
    layers = config.get("layers", 2)
    for n, p in model.encoder.named_parameters():
        p.requires_grad = layers == 6 or any(f"layer.{i}." in n for i in range(6-layers,6))
    optimizer = torch.optim.AdamW([
        {"params":[p for p in model.encoder.parameters() if p.requires_grad],"lr":config.get("lr",3e-5)},
        {"params":list(model.project.parameters()) + list(model.heads.parameters()),"lr":config.get("head_lr",8e-4)}],weight_decay=.01)
    losses = []
    best = None; best_score = -1.
    for epoch in range(config.get("epochs", 5)):
        if time.perf_counter()-start + elapsed > 5400:
            raise RuntimeError("compute budget reached during run")
        model.train(); total = 0.
        indices=torch.randperm(len(train))
        if config.get("curriculum_after") is not None and epoch>=config["curriculum_after"]:
            indices=indices[torch.tensor([train[int(i)]["origin"]!="synthetic-training-only" for i in indices])]
        for idx in indices.split(16):
            optimizer.zero_grad(set_to_none=True)
            logits,z = model(**{k:v[idx] for k,v in tr.items()})
            weights = config.get("task_weights",[1.,.4,.35,.2]) if config.get("multitask",True) else [1.,0.,0.,0.]
            loss = sum(w*(nn.functional.cross_entropy(logits[j],y[idx,j],reduction="none",weight=outcome_weights if j==0 else None)*sample_weight[idx]).mean() for j,w in enumerate(weights))
            if config.get("contrast",0):
                loss += config["contrast"] * supervised_contrast(z,y[idx,0])
            loss.backward(); nn.utils.clip_grad_norm_(model.parameters(),1.)
            optimizer.step();total += float(loss.detach())
        dev_logits = predict(model,dv)
        result = evaluate(dev,dev_logits,.5)
        # Predeclared trade-off, no final data: exact 4-field accuracy and
        # relation macro F1. The last epoch is also retained as an ablation.
        score = (result["exact_all_accuracy"]+result["relation_macro_f1"])/2
        losses.append(dict(epoch=epoch+1,loss=total/(len(indices)/16),dev=result))
        print(name,epoch+1,round(total/(len(indices)/16),3),
              round(result["relation_macro_f1"],3),result["accepted"],result["correct"],flush=True)
        if score > best_score:
            best_score = score
            best = {k:v.detach().clone() for k,v in model.state_dict().items()}
    model.load_state_dict(best)
    dev_logits = predict(model,dv)
    np.savez(out/(name+".npz"),**{f"head{i}":v for i,v in enumerate(dev_logits)})
    result = dict(config=config,seconds=time.perf_counter()-start,
                  peak_rss_kib=resource.getrusage(resource.RUSAGE_SELF).ru_maxrss,
                  train_n=len(train),real_train_n=sum(r["origin"]=="real-public-github" for r in train),
                  weak_train_n=sum(r["origin"]=="weak-supervised-real" for r in train),
                  synthetic_train_n=sum(r["origin"]=="synthetic-training-only" for r in train),
                  train_hash=hashlib.sha256(train_file.read_bytes()).hexdigest(),
                  dev_hash=hashlib.sha256(dev_file.read_bytes()).hexdigest(),
                  epochs=losses,selection_score=best_score,curve=risk_curve(dev,dev_logits))
    # Retain one checkpoint per explicit run request, bounded by caller; other
    # runs preserve metrics and logits without retaining 90 MB weights each.
    if config.get("save",False):
        torch.save(dict(state=model.state_dict(),config=config),out/(name+".pt"))
    (out/(name+".json")).write_text(json.dumps(result,indent=2))
    print("completed",name,round(result["seconds"],2),"seconds",flush=True)


if __name__ == "__main__":
    p=argparse.ArgumentParser();p.add_argument("root",type=pathlib.Path);p.add_argument("config",type=pathlib.Path)
    args=p.parse_args();run(args.root,json.loads(args.config.read_text()))
