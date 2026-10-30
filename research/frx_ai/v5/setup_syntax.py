"""Explicit, bounded setup of official Stanford models; never run at inference."""
import argparse,hashlib,json,urllib.request
from pathlib import Path
from common import write

PROCESSORS={'tokenize':'combined_nocharlm','mwt':'combined','pos':'combined_nocharlm','lemma':'combined_nocharlm','depparse':'combined_nocharlm'}

def fetch(url,limit):
 with urllib.request.urlopen(url,timeout=45) as response:
  if response.geturl().split(':')[0]!='https':raise ValueError('TLS required')
  raw=response.read(limit+1)
 if len(raw)>limit:raise ValueError('download bound')
 return raw

def run(root):
 if (root/'frozen.json').exists():raise ValueError('preserve sealed models')
 root.mkdir(parents=True,exist_ok=True);resource_file=root/'stanza-resources.json'
 if not resource_file.exists():resource_file.write_bytes(fetch('https://raw.githubusercontent.com/stanfordnlp/stanza-resources/main/resources_1.14.0.json',2000000))
 resources=resource_file.read_bytes()
 if hashlib.sha256(resources).hexdigest()!='4e41c1df152146fa26ed0c006a08feea7a60bb3414bb6d57dbda24ad2e3cb99c':raise ValueError('resource metadata changed; pinned snapshot required')
 r=json.loads(resources);api=json.loads(fetch('https://huggingface.co/api/models/stanfordnlp/stanza-en/revision/88e0cd3ddaa0c5a4b682895527ac013ce0241c21?blobs=true',2000000));revision=api['sha'];files={x['rfilename']:x for x in api['siblings']};wanted=set(PROCESSORS.items())
 for proc,package in list(wanted):
  for dep in r['en'][proc][package].get('dependencies',[]):wanted.add((dep['model'],dep['package']))
 manifest=[]
 for proc,package in sorted(wanted):
  name=f'models/{proc}/{package}.pt';info=files[name];lfs=info.get('lfs') or {};expected=lfs.get('sha256') or lfs.get('oid');size=info.get('size',lfs.get('size'))
  if not expected or not size or size>250000000:raise ValueError('published integrity/size required')
  manifest.append(dict(processor=proc,package=package,path=f'en/{proc}/{package}.pt',upstream=name,bytes=size,sha256=expected,md5=r['en'][proc][package]['md5']))
 total=sum(x['bytes'] for x in manifest)
 if total>400*1024*1024:raise ValueError('aggregate model budget')
 print('Official Stanford pinned model bytes:',total,flush=True);dest=root/'syntax-models';dest.mkdir(exist_ok=True);(dest/'resources.json').write_bytes(resources)
 for item in manifest:
  file=dest/item['path'];file.parent.mkdir(parents=True,exist_ok=True)
  if file.exists():raw=file.read_bytes()
  else:
   raw=fetch(f'https://huggingface.co/stanfordnlp/stanza-en/resolve/{revision}/{item["upstream"]}',item['bytes']);file.write_bytes(raw)
  if len(raw)!=item['bytes'] or hashlib.sha256(raw).hexdigest()!=item['sha256'] or hashlib.md5(raw).hexdigest()!=item['md5']:raise ValueError('published model integrity mismatch')
  print(item['path'],len(raw),'verified',flush=True)
 write(root/'syntax-manifest.json',dict(repository='stanfordnlp/stanza-en',revision=revision,stanza_version='1.14.0',resources_sha256=hashlib.sha256(resources).hexdigest(),processors=PROCESSORS,files=manifest,total_bytes=total,license_note='Stanza code Apache-2.0; upstream model and UD training-data licenses require separate review before distribution. No model is bundled.'))

if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('root',type=Path);run(p.parse_args().root)
