"""Build six known-source demonstrations from cached bodies or explicit API fetch.

No comments or payloads are executed. These known examples are demonstrations,
not the independent final set or the local machine's telemetry.
"""
import argparse,hashlib,json,re,subprocess
from pathlib import Path
from data import write,jsonl

HERE=Path(__file__).resolve().parent

def make(root,fetch=False,body_cache=None):
 if root.resolve().is_relative_to(HERE.parents[2]):raise ValueError('raw sources must remain outside the repository')
 cache=root/'bodies';cache.mkdir(parents=True,exist_ok=True);cases=[]
 for spec in json.loads((HERE/'snapshot/demos.json').read_text()):
  digest=spec['source_sha256'];file=cache/(digest+'.txt')
  if not file.exists() and body_cache and (body_cache/(digest+'.txt')).exists():file.write_bytes((body_cache/(digest+'.txt')).read_bytes())
  if not file.exists():
   if not fetch:raise FileNotFoundError('demo source cache required; explicit --fetch permitted')
   m=re.fullmatch(r'https://github.com/ValveSoftware/Proton/issues/\d+#issuecomment-(\d+)',spec['url'])
   if not m:raise ValueError('public source identity')
   raw=subprocess.check_output(['gh','api','repos/ValveSoftware/Proton/issues/comments/'+m[1]],timeout=45)
   if len(raw)>2000000:raise ValueError('public response bound')
   body=json.loads(raw)['body']
   if hashlib.sha256(body.encode()).hexdigest()!=digest:raise ValueError('changed demo source; original cache required')
   file.write_bytes(body.encode())
  body=file.read_bytes().decode()
  if hashlib.sha256(body.encode()).hexdigest()!=digest:raise ValueError('demo source integrity')
  text=body[spec['offset']:spec['offset']+spec['length']]
  if hashlib.sha256(text.encode()).hexdigest()!=spec['view_sha256']:raise ValueError('demo view integrity')
  case=dict(spec['case'])
  if spec['view_is_symptom']:case['symptom']=text
  else:case['evidence']=[dict(app_id=case['context']['app_id'],url=spec['url'],text=text)]
  if spec['view_is_log']:case['log']=text
  cases.append(case)
 target=root/'demos.jsonl';payload=''.join(json.dumps(r,ensure_ascii=False)+'\n' for r in cases)
 if target.exists() and [json.loads(l) for l in target.read_text().splitlines()]!=cases:raise ValueError('preserve different existing demonstration')
 target.write_text(payload);print('Prepared six historical/controlled-view demonstrations; no observed local incident or validated fix')

if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('root',type=Path);p.add_argument('--fetch',action='store_true');p.add_argument('--body-cache',type=Path);a=p.parse_args();make(a.root,a.fetch,a.body_cache)
