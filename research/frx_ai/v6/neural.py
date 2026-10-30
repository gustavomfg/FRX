"""Compact cross-encoder controls. Local training only; never authorizes repairs."""
import argparse,json,time,hashlib,random,importlib.metadata
from pathlib import Path
import numpy as np
import torch
from torch import nn
from transformers import AutoModel,AutoTokenizer
from bridge import read,write,digest,sha,immutable,CompactSyntax
from grounding import deconstruct
from pairs import candidates
from learning import choose,pair_text

CONFIGS={'N1-real-text':dict(synthetic=False,features=False,seed=606),'N2-contrast':dict(synthetic=True,features=False,seed=606),'N3-structured-contrast':dict(synthetic=True,features=True,seed=606),'N4-seed-control':dict(synthetic=True,features=True,seed=607)}
class Verifier(nn.Module):
 def __init__(self,old,features):
  super().__init__();self.encoder=AutoModel.from_pretrained(old/'pretrained',local_files_only=True,attn_implementation='eager');self.features=features;self.head=nn.Sequential(nn.Dropout(.1),nn.Linear(384+(19 if features else 0),3))
 def forward(self,input_ids,attention_mask,token_type_ids,x):
  h=self.encoder(input_ids=input_ids,attention_mask=attention_mask,token_type_ids=token_type_ids).last_hidden_state[:,0]
  return self.head(torch.cat([h,x],-1) if self.features else h)
def tensors(texts,features,tokenizer):
 batch=tokenizer(texts,truncation=True,padding='max_length',max_length=224,return_tensors='pt');batch['x']=torch.tensor(features,dtype=torch.float32);return batch
def probability(model,x):
 model.eval();out=[]
 with torch.inference_mode():
  for ids in torch.arange(len(x['input_ids'])).split(16):out.append(model(**{k:v[ids] for k,v in x.items()}).softmax(-1).numpy())
 return np.concatenate(out) if out else np.zeros((0,3))
def fit(root,old,name):
 immutable(root);out=root/'neural';out.mkdir(exist_ok=True);dest=out/(name+'.json')
 if dest.exists():raise ValueError('preserve completed fit')
 cfg=CONFIGS[name];torch.set_num_threads(4);torch.set_num_interop_threads(1);start=time.perf_counter();data=read(root/'training-pairs.jsonl')
 if not cfg['synthetic']:data=[r for r in data if not r['source_origin'].startswith('explicit-synthetic')]
 tokenizer=AutoTokenizer.from_pretrained(old/'pretrained',local_files_only=True);x=tensors([r['text'] for r in data],[r['x'] for r in data],tokenizer);y=torch.tensor([r['y'] for r in data]);parser=CompactSyntax(root);catalog=json.loads((root/'catalog.json').read_text());rows=read(root/'development-gold-v2.jsonl');prepared=[]
 # Exhaust all parsing/tokenization before setting initialization seeds.
 for row in rows:
  b=deconstruct(row,parser,catalog);ps=candidates(row['text'],b);prepared.append((row,b,ps))
 texts=[pair_text(row,b,p) for row,b,ps in prepared for p in ps];feats=[p['features'] for row,b,ps in prepared for p in ps];v=tensors(texts,feats,tokenizer)
 random.seed(cfg['seed']);np.random.seed(cfg['seed']);torch.manual_seed(cfg['seed']);model=Verifier(old,cfg['features'])
 for n,p in model.encoder.named_parameters():p.requires_grad=any(f'layer.{i}.' in n for i in [4,5])
 opt=torch.optim.AdamW([p for p in model.parameters() if p.requires_grad],lr=4e-5,weight_decay=.01);counts=torch.bincount(y,minlength=3);weight=len(y)/(3*counts.clamp(min=1));loss_fn=nn.CrossEntropyLoss(weight=weight);history=[];best=-100
 input_audit=dict(data_sha256=digest(json.dumps(data,sort_keys=True)),train_tensor_sha256={k:hashlib.sha256(t.numpy().tobytes()).hexdigest() for k,t in x.items()},dev_tensor_sha256={k:hashlib.sha256(t.numpy().tobytes()).hexdigest() for k,t in v.items()},classes=counts.tolist(),versions={n:importlib.metadata.version(n) for n in ['torch','transformers','numpy','tokenizers']})
 for epoch in range(5):
  model.train();losses=[]
  for ids in torch.randperm(len(y)).split(12):
   opt.zero_grad(set_to_none=True);loss=loss_fn(model(**{k:t[ids] for k,t in x.items()}),y[ids]);loss.backward();torch.nn.utils.clip_grad_norm_(model.parameters(),1);opt.step();losses.append(float(loss.detach()))
  probs=probability(model,v);lookup={row['id']:probs[sum(len(z[2]) for z in prepared[:i]):sum(len(z[2]) for z in prepared[:i+1]),1] for i,(row,b,ps) in enumerate(prepared)}
  def verify(row,b,p):return lookup[row['id']]
  result,pred=choose(root,rows,parser,catalog,verify,name,[.3,.4,.5,.6,.7,.8]);selection=result['selection'];m=selection['metrics'];p=m['relation_precision'] or 0;r=m['relation_recall'] or 0;f=1.25*p*r/(.25*p+r) if p+r else 0;score=(10 if selection['eligible'] else 0)+f-2*m['false_success']/max(1,m['relations']);record=dict(epoch=epoch+1,loss=float(np.mean(losses)),selection=selection);history.append(record);print(name,epoch+1,m['relation_correct'],m['relations'],m['false_success'],selection['eligible'],flush=True)
  if score>best:
   best=score;state={k:t.detach().clone() for k,t in model.state_dict().items() if k.startswith('head.') or any(f'layer.{i}.' in k for i in [4,5])};torch.save(dict(state=state,config=cfg),out/(name+'.pt'));np.save(out/(name+'-dev.npy'),probs);write(out/(name+'-selection.json'),selection);write(out/(name+'-predictions.json'),pred)
 write(dest,dict(name=name,config=cfg,examples=len(y),seconds=time.perf_counter()-start,input_audit=input_audit,checkpoint_sha256=sha(out/(name+'.pt')),history=history,automatic_approval=False))
if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('root',type=Path);p.add_argument('old',type=Path);p.add_argument('name',choices=CONFIGS);a=p.parse_args();fit(a.root,a.old,a.name)
