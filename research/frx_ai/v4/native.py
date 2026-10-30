"""Local ONNX event proposals plus literal verification; no training imports."""
from pathlib import Path
import json,hashlib
import numpy as np
from decode import decode

class NativeEvents:
 def __init__(self,directory):
  import onnxruntime as ort
  from tokenizers import Tokenizer
  directory=Path(directory);self.meta=json.loads((directory/'runtime.json').read_text());file=directory/self.meta['model_file'];tokenizer=directory/'tokenizer.json'
  if file.parent.resolve()!=directory.resolve() or file.stat().st_size>140000000:raise ValueError('event model path/size bound')
  if hashlib.sha256(file.read_bytes()).hexdigest()!=self.meta['sha256'] or hashlib.sha256(tokenizer.read_bytes()).hexdigest()!=self.meta['tokenizer_sha256']:raise ValueError('event artifact changed')
  options=ort.SessionOptions();options.intra_op_num_threads=4;options.inter_op_num_threads=1
  self.session=ort.InferenceSession(str(file),sess_options=options,providers=['CPUExecutionProvider'])
  self.tokenizer=Tokenizer.from_file(str(tokenizer));self.tokenizer.enable_truncation(max_length=256);self.tokenizer.enable_padding(length=256,pad_id=0,pad_token='[PAD]')
 def predict(self,rows,verified=True,head_projection=None):
  out=[]
  if head_projection is None:head_projection=self.meta.get('head_projection',False)
  for start in range(0,len(rows),8):
   batch=rows[start:start+8]
   if any(not isinstance(r.get('text'),str) or len(r['text'])>16000 for r in batch):raise ValueError('event input bound')
   enc=self.tokenizer.encode_batch([r['text'] for r in batch]);feed={k:np.array([getattr(e,attr) for e in enc],dtype=np.int64) for k,attr in [('input_ids','ids'),('attention_mask','attention_mask'),('token_type_ids','type_ids')]}
   logits=self.session.run(None,feed);prob=[]
   for z in logits:
    p=np.exp(z-z.max(-1,keepdims=True));prob.append(p/p.sum(-1,keepdims=True))
   for i,row in enumerate(batch):
    item=decode(row,enc[i].offsets,[p[i] for p in prob],verified,self.meta.get('threshold',.6),True,head_projection)
    item['truncated']=bool(enc[i].overflowing);out.append(item)
  return out
