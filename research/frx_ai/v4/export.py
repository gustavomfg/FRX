"""Select native float/int8 on development only, before final source access."""
import argparse
from pathlib import Path
import json,hashlib,shutil
import torch
from torch import nn
from transformers import AutoTokenizer
from onnxruntime.quantization import quantize_dynamic,QuantType
from data import read,write
from model import EventModel
from native import NativeEvents
from metrics import evaluate

class Wrapper(nn.Module):
 def __init__(self,model):super().__init__();self.model=model
 def forward(self,input_ids,attention_mask,token_type_ids):return self.model(input_ids=input_ids,attention_mask=attention_mask,token_type_ids=token_type_ids)[:3]

def run(root,old):
 if (root/'frozen.json').exists():raise ValueError('sealed research')
 selection=json.loads((root/'selection.json').read_text());name=selection['neural_model']
 if not name:raise ValueError('no neural model selected; preserve symbolic selection')
 torch.set_num_threads(4);model=EventModel(old/'pretrained');checkpoint=root/'runs'/(name+'.pt');model.load_state_dict(torch.load(checkpoint,map_location='cpu',weights_only=True)['state']);model.eval()
 tok=AutoTokenizer.from_pretrained(old/'pretrained',local_files_only=True);batch=tok(['I installed the required library.','Try enabling the setting.'],padding=True,return_tensors='pt')
 out=root/'runtime-events';out.mkdir(exist_ok=True);floatfile=out/'events-float.onnx';quantfile=out/'events-int8.onnx'
 torch.onnx.export(Wrapper(model),tuple(batch[k] for k in ['input_ids','attention_mask','token_type_ids']),str(floatfile),input_names=['input_ids','attention_mask','token_type_ids'],output_names=['trigger','status','direction'],dynamic_axes={k:{0:'batch',1:'sequence'} for k in ['input_ids','attention_mask','token_type_ids','trigger','status','direction']},opset_version=17,dynamo=False)
 quantize_dynamic(str(floatfile),str(quantfile),weight_type=QuantType.QInt8,per_channel=True,reduce_range=True,op_types_to_quantize=['MatMul','Gemm','Gather']);shutil.copyfile(old/'pretrained/tokenizer.json',out/'tokenizer.json')
 meta=dict(model_name=name,tokenizer_sha256=hashlib.sha256((out/'tokenizer.json').read_bytes()).hexdigest(),source_checkpoint_sha256=hashlib.sha256(checkpoint.read_bytes()).hexdigest(),threshold=.6,head_projection=selection['pipeline'].endswith('verified-head'),automatic_approval=False)
 rows=read(root/'dev-audited.jsonl');comparison={}
 for file in [floatfile,quantfile]:
  m=meta|dict(model_file=file.name,sha256=hashlib.sha256(file.read_bytes()).hexdigest());write(out/'runtime.json',m);runtime=NativeEvents(out);pred=runtime.predict(rows);score=evaluate(rows,pred,strict=True);comparison[file.name]=dict(bytes=file.stat().st_size,metrics=score)
 a=comparison[floatfile.name]['metrics'];b=comparison[quantfile.name]['metrics'];chosen=quantfile if b['false_performed']<=a['false_performed'] and b['performed_correct']>=a['performed_correct'] and b['trigger_f1']>=a['trigger_f1']-.03 else floatfile
 write(out/'runtime.json',meta|dict(model_file=chosen.name,sha256=hashlib.sha256(chosen.read_bytes()).hexdigest()));write(root/'export-comparison.json',dict(chosen=chosen.name,comparison=comparison))
 print('Native selection:',chosen.name,chosen.stat().st_size)

if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('root',type=Path);p.add_argument('old',type=Path);a=p.parse_args();run(a.root,a.old)
