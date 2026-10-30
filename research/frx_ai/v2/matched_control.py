# POST-EVALUATION AUDIT ONLY: no training, candidate change, or threshold change.
import sys,pathlib,json,hashlib,shutil
sys.path.insert(0,str(pathlib.Path(__file__).resolve().parent))
import torch,numpy as np
from transformers import AutoModel,AutoTokenizer
from onnxruntime.quantization import quantize_dynamic,QuantType
from retrieval_export import Wrapper
from retrieval_runtime import Retriever
from retrieval import measure,QUERIES
import argparse
parser=argparse.ArgumentParser();parser.add_argument('root',type=pathlib.Path);root=parser.parse_args().root
if (root/'matched-quantization-control.json').exists():raise ValueError('preserve first matched-control audit')
dest=root/'runtime-frozen-control';dest.mkdir(exist_ok=True);torch.set_num_threads(4)
model=AutoModel.from_pretrained(root/'pretrained',local_files_only=True,attn_implementation='eager');model.eval();tok=AutoTokenizer.from_pretrained(root/'pretrained',local_files_only=True);tokens=tok(['The game does not launch.','Missing legacy runtime.'],padding=True,return_tensors='pt')
f=dest/'frozen-float.onnx';q=dest/'frozen-int8.onnx'
torch.onnx.export(Wrapper(model),tuple(tokens[k] for k in ['input_ids','attention_mask','token_type_ids']),str(f),input_names=['input_ids','attention_mask','token_type_ids'],output_names=['embedding'],dynamic_axes={k:{0:'batch',1:'sequence'} for k in tokens}|{'embedding':{0:'batch'}},opset_version=17,dynamo=False)
quantize_dynamic(str(f),str(q),weight_type=QuantType.QInt8,per_channel=True,reduce_range=True,op_types_to_quantize=['MatMul','Gemm','Gather']);shutil.copyfile(root/'pretrained/tokenizer.json',dest/'tokenizer.json')
meta=dict(model_name='frozen-MiniLM-matched-quantization-control',model_file=q.name,sha256=hashlib.sha256(q.read_bytes()).hexdigest(),tokenizer_sha256=hashlib.sha256((dest/'tokenizer.json').read_bytes()).hexdigest(),alpha=1.)
(dest/'runtime.json').write_text(json.dumps(meta));runtime=Retriever(dest);rows=[json.loads(x) for x in (root/'final-events.jsonl').read_text().splitlines()]
result=measure(rows,runtime.encode([r['text'] for r in rows]),runtime.encode(list(QUERIES.values())),1.)
full=json.loads((root/'retrieval-final-v2.json').read_text());before={(d['app_id'],d['topic']):d for d in result['details']};native=full['native_runtime'];apps=sorted({d['app_id'] for d in native['details']});rng=np.random.default_rng(919);metrics=['p3','ndcg5','mrr'];diff=[]
for _ in range(4000):
 chosen=rng.choice(apps,len(apps),replace=True);sample=[d for a in chosen for d in native['details'] if d['app_id']==a]
 diff.append([np.mean([d['semantic'][m]-before[(d['app_id'],d['topic'])]['semantic'][m] for d in sample]) for m in metrics])
audit=dict(scope='Post-evaluation matched-quantization baseline audit; not used for selection; candidate unchanged',baseline=result,candidate_summary=native['summary'],gain_intervals={m:np.quantile(np.array(diff)[:,i],[.025,.975]).tolist() for i,m in enumerate(metrics)})
(root/'matched-quantization-control.json').write_text(json.dumps(audit,indent=2));print(json.dumps({'baseline':result['summary'],'candidate':native['summary'],'CI':audit['gain_intervals']},indent=2))
