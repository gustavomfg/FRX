"""Fresh-process CPU measurements, with Torch deliberately unavailable."""
import argparse,json,sys,time,resource,importlib.abc
from pathlib import Path

class BlockTorch(importlib.abc.MetaPathFinder):
 def find_spec(self,fullname,path=None,target=None):
  if fullname=='torch' or fullname.startswith('torch.'):raise ImportError('Torch deliberately unavailable for primary v6 inference')

def run(root,old,baseline=False):
 start=time.perf_counter()
 if not baseline:sys.meta_path.insert(0,BlockTorch())
 from bridge import read,write,sha
 from investigate import Investigator
 import numpy as np
 if baseline:
  import shutil
  from lab import Intelligence
  original=Path(str(old).replace('v2-20261029','v5-20261030'));area=root/'benchmark-v5';area.mkdir(exist_ok=True)
  for name in ['selection.json','syntax-manifest.json']:shutil.copyfile(original/name,area/name)
  if not (area/'syntax-models').exists():(area/'syntax-models').symlink_to((original/'syntax-models').resolve(),target_is_directory=True)
  engine=Intelligence(area,old,check_seal=False)
 else:engine=Investigator(root,old)
 startup=time.perf_counter()-start
 rows=read(root/'development-gold-v2.jsonl')
 selected=[rows[i] for i in [0,8,16,21,24,27,32,36,40,43,44,47]]
 # Clear only the new benchmark instance's in-memory parse cache: never delete
 # historical cache files or alter prior laboratory artifacts.
 parser=engine.parser if baseline else engine.interpreter.parser
 print('PARSER',type(parser).__name__,file=sys.stderr)
 def request(row,index):
  text=row['text'];case=dict(context=dict(app_id=row['app_id']),symptom='Investigate the reported initialization or stability problem.',origin='known-public-source-benchmark',evidence=[dict(text=text,url=row['url'],app_id=row['app_id'],document_id=row['document_id'],reporter_id=row['reporter_id'])])
  # No prior parse cache is read for the measured requests. Existing cache
  # files remain untouched; repeats create their own external measurement run.
  import tempfile
  parser.cache=Path(tempfile.mkdtemp(prefix='benchmark-parse-',dir=root))
  t=time.perf_counter();result=engine.analyze(case);return 1000*(time.perf_counter()-t),result
 values=[]
 for i,row in enumerate(selected):
  elapsed,out=request(row,i);values.append(elapsed)
  if out.get('execution_authorized',False):raise AssertionError('unexpected execution approval')
 compact=json.loads((root/'compact-manifest.json').read_text());weights=Path(compact['directory']);weight_bytes=sum(p.stat().st_size for p in weights.rglob('*') if p.is_file());ir=old/'runtime-retrieval'
 runtime=json.loads((ir/'runtime.json').read_text());ir_bytes=sum((ir/name).stat().st_size for name in [runtime['model_file'],'tokenizer.json','runtime.json'])
 record=dict(cpu_only=True,torch_loaded=any(x=='torch' or x.startswith('torch.') for x in sys.modules),startup_seconds=startup,first_request_ms=values[0],warm_uncached_median_ms=float(np.median(values[1:])),warm_uncached_p95_ms=float(np.quantile(values[1:],.95)),requests=len(values),request_ms=values,max_rss_mib=resource.getrusage(resource.RUSAGE_SELF).ru_maxrss/1024,compact_model_bytes=weight_bytes,retrieval_runtime_bytes=ir_bytes,retrieval_archival_directory_bytes=sum(p.stat().st_size for p in ir.glob('*') if p.is_file()),classifier_bytes=(root/'verifiers/L1-structured-real-text.json').stat().st_size,selection_sha256=sha(root/'selection.json'),automatic_approval=False,limits='Twelve known development requests, not a performance distribution guarantee. Python/dependencies excluded from weight size. Archival float ONNX is preserved but not loaded.')
 record['pipeline']='preserved-v5-stanza' if baseline else 'frozen-v6-selected'
 if baseline:
  record['syntax_bytes']=json.loads((root/'syntax-manifest.json').read_text())['total_bytes'];record.pop('compact_model_bytes');record['classifier_bytes']=0;record['selection_sha256']=sha(area/'selection.json')
 elif record['torch_loaded']:raise AssertionError('primary loaded Torch')
 write(root/('efficiency-v5-paired.json' if baseline else 'efficiency.json'),record);print(json.dumps(record,indent=2))
if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('root',type=Path);p.add_argument('old',type=Path);p.add_argument('--baseline',action='store_true');a=p.parse_args();run(a.root,a.old,a.baseline)
