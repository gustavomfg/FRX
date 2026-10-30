"""Explicit research conversion. Compare float and int8 on DEV before freezing."""
import argparse
import hashlib
import json
import pathlib
import shutil
import numpy as np
import torch
from torch import nn
from onnxruntime.quantization import quantize_dynamic,QuantType
from transformers import AutoTokenizer
from model import RelationModel,tokenize,predict
from train import read
from runtime import Runtime
from metrics import evaluate


class ExportWrapper(nn.Module):
    def __init__(self,model):super().__init__();self.model=model
    def forward(self,input_ids,attention_mask,token_type_ids):
        heads,_=self.model(input_ids=input_ids,attention_mask=attention_mask,token_type_ids=token_type_ids)
        return tuple(heads)


def main(root,name,goal=False):
    torch.set_num_threads(4)
    checkpoint=torch.load(root/'runs'/(name+'.pt'),map_location='cpu',weights_only=True)
    mode=checkpoint['config'].get('mode','marked')
    if mode=='oracle':raise ValueError('gold evidence spans cannot be deployed')
    dest=root/('runtime-goal' if goal else 'runtime-relation');dest.mkdir(exist_ok=True)
    model=RelationModel(root/'pretrained',eager=True);model.load_state_dict(checkpoint['state']);model.eval()
    tokenizer=AutoTokenizer.from_pretrained(root/'pretrained',local_files_only=True)
    dev=read(root/('dev-goals.jsonl' if goal else 'dev-events.jsonl'));tokens=tokenize(dev,tokenizer,mode)
    sample={k:v[:2] for k,v in tokens.items()}
    float_file=dest/'relation-float.onnx'
    torch.onnx.export(ExportWrapper(model),tuple(sample[k] for k in ['input_ids','attention_mask','token_type_ids']),
        str(float_file),input_names=['input_ids','attention_mask','token_type_ids'],
        output_names=['outcome','direction','modality','grouping'],opset_version=17,dynamo=False,
        dynamic_axes={k:{0:'batch',1:'sequence'} for k in sample}|{k:{0:'batch'} for k in ['outcome','direction','modality','grouping']})
    quant_file=dest/'relation-int8.onnx'
    quantize_dynamic(str(float_file),str(quant_file),weight_type=QuantType.QInt8,
        per_channel=True,reduce_range=True,op_types_to_quantize=['MatMul','Gemm','Gather'])
    shutil.copyfile(root/'pretrained/tokenizer.json',dest/'tokenizer.json')
    results={}
    for file in [float_file,quant_file]:
        meta=dict(model_name=name,mode=mode,task='goal-pair' if goal else 'comment-action',model_file=file.name,sha256=hashlib.sha256(file.read_bytes()).hexdigest(),
            tokenizer_sha256=hashlib.sha256((dest/'tokenizer.json').read_bytes()).hexdigest(),
            temperatures=[1.]*4,threshold=.7,automatic_interpretation_approved=False)
        (dest/'runtime.json').write_text(json.dumps(meta,indent=2));runtime=Runtime(dest)
        # Tokenizer parity is an integration check, not another accuracy test.
        actual=runtime.tokens(dev)
        assert all(np.array_equal(actual[k],v.numpy()) for k,v in tokens.items())
        logits,_=runtime.classify(dev)
        np.savez(dest/(file.stem+'.npz'),**{f'head{i}':x for i,x in enumerate(logits)})
        results[file.name]=dict(bytes=file.stat().st_size,dev=evaluate(dev,logits,.5))
    float_m=results[float_file.name]['dev'];quant_m=results[quant_file.name]['dev']
    chosen=quant_file if quant_m['exact_all_accuracy']>=float_m['exact_all_accuracy']-.025 and quant_m['relation_macro_f1']>=float_m['relation_macro_f1']-.025 else float_file
    meta['model_file']=chosen.name;meta['sha256']=hashlib.sha256(chosen.read_bytes()).hexdigest()
    (dest/'runtime.json').write_text(json.dumps(meta,indent=2))
    (root/('export-goal-comparison.json' if goal else 'export-comparison.json')).write_text(json.dumps(dict(results=results,chosen=chosen.name,
        parameters=sum(p.numel() for p in model.parameters())),indent=2))
    print('chosen',chosen.name,chosen.stat().st_size)


if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('root',type=pathlib.Path);p.add_argument('name');p.add_argument('--goal',action='store_true')
    a=p.parse_args();main(a.root,a.name,a.goal)
