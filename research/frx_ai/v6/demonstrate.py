"""Materialize real, known-source demonstrations outside Git, not new tests."""
import argparse,json,copy
from pathlib import Path
from bridge import read,write,jsonl
from investigate import Investigator

def make(root,old):
 dev=read(root/'development-gold-v2.jsonl');accept=read(root/'acceptance-gold.jsonl');examples=[]
 for identity,row,symptom in [
  ('suggested-not-executed',accept[3],'A request to share information is not a completed test.'),
  ('failed-trial',accept[27],'The game crashed again after a reported trial and later stopped starting.'),
  ('quoted-other-person',accept[1],'A quoted person reports freezes after opening the map.'),
  ('reported-joint-procedure',dev[20],'A reported audio procedure combines several configuration changes.'),
  ('dependency-symbol',dev[21],'Initialization reports a missing library symbol.'),
  ('unknown-cause',dev[36],'The game stopped working after an update, with insufficient cause evidence.'),
  ('known-failure-negation',accept[61],'No tested Proton version gets past the menu. The frozen extractor still makes an incorrect positive association here.')]:
  case=dict(id=identity,context=dict(app_id=row['app_id']),symptom=symptom,origin='known-public-source-demonstration; not independent evaluation',evidence=[dict(text=row['text'],url=row['url'],app_id=row['app_id'],document_id=row['document_id'],reporter_id=row['reporter_id'])])
  if identity=='dependency-symbol':case['log']='\n'.join(line for line in row['text'].splitlines() if 'undefined symbol:' in line)
  examples.append(case)
 # This is a historically known regression example, never a training/dev/final
 # selection case for v6. No synthetic variant is derived for model fitting.
 old5=Path(str(old).replace('v2-20261029','v5-20261030'))
 if (old5/'examples.jsonl').exists():
  for row in read(old5/'examples.jsonl'):
   if row.get('id')=='reported-regression':examples.append(dict(row,origin='historical-known-v5-demonstration; not v6 evaluation or training'))
 # Keep source-free, explicit synthetic contracts separate from public reports.
 if (root/'replay-inputs.jsonl').exists():
  for snap in read(root/'replay-inputs.jsonl'):
   if snap['id'] in ['failed-attempt','hardware-analogy','missing-library']:examples.append(dict(snap['input'],id=snap['id']))
 jsonl(root/'examples.jsonl',examples);engine=Investigator(root,old)
 results=[]
 for case in examples:
  out=engine.analyze(case);results.append(dict(id=case['id'],result=out))
 write(root/'example-results.json',results)
 print('Prepared',len(examples),'known demonstrations; raw source and output remain outside Git')
if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('root',type=Path);p.add_argument('old',type=Path);a=p.parse_args();make(a.root,a.old)
