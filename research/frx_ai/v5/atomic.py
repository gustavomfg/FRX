"""Target-marked atomic status verifier, genuinely fine-tuned in v5.

Four independent binary claims replace a forced whole-comment class. Span
proposal, syntax evidence, outcome reconstruction and execution remain separate.
"""
import os
os.environ.setdefault('OMP_NUM_THREADS','4');os.environ.setdefault('TOKENIZERS_PARALLELISM','false')
import re,torch,numpy as np
from torch import nn
from transformers import AutoModel,AutoTokenizer
from events import children,governing

STATES=['performed','suggested','negated','preexisting']

class Atomic(nn.Module):
 def __init__(self,pretrained,features=False):
  super().__init__();self.encoder=AutoModel.from_pretrained(pretrained,local_files_only=True,attn_implementation='eager');h=self.encoder.config.hidden_size;self.features=features
  self.head=nn.Sequential(nn.Dropout(.1),nn.Linear(h*2+(14 if features else 0),4))
 def forward(self,input_ids,attention_mask,target_mask,features):
  hidden=self.encoder(input_ids=input_ids,attention_mask=attention_mask).last_hidden_state;pooled=(hidden*target_mask.unsqueeze(-1)).sum(1)/target_mask.sum(1).clamp(min=1).unsqueeze(-1)
  values=[hidden[:,0],pooled]
  if self.features:values.append(features)
  return self.head(torch.cat(values,1))

def grammar_features(parsed,span):
 words=parsed['words'];hits=[w for w in words if min(w['span'][1],span[1])>max(w['span'][0],span[0])]
 if not hits:return [0.]*14
 w=hits[0];chain=governing(words,w);direct=children(words,w['i']);related=[c for p in chain for c in children(words,p['i'])]
 lemmas={x['lemma'].lower() for x in related};subject={x['lemma'].lower() for x in related if x['rel'].startswith('nsubj')}
 return [float(x) for x in [w['tag']=='VBD',w['tag']=='VBN',w['tag']=='VBG',bool(subject&{'i','we'}),bool(subject&{'you'}),bool(lemmas&{'not','never'}),bool(lemmas&{'will','should','could','would','may','might','can'}),any(x['lemma'].lower()=='try' for x in chain),any(x['rel'].endswith(':pass') for x in direct),w['rel']=='conj','already' in lemmas,bool(lemmas&{'if','unless'}),w['rel'] in ['xcomp','advcl','acl'],w['pos']=='NOUN']]

def encode(units,tokenizer,parser,scope='paragraph',max_length=192):
 tensors={'input_ids':[],'attention_mask':[],'target_mask':[],'features':[]}
 for unit in units:
  text=unit['text'];a,b=unit['span'];parsed=parser.parse(text)
  left,right=0,len(text)
  if scope=='sentence':
   sentence=next((s for s in parsed['sentences'] if s['span'][0]<=a<s['span'][1]),None)
   if sentence:left,right=sentence['span']
  encoded=tokenizer(text[left:right],add_special_tokens=False,return_offsets_mapping=True);positions=encoded['offset_mapping'];active=[j for j,(x,y) in enumerate(positions) if min(y+left,b)>max(x+left,a)]
  if not active:raise ValueError('target tokenization absent')
  first,last=active[0],active[-1];start=max(0,first-(max_length-6)//2);end=min(len(positions),start+max_length-4)
  if end<=last:end=last+1;start=max(0,end-max_length+4)
  ids=[tokenizer.cls_token_id];mask=[0.]
  for j in range(start,end):
   if j==first:ids.append(tokenizer.convert_tokens_to_ids('[unused1]'));mask.append(0.)
   ids.append(encoded['input_ids'][j]);mask.append(float(j in active))
   if j==last:ids.append(tokenizer.convert_tokens_to_ids('[unused2]'));mask.append(0.)
  ids.append(tokenizer.sep_token_id);mask.append(0.);attention=[1]*len(ids);pad=max_length-len(ids)
  if pad<0:raise ValueError('atomic context bound')
  tensors['input_ids'].append(ids+[0]*pad);tensors['attention_mask'].append(attention+[0]*pad);tensors['target_mask'].append(mask+[0.]*pad);tensors['features'].append(grammar_features(parsed,[a,b]))
 return {k:torch.tensor(v,dtype=torch.float if k in ['features','target_mask'] else torch.long) for k,v in tensors.items()}

def probability(model,tensors):
 model.eval();out=[]
 with torch.inference_mode():
  for ids in torch.arange(len(tensors['input_ids'])).split(16):out.append(model(**{k:v[ids] for k,v in tensors.items()}).sigmoid().numpy())
 return np.concatenate(out) if out else np.empty((0,4))
