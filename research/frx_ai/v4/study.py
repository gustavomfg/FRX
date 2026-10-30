"""Comparable development evaluation after all exploratory model fits."""
import argparse
from pathlib import Path
import gc,json,time
import torch
from transformers import AutoTokenizer
from data import read,write,STATUSES
from model import EventModel,predictions
from metrics import evaluate
from proof import lexical,verify,attach_observations,temporal_edges
from proposals import fit,propose

def legacy_status(rows,old,v3root):
 import sys
 from pathlib import Path
 prior=Path(__file__).resolve().parents[1]/'v3';sys.path.append(str(prior))
 from events import EventHeads
 from common import near
 heads=EventHeads(v3root,old);units=[];counts=[]
 for row in rows:
  counts.append(len(row['events']))
  for event in row['events']:
   units.append(dict(text=row['text'][slice(*event['trigger'])],context=near(row['text'],event['trigger'])))
 probability=heads.predict(units,'modality');labels=['unknown','suggested','performed'];out=[];start=0
 for row,n in zip(rows,counts):
  events=[]
  for j,gold in enumerate(row['events']):
   # Supplied-span control: no detection advantage is claimed.
   item=verify(row['text'],gold['trigger'])
   if item is None:item=dict(trigger={'span':gold['trigger'],'text':row['text'][slice(*gold['trigger'])]},status='unknown',direction='unknown')
   item['status']=labels[int(probability[start+j].argmax())] if probability[start+j].max()>=.8 else 'unknown';events.append(item)
  out.append(dict(events=events,observations=[]));start+=n
 return out

def run(root,old,v3root):
 if (root/'frozen.json').exists():raise ValueError('selection already sealed')
 torch.set_num_threads(4);rows=read(root/'dev-audited.jsonl');train=read(root/'train-audited.jsonl');lexicon=fit(train);write(root/'lexicon.json',lexicon)
 results={};raw_predictions={};start=time.perf_counter();tok=AutoTokenizer.from_pretrained(old/'pretrained',local_files_only=True)
 for name in ['S1-legacy-span','S2-audited-real','S3-contrast-events','S4-frozen-body','S5-balanced-events','S6-balanced-no-contrast']:
  file=root/'runs'/(name+'.pt')
  if not file.exists():continue
  checkpoint=torch.load(file,map_location='cpu',weights_only=True);model=EventModel(old/'pretrained');model.load_state_dict(checkpoint['state'])
  for variant,checked,whole,head in [('raw-subword',False,False,False),('raw-word',False,True,False),('verified-word',True,True,False),('raw-head',False,True,True),('verified-head',True,True,True)]:
   key=name+'/'+variant;out=predictions(model,tok,rows,verified=checked,whole_word=whole,head_projection=head)
   results[key]=evaluate(rows,out,strict=True);raw_predictions[key]=out
  del model,checkpoint;gc.collect()
 for name,lex in [('fixed-lexical-verifier',None),('training-lexicon-verifier',lexicon)]:
  out=[propose(r['text'],lex) for r in rows];results[name]=evaluate(rows,out,strict=True);raw_predictions[name]=out
 supplied=[]
 for row in rows:
  events=[item for e in row['events'] if (item:=verify(row['text'],e['trigger'])) is not None];supplied.append(dict(events=events,observations=[]))
 results['supplied-spans/verifier']=evaluate(rows,supplied,strict=True)
 results['supplied-spans/v3-learned']=evaluate(rows,legacy_status(rows,old,v3root),strict=True)
 # Select the useful supported-event pipeline on development, never final labels.
 candidates=[(k,v) for k,v in results.items() if k.endswith(('verified-word','verified-head','lexical-verifier')) or k=='training-lexicon-verifier']
 eligible=[(k,v) for k,v in candidates if v['performed_claims']>=3 and (v['performed_precision'] or 0)>=.8]
 winner=max(eligible or candidates,key=lambda pair:(pair[1]['performed_correct']-2*pair[1]['false_performed'],pair[1]['trigger_f1']))[0]
 selection=dict(pipeline=winner,neural_model=winner.split('/')[0] if '/' in winner else None,
  criterion='development performed-correct minus twice false-performed; >=3 claims and >=.8 precision if available; trigger F1 tiebreak',
  automatic_approval=False)
 write(root/'development.json',results);write(root/'development-predictions.json',raw_predictions);write(root/'selection.json',selection)
 write(root/'study-timing.json',dict(seconds=time.perf_counter()-start))
 print('Selected',winner)
 for name,m in results.items():print(name,'F1',round(m['trigger_f1'],3),'performed',m['performed_correct'],m['performed_claims'],'false',m['false_performed'],'tuple',m['accepted_tuple_precision'])

if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('root',type=Path);p.add_argument('old',type=Path);p.add_argument('v3',type=Path);a=p.parse_args();run(a.root,a.old,a.v3)
