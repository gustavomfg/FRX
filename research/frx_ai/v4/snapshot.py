"""Publish small offsets/metrics; keep all original source bodies and weights external."""
import argparse,hashlib,json
from pathlib import Path
from data import read,write

HERE=Path(__file__).resolve().parent

def run(root):
 out=HERE/'snapshot';out.mkdir(exist_ok=True);bodies=root/'bodies';bodies.mkdir(exist_ok=True);datasets={}
 for split in ['train','dev','final']:
  file=root/(split+'-audited.jsonl');rows=read(file);annotations=[]
  for r in rows:
   body=r['source_text'];digest=hashlib.sha256(body.encode()).hexdigest();assert digest==r['source_sha256']
   cache=bodies/(digest+'.txt')
   if cache.exists():assert cache.read_bytes()==body.encode()
   else:cache.write_bytes(body.encode())
   annotations.append(dict(row_keys=list(r),data={k:v for k,v in r.items() if k not in ['text','source_text']},paragraph_length=len(r['text']),paragraph_sha256=hashlib.sha256(r['text'].encode()).hexdigest()))
  write(out/(split+'-annotations.json'),annotations)
  datasets[split]=dict(paragraphs=len(rows),events=sum(len(r['events']) for r in rows),games=sorted({r['app_id'] for r in rows}),file_sha256=hashlib.sha256(file.read_bytes()).hexdigest())
 write(out/'datasets.json',dict(label_quality='provisional research-agent annotations; no independent human review',datasets=datasets,body_redistribution=False,offsets='Unicode code points; preserve CRLF'))
 for name in ['selection','development','export-comparison','final-results','posthoc-review','frozen','reservation','benchmark','diagnostic-results','structural-audit','final-annotation-provenance','human-review-queue','parity','retrieval-replay','integration','counterfactuals']:
  write(out/(name+'.json'),json.loads((root/(name+'.json')).read_text()))
 experiments=[]
 for file in sorted((root/'runs').glob('*.json')):
  result=json.loads(file.read_text());experiments.append(dict(name=result['name'],config=result['config'],real_training_paragraphs=result['real_training_paragraphs'],synthetic_training=result['synthetic_training'],seconds=result['seconds'],epochs=[{k:v for k,v in e.items() if k!='raw' and k!='verified'} for e in result['epochs']],early_selection_score=result['selection_score'],early_metric='exploratory any-overlap, not definitive strict extraction quality'))
 write(out/'experiments.json',dict(trained_configurations=len(experiments),development_readouts=34,configs=experiments,post_freeze_files=['benchmark.py','review.py','run.py','test_intake.py','snapshot.py','hydrate.py','reproduce.py','make_demo.py','parity.py','retrieval_replay.py','integration.py','counterfactuals.py'],notes=['Only six fitted configurations; decoding readouts are not additional fits.','Preserve first independent event result; later tools do not tune its inference.']))
 print('Saved source-free snapshots; original public bodies and all checkpoints remain external')

if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('root',type=Path);run(p.parse_args().root)
