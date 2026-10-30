"""Export the selected compact retrieval encoder for local ONNX inference."""
import argparse
import hashlib
import json
import pathlib
import shutil
import torch
import numpy as np
from torch import nn
from transformers import AutoModel,AutoTokenizer
from onnxruntime.quantization import quantize_dynamic,QuantType
from retrieval_train import pool
from retrieval import measure,QUERIES
from retrieval_train import encode
from retrieval_runtime import Retriever
from train import read


class Wrapper(nn.Module):
    def __init__(self,model):super().__init__();self.model=model
    def forward(self,input_ids,attention_mask,token_type_ids):
        return pool(self.model,dict(input_ids=input_ids,attention_mask=attention_mask,token_type_ids=token_type_ids))


def main(root):
    selection=json.loads((root/'retrieval-selection.json').read_text());name=selection['model_name']
    torch.set_num_threads(4)
    model=AutoModel.from_pretrained(root/'pretrained',local_files_only=True,attn_implementation='eager')
    if name!='frozen-MiniLM':
        state=torch.load(root/'retriever-runs'/(name+'.pt'),map_location='cpu',weights_only=True);model.load_state_dict(state['state'])
    model.eval();dest=root/'runtime-retrieval';dest.mkdir(exist_ok=True)
    tokenizer=AutoTokenizer.from_pretrained(root/'pretrained',local_files_only=True)
    tokens=tokenizer(['The game does not launch.','Missing legacy runtime.'],padding=True,return_tensors='pt')
    path=dest/'retriever-float.onnx'
    torch.onnx.export(Wrapper(model),tuple(tokens[k] for k in ['input_ids','attention_mask','token_type_ids']),str(path),
        input_names=['input_ids','attention_mask','token_type_ids'],output_names=['embedding'],
        dynamic_axes={k:{0:'batch',1:'sequence'} for k in tokens}|{'embedding':{0:'batch'}},opset_version=17,dynamo=False)
    quant=dest/'retriever-int8.onnx'
    quantize_dynamic(str(path),str(quant),weight_type=QuantType.QInt8,per_channel=True,reduce_range=True,op_types_to_quantize=['MatMul','Gemm','Gather'])
    shutil.copyfile(root/'pretrained/tokenizer.json',dest/'tokenizer.json')
    meta=dict(model_name=name,model_file=quant.name,sha256=hashlib.sha256(quant.read_bytes()).hexdigest(),
        tokenizer_sha256=hashlib.sha256((dest/'tokenizer.json').read_bytes()).hexdigest(),alpha=selection['selected_alpha'],
        similarity_is_causal_evidence=False)
    dev=read(root/'retrieval-dev.jsonl');comparison={}
    for file in [path,quant]:
        meta['model_file']=file.name;meta['sha256']=hashlib.sha256(file.read_bytes()).hexdigest()
        (dest/'runtime.json').write_text(json.dumps(meta,indent=2));runtime=Retriever(dest)
        result=measure(dev,runtime.encode([r['text'] for r in dev]),runtime.encode(list(QUERIES.values())),meta['alpha'])
        comparison[file.name]=dict(bytes=file.stat().st_size,metrics=result)
    f=comparison[path.name]['metrics']['summary']['hybrid']['ndcg5'];q=comparison[quant.name]['metrics']['summary']['hybrid']['ndcg5']
    chosen=quant if q>=f-.025 else path
    meta['model_file']=chosen.name;meta['sha256']=hashlib.sha256(chosen.read_bytes()).hexdigest()
    (dest/'runtime.json').write_text(json.dumps(meta,indent=2))
    (root/'retrieval-export-comparison.json').write_text(json.dumps(dict(chosen=chosen.name,comparison=comparison),indent=2))
    print('retriever export',chosen.name,chosen.stat().st_size)


if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('root',type=pathlib.Path);main(p.parse_args().root)
