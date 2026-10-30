"""Explicit setup of the official MIT small English pipeline, hash pinned."""
import argparse,hashlib,subprocess,sys,urllib.request
from pathlib import Path
from common import write

URL='https://github.com/explosion/spacy-models/releases/download/en_core_web_sm-3.8.0/en_core_web_sm-3.8.0-py3-none-any.whl'
SHA256='1932429db727d4bff3deed6b34cfc05df17794f4a52eeb26cf8928f7c1a0fb85'

def run(root):
 if (root/'frozen.json').exists():raise ValueError('preserve sealed models')
 root.mkdir(parents=True,exist_ok=True)
 file=root/'en_core_web_sm-3.8.0-py3-none-any.whl'
 if not file.exists():
  with urllib.request.urlopen(URL,timeout=40) as response:raw=response.read(16000000)
  if len(raw)>=16000000 or hashlib.sha256(raw).hexdigest()!=SHA256:raise ValueError('official pipeline checksum/size mismatch')
  file.write_bytes(raw)
 if hashlib.sha256(file.read_bytes()).hexdigest()!=SHA256:raise ValueError('cached pipeline changed')
 subprocess.run([sys.executable,'-m','pip','install','--no-deps',str(file)],check=True)
 import en_core_web_sm,spacy
 directory=Path(en_core_web_sm.__file__).parent/'en_core_web_sm-3.8.0'
 manifest=dict(repository='explosion/spacy-models',release='en_core_web_sm-3.8.0',url=URL,published_sha256=SHA256,license='MIT',bytes=file.stat().st_size,spacy_version=spacy.__version__,directory=str(directory),files={str(p.relative_to(directory)):hashlib.sha256(p.read_bytes()).hexdigest() for p in directory.rglob('*') if p.is_file()})
 write(root/'compact-manifest.json',manifest);print('Verified official small parser; model size',file.stat().st_size)

if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('root',type=Path);run(p.parse_args().root)
