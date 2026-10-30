"""Four targeted ablations: annotation unit, real-only, contrast, frozen body."""
import argparse
from pathlib import Path
import json,time,hashlib
import numpy as np
import torch
from torch import nn
from transformers import AutoTokenizer
from data import read,write
from model import EventModel,tokenize,predictions
from metrics import evaluate
from synthetic import build

CONFIGS={
 'S1-legacy-span':dict(legacy=True,synthetic=0,layers=2,contrast=0.),
 'S2-audited-real':dict(legacy=False,synthetic=0,layers=2,contrast=0.),
 'S3-contrast-events':dict(legacy=False,synthetic=160,layers=2,contrast=.05),
 'S4-frozen-body':dict(legacy=False,synthetic=160,layers=0,contrast=.05),
 'S5-balanced-events':dict(legacy=False,synthetic=160,layers=2,contrast=.05,balanced=True),
 'S6-balanced-no-contrast':dict(legacy=False,synthetic=160,layers=2,contrast=0.,balanced=True)}

def contrast(hidden,labels):
 # Contrast event status representations, excluding non-event and padding tokens.
 mask=labels!=-100
 if int(mask.sum())<3:return hidden.sum()*0
 z=nn.functional.normalize(hidden[mask],dim=-1);y=labels[mask];z=z[:128];y=y[:128]
 same=(y[:,None]==y[None,:]);eye=torch.eye(len(z),dtype=torch.bool);positive=same&~eye
 score=z@z.T/.12;score=score.masked_fill(eye,-1e4);log=score-torch.logsumexp(score,1,keepdim=True);valid=positive.sum(1)>0
 return -(log*positive).sum(1)[valid].div(positive.sum(1)[valid]).mean() if valid.any() else hidden.sum()*0

def run(root,old,name):
 if (root/'frozen.json').exists():raise ValueError('sealed research; no training')
 out=root/'runs';out.mkdir(exist_ok=True)
 if (out/(name+'.json')).exists():raise ValueError('preserve completed experiment')
 start=time.perf_counter();torch.set_num_threads(4);torch.set_num_interop_threads(1);torch.manual_seed(17);cfg=CONFIGS[name]
 train=read(root/'train-audited.jsonl');dev=read(root/'dev-audited.jsonl');assert not {r['app_id'] for r in train}&{r['app_id'] for r in dev}
 real=len(train);train+=build(cfg['synthetic']);tokenizer=AutoTokenizer.from_pretrained(old/'pretrained',local_files_only=True);model=EventModel(old/'pretrained')
 for n,p in model.encoder.named_parameters():p.requires_grad=any(f'layer.{i}.' in n for i in range(6-cfg['layers'],6))
 optimizer=torch.optim.AdamW([p for p in model.parameters() if p.requires_grad],lr=4e-5,weight_decay=.01)
 tokens,offsets,labels=tokenize(train,tokenizer,cfg['legacy']);ratio=float((labels[0]==0).sum()/max(1,int((labels[0]==1).sum())));positive_weight=min(80.,ratio) if cfg.get('balanced') else 8.;criterion=nn.CrossEntropyLoss(weight=torch.tensor([1.,positive_weight]));history=[];best=-1;weights=None
 for epoch in range(8):
  model.train();losses=[]
  for ids in torch.randperm(len(train)).split(8):
   optimizer.zero_grad(set_to_none=True);det,st,di,h=model(**{k:v[ids] for k,v in tokens.items()});yd,ys,yi=[y[ids] for y in labels]
   loss=criterion(det.reshape(-1,2),yd.reshape(-1))
   if (ys!=-100).any():loss=loss+.5*nn.functional.cross_entropy(st.reshape(-1,st.shape[-1]),ys.reshape(-1))+.25*nn.functional.cross_entropy(di.reshape(-1,di.shape[-1]),yi.reshape(-1))
   if cfg['contrast']:loss+=cfg['contrast']*contrast(h,ys)
   loss.backward();torch.nn.utils.clip_grad_norm_(model.parameters(),1.);optimizer.step();losses.append(float(loss.detach()))
  raw=evaluate(dev,predictions(model,tokenizer,dev));checked=evaluate(dev,predictions(model,tokenizer,dev,True));score=raw['trigger_f1']*(.5+.5*(raw['status_accuracy_on_matched'] or 0))
  history.append(dict(epoch=epoch+1,loss=float(np.mean(losses)),raw=raw,verified=checked,selection_score=score))
  print(name,epoch+1,'triggerF1',round(raw['trigger_f1'],3),'performed',raw['performed_correct'],raw['performed_claims'],'verified',checked['performed_correct'],checked['performed_claims'],flush=True)
  if score>best:best=score;weights={k:v.detach().clone() for k,v in model.state_dict().items()}
 torch.save(dict(state=weights,config=cfg),out/(name+'.pt'));write(out/(name+'.json'),dict(name=name,config=cfg,real_training_paragraphs=real,synthetic_training=cfg['synthetic'],positive_token_weight=positive_weight,seconds=time.perf_counter()-start,epochs=history,selection_score=best))

if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('root',type=Path);p.add_argument('old',type=Path);p.add_argument('name',choices=CONFIGS);a=p.parse_args();run(a.root,a.old,a.name)
