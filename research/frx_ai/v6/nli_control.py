"""Existing 92MB local NLI ONNX control on action/result claims; no training."""
import argparse,json,time
from pathlib import Path
import numpy as np
from bridge import read,write,sha,CompactSyntax,immutable
from grounding import deconstruct
from pairs import candidates
from learning import choose
def run(root,directory):
 immutable(root);dest=root/'nli-development.json'
 if dest.exists():raise ValueError('preserve NLI control')
 import onnxruntime as ort
 from tokenizers import Tokenizer
 opts=ort.SessionOptions();opts.intra_op_num_threads=4;opts.inter_op_num_threads=1;file=directory/'model_quint8_avx2.onnx';session=ort.InferenceSession(str(file),sess_options=opts,providers=['CPUExecutionProvider']);tok=Tokenizer.from_file(str(directory/'tokenizer.json'));tok.enable_truncation(max_length=256);tok.enable_padding();inputs={v.name for v in session.get_inputs()};rows=read(root/'development-gold-v2.jsonl');catalog=json.loads((root/'catalog.json').read_text());parser=CompactSyntax(root);lookup={};start=time.perf_counter()
 for row in rows:
  b=deconstruct(row,parser,catalog);ps=candidates(row['text'],b);texts=[]
  for p in ps:
   obs=b['observations'][p['observation']];es=[b['events'][i] for i in p['event_indices']];actions=' and '.join(e['trigger']['text']+' '+ ' '.join(a['text'] for a in e.get('arguments',[])) for e in es)[:260];claim='The report states that '+actions+' was performed before the observed result: '+obs['evidence']['text'][:180]
   texts.append((row['text'][:1800],claim))
  probs=[]
  for i in range(0,len(texts),8):
   enc=tok.encode_batch(texts[i:i+8]);feed={k:np.array([getattr(e,attr) for e in enc],dtype=np.int64) for k,attr in [('input_ids','ids'),('attention_mask','attention_mask'),('token_type_ids','type_ids')] if k in inputs};z=session.run(None,feed)[0];z-=z.max(1,keepdims=True);p=np.exp(z);probs.extend((p/p.sum(1,keepdims=True))[:,1].tolist())
  lookup[row['id']]=probs
 result,pred=choose(root,rows,parser,catalog,lambda row,b,ps:lookup[row['id']],'Z-existing-NLI',[.3,.4,.5,.6,.7,.8]);result.update(seconds=time.perf_counter()-start,model_sha256=sha(file),trained=False,source='preserved v1 local NLI control; not a new downloaded model');write(dest,result);write(root/'nli-predictions.json',pred);print(result['selection']['metrics']['relation_correct'],result['selection']['metrics']['relations'])
if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('root',type=Path);p.add_argument('model',type=Path);a=p.parse_args();run(a.root,a.model)
