"""Controlled atomic-verifier fits, with old known development only."""
import argparse,time,json,hashlib
from pathlib import Path
import numpy as np,torch
from transformers import AutoTokenizer
from common import read,write
from syntax import Syntax
from atomic import Atomic,STATES,encode,probability
from training_data import synthetic,units

CONFIGS={'A1-real-paragraph':dict(synthetic=0,scope='paragraph',features=False),'A2-contrast-paragraph':dict(synthetic=120,scope='paragraph',features=False),'A3-contrast-sentence':dict(synthetic=120,scope='sentence',features=False),'A4-syntax-features':dict(synthetic=120,scope='sentence',features=True)}

CONFIGS['A4-coverage-control']=dict(CONFIGS['A4-syntax-features'],selector='coverage')
CONFIGS['A5-audited-data']=dict(CONFIGS['A4-syntax-features'],selector='coverage',extra=True)
CONFIGS['A6-seed-control']=dict(CONFIGS['A5-audited-data'],seed_after_encoding=True)

def run(root,old,name):
 if (root/'frozen.json').exists():raise ValueError('sealed; no more fitting')
 out=root/'atomic-runs';out.mkdir(exist_ok=True);dest=out/(name+'.json')
 if dest.exists():raise ValueError('preserve completed fit')
 cfg=CONFIGS[name];start=time.perf_counter();torch.set_num_threads(4);torch.set_num_interop_threads(1);torch.manual_seed(55)
 parser=Syntax(root);tokenizer=AutoTokenizer.from_pretrained(old/'pretrained',local_files_only=True);train=units(read(root/'train-known.jsonl')+(read(root/'extra-audited.jsonl') if cfg.get('extra') else [])+synthetic(cfg['synthetic']));dev=units(read(root/'dev-known.jsonl'))
 x=encode(train,tokenizer,parser,cfg['scope']);v=encode(dev,tokenizer,parser,cfg['scope']);labels=torch.tensor([[float(r['status']==c) for c in STATES] for r in train])
 # New controlled variant: lazy pretrained-parser construction consumes the
 # Torch RNG on a cold cache. Reseed here so cache state cannot choose weights.
 # Preserve A1-A5's historical behavior rather than rewrite their records.
 if cfg.get('seed_after_encoding'):torch.manual_seed(55)
 input_audit=dict(real_events_expected=sum(len(r['events']) for r in read(root/'train-known.jsonl')+(read(root/'extra-audited.jsonl') if cfg.get('extra') else [])),total_training_units=len(train),unit_sha256=hashlib.sha256(json.dumps(train,sort_keys=True).encode()).hexdigest(),feature_sha256={k:hashlib.sha256(v.numpy().tobytes()).hexdigest() for k,v in x.items()})
 write(out/(name+'-inputs.json'),input_audit)
 model=Atomic(old/'pretrained',cfg['features'])
 for n,p in model.encoder.named_parameters():p.requires_grad=any(f'layer.{i}.' in n for i in [4,5])
 optimizer=torch.optim.AdamW([p for p in model.parameters() if p.requires_grad],lr=4e-5,weight_decay=.01);weight=(len(train)-labels.sum(0))/labels.sum(0).clamp(min=1);loss_fn=torch.nn.BCEWithLogitsLoss(pos_weight=weight.clamp(max=8));history=[];best=-1
 for epoch in range(6):
  model.train();losses=[]
  for ids in torch.randperm(len(train)).split(12):
   optimizer.zero_grad(set_to_none=True);logits=model(**{k:t[ids] for k,t in x.items()});loss=loss_fn(logits,labels[ids]);loss.backward();torch.nn.utils.clip_grad_norm_(model.parameters(),1.);optimizer.step();losses.append(float(loss.detach()))
  p=probability(model,v);pred=np.where(p.max(1)>=.6,p.argmax(1)+1,0);reference=np.array([STATES.index(r['status'])+1 if r['status'] in STATES else 0 for r in dev]);accuracy=float((pred==reference).mean());performed=int((pred==1).sum());correct=int(((pred==1)&(reference==1)).sum());score=correct-2*(performed-correct)
  if cfg.get('selector')=='coverage':
   recall=correct/max(1,int((reference==1).sum()));precision=correct/max(1,performed);score=2*precision*recall/(precision+recall) if precision+recall else 0
  history.append(dict(epoch=epoch+1,loss=float(np.mean(losses)),supplied_span_accuracy=accuracy,performed_claims=performed,performed_correct=correct,selection_score=score));print(name,history[-1],flush=True)
  if score>best:
   best=score;state={k:v.detach().clone() for k,v in model.state_dict().items() if k.startswith('head.') or any(f'layer.{i}.' in k for i in [4,5])};torch.save(dict(state=state,config=cfg),out/(name+'.pt'));np.save(out/(name+'-dev.npy'),p)
 write(dest,dict(name=name,config=cfg,real_training_events=sum(len(r['events']) for r in read(root/'train-known.jsonl')+(read(root/'extra-audited.jsonl') if cfg.get('extra') else [])),synthetic_training_events=cfg['synthetic'],seconds=time.perf_counter()-start,epochs=history,seed=55,quality='agent-provisional labels; supplied-trigger development control; not end-to-end extraction'))

if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('root',type=Path);p.add_argument('old',type=Path);p.add_argument('name',choices=CONFIGS);a=p.parse_args();run(a.root,a.old,a.name)
