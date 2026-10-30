"""Rebuild one real public diagnostic demonstration without executing its text."""
import argparse
from pathlib import Path
import json
import hashlib
import subprocess
import re

def main():
 p=argparse.ArgumentParser();p.add_argument('root',type=Path);p.add_argument('--fetch',action='store_true');a=p.parse_args();root=a.root
 examples=json.loads((Path(__file__).parent/'snapshot/diagnostic-study.json').read_text())['historical_demonstrations']
 item=next(x for x in examples if x['predicates']==['reported-library-not-found'] and x['components']==['AVRT.dll'])
 cache=root/'bodies';cache.mkdir(parents=True,exist_ok=True);file=cache/(item['source_sha256']+'.txt')
 if not file.exists():
  if not a.fetch:raise FileNotFoundError('explicit --fetch or original cache required')
  identity=re.fullmatch(r'https://github.com/ValveSoftware/Proton/issues/\d+#issuecomment-(\d+)',item['url'])
  if not identity:raise ValueError('source identity')
  raw=subprocess.check_output(['gh','api','repos/ValveSoftware/Proton/issues/comments/'+identity[1]],timeout=45)
  if len(raw)>2000000:raise ValueError('source response bound')
  text=json.loads(raw)['body']
  if hashlib.sha256(text.encode()).hexdigest()!=item['source_sha256']:raise ValueError('source changed; original cache required')
  file.write_bytes(text.encode())
 text=file.read_bytes().decode()
 if hashlib.sha256(text.encode()).hexdigest()!=item['source_sha256']:raise ValueError('source hash')
 case=dict(context={'app_id':item['app_id']},symptom='The game cannot start and the loader reports a missing module.',
           log=text,evidence=[dict(app_id=item['app_id'],url=item['url'],text=text)],origin='public-reported-log; not-local-telemetry')
 (root/'demo-real.jsonl').write_text(json.dumps(case)+'\n');print('Prepared one historical public demonstration; no game or binary executed')

if __name__=='__main__':main()
