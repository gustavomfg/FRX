"""Decode frozen experimental checkpoints with the final development verifier.
This is inference, not refitting or independent validation.
"""
import argparse,json,time,hashlib
from pathlib import Path
import torch
from transformers import AutoTokenizer
from bridge import read,write,sha,immutable,CompactSyntax
from grounding import deconstruct
from pairs import candidates
from learning import pair_text,choose
from neural import Verifier,tensors,probability
def run(root,old):
 immutable(root);dest=root/'neural-decoding.json'
 if dest.exists():raise ValueError('preserve model replay')
 torch.set_num_threads(4);rows=read(root/'development-gold-v2.jsonl');parser=CompactSyntax(root);catalog=json.loads((root/'catalog.json').read_text());prepared=[]
 for row in rows:
  b=deconstruct(row,parser,catalog);prepared.append((row,b,candidates(row['text'],b)))
 tok=AutoTokenizer.from_pretrained(old/'pretrained',local_files_only=True);texts=[pair_text(row,b,p) for row,b,ps in prepared for p in ps];v=tensors(texts,[p['features'] for row,b,ps in prepared for p in ps],tok);hashes={k:hashlib.sha256(t.numpy().tobytes()).hexdigest() for k,t in v.items()};result={}
 for name in ['N1-real-text','N2-contrast','N3-structured-contrast']:
  before=time.perf_counter();meta=json.loads((root/'neural'/(name+'.json')).read_text());path=root/'neural'/(name+'.pt')
  if sha(path)!=meta['checkpoint_sha256']:raise ValueError('experimental checkpoint changed')
  saved=torch.load(path,map_location='cpu',weights_only=True);model=Verifier(old,saved['config']['features']);model.load_state_dict(saved['state'],strict=False);p=probability(model,v);lookup={};start=0
  for row,b,ps in prepared:lookup[row['id']]=p[start:start+len(ps),1];start+=len(ps)
  rec,pred=choose(root,rows,parser,catalog,lambda row,b,ps:lookup[row['id']],name,[.3,.4,.5,.6,.7,.8]);rec.update(seconds=time.perf_counter()-before,same_input_tensors=hashes==meta['input_audit']['dev_tensor_sha256']);result[name]=rec;write(root/(name+'-current-predictions.json'),pred);m=rec['selection']['metrics'];print(name,m['relation_correct'],m['relations'],m['false_success'],rec['same_input_tensors'],flush=True)
 write(dest,dict(status='development decoding only; no final data, no fitting',results=result,current_tensor_hashes=hashes))
if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('root',type=Path);p.add_argument('old',type=Path);a=p.parse_args();run(a.root,a.old)
