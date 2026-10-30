"""Actual token-level encoder fine-tuning, not a comment sentiment head."""
import os
os.environ.setdefault('OMP_NUM_THREADS','4')
os.environ.setdefault('OPENBLAS_NUM_THREADS','4')
os.environ.setdefault('TOKENIZERS_PARALLELISM','false')
import torch
import numpy as np
from torch import nn
from transformers import AutoModel,AutoTokenizer
from data import STATUSES,DIRECTIONS,VERBS
from proof import verify,attach_observations,quoted,direction

class EventModel(nn.Module):
 def __init__(self,pretrained):
  super().__init__();self.encoder=AutoModel.from_pretrained(pretrained,local_files_only=True,attn_implementation='eager');h=self.encoder.config.hidden_size
  self.detect=nn.Linear(h,2);self.status=nn.Linear(h,len(STATUSES));self.direction=nn.Linear(h,len(DIRECTIONS))
 def forward(self,**tokens):
  hidden=self.encoder(**tokens).last_hidden_state
  return self.detect(hidden),self.status(hidden),self.direction(hidden),hidden

def tokenize(rows,tokenizer,legacy=False):
 encoded=tokenizer([r['text'] for r in rows],padding='max_length',truncation=True,max_length=256,return_offsets_mapping=True,return_tensors='pt')
 offsets=encoded.pop('offset_mapping');detection=torch.zeros(offsets.shape[:2],dtype=torch.long);status=torch.full_like(detection,-100);directions=torch.full_like(detection,-100)
 for i,row in enumerate(rows):
  events=row['events']
  if legacy and 'legacy_span' in row:
   state={'tested':'performed','suggested':'suggested','unknown':'unknown'}[row['legacy_modality']]
   events=[dict(trigger=row['legacy_span'],status=state,direction='unknown')]
  for j,(a,b) in enumerate(offsets[i].tolist()):
   if a==b or not encoded['attention_mask'][i,j]:detection[i,j]=-100;continue
   matches=[e for e in events if min(b,e['trigger'][1])>max(a,e['trigger'][0])]
   if matches:
    e=matches[0];detection[i,j]=1;status[i,j]=STATUSES.index(e['status']);directions[i,j]=DIRECTIONS.index(e['direction'])
 return encoded,offsets,(detection,status,directions)

def predictions(model,tokenizer,rows,verified=False,threshold=.6,whole_word=False,head_projection=False):
 from decode import decode
 encoded=tokenizer([r['text'] for r in rows],padding='max_length',truncation=True,max_length=256,return_offsets_mapping=True,return_tensors='pt');offsets=encoded.pop('offset_mapping');tokens=encoded;outputs=[];model.eval()
 with torch.inference_mode():
  for start in range(0,len(rows),8):
   logits=model(**{k:v[start:start+8] for k,v in tokens.items()});prob=[torch.softmax(v,dim=-1).numpy() for v in logits[:3]]
   for ix,row in enumerate(rows[start:start+8]):
    outputs.append(decode(row,offsets[start+ix].tolist(),[p[ix] for p in prob],verified,threshold,whole_word,head_projection))
 return outputs
