"""Local NLI evidence adapter: fixed hypotheses, no text generation or execution."""
import json,pathlib,numpy as np
from core import clean,state_of,ENV
from factor import focus
HYPOTHESES=[
'The author only suggests or asks about the change, without reporting its result.',
'The author reports that {action} fixed or improved the game problem.',
'The author reports that {action} did not solve the game problem.',
'The author reports that {action} caused new problems or made the game worse.',
'The author reports that {action} helped with one problem but another problem remained.',
]
STATE={'enable':'enabling the option','disable':'disabling the option','install':'installing the component','remove':'removing the component','update':'updating the software','replace':'replacing the file','switch':'switching the software version','configure':'changing the configuration','unknown':'making the change'}
class NLI:
 def __init__(self,path):
  import onnxruntime as ort
  from tokenizers import Tokenizer
  path=pathlib.Path(path);opt=ort.SessionOptions();opt.intra_op_num_threads=4;opt.inter_op_num_threads=1
  self.session=ort.InferenceSession(str(path/'model_quint8_avx2.onnx'),sess_options=opt,providers=['CPUExecutionProvider']);self.tokenizer=Tokenizer.from_file(str(path/'tokenizer.json'));self.tokenizer.enable_truncation(max_length=256,strategy='only_first');self.tokenizer.enable_padding();self.names={x.name for x in self.session.get_inputs()}
 def features(self,rows):
  pairs=[]
  for row in rows:
   premise=focus(row) or clean(row['text'])[:1800]
   premise=ENV.sub(lambda m:'the launch option' if m.group()==row['target'] else 'another launch option',premise)
   for hypothesis in HYPOTHESES:pairs.append((premise,hypothesis.format(action=STATE[state_of(row)])))
  outputs=[]
  for i in range(0,len(pairs),8):
   batch=self.tokenizer.encode_batch(pairs[i:i+8]);ids=np.array([x.ids for x in batch],dtype=np.int64);feed={'input_ids':ids,'attention_mask':np.array([x.attention_mask for x in batch],dtype=np.int64),'token_type_ids':np.array([x.type_ids for x in batch],dtype=np.int64)}
   logits=self.session.run(None,{k:v for k,v in feed.items() if k in self.names})[0];logits-=logits.max(1,keepdims=True);exp=np.exp(logits);outputs.append(exp/exp.sum(1,keepdims=True))
  return np.concatenate(outputs).reshape(len(rows),15)
