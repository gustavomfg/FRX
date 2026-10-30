"""Post-freeze native-versus-training-runtime consistency check on known dev data."""
import argparse,json
from pathlib import Path
import torch
from transformers import AutoTokenizer
from data import read,write
from model import EventModel,predictions
from native import NativeEvents
from metrics import evaluate

def clean(item):
 return [{k:v for k,v in e.items() if k!='proposal_score'} for e in item['events']]

def run(root,old):
 torch.set_num_threads(4);runtime=NativeEvents(root/'runtime-events');name=runtime.meta['model_name'];model=EventModel(old/'pretrained');model.load_state_dict(torch.load(root/'runs'/(name+'.pt'),map_location='cpu',weights_only=True)['state']);tokenizer=AutoTokenizer.from_pretrained(old/'pretrained',local_files_only=True)
 rows=read(root/'dev-audited.jsonl');a=predictions(model,tokenizer,rows,verified=True,whole_word=True,head_projection=runtime.meta['head_projection']);b=runtime.predict(rows)
 disagreements=[r['id'] for r,x,y in zip(rows,a,b) if clean(x)!=clean(y)]
 result=dict(scope='known development units; engineering parity, not independent accuracy',documents=len(rows),event_content_disagreements=disagreements,torch_metrics=evaluate(rows,a,strict=True),native_metrics=evaluate(rows,b,strict=True))
 write(root/'parity.json',result);print('Native event-content disagreements:',len(disagreements),'/',len(rows))
 if disagreements:raise ValueError('native parity changed; report without tuning frozen model')

if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('root',type=Path);p.add_argument('old',type=Path);a=p.parse_args();run(a.root,a.old)
