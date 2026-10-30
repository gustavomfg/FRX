"""Research-only helpers; no application imports or operations."""
import os
os.environ.setdefault('OPENBLAS_NUM_THREADS','4')
os.environ.setdefault('OMP_NUM_THREADS','4')
import hashlib
import importlib.util
import json
from pathlib import Path
import re
import numpy as np

HERE=Path(__file__).resolve().parent
V2=HERE.parent/'v2'
TOPICS=['startup','crash','performance','graphics','controls','dependency','audio','network']
QUERIES={
 'startup':'The game closes when loading and never reaches the main menu.',
 'crash':'The application freezes or exits unexpectedly during gameplay.',
 'performance':'Uneven frame pacing, low speed and pauses while playing.',
 'graphics':'The picture has flashing surfaces, broken shapes or absent textures.',
 'controls':'Buttons, mouse movement or the gamepad do not respond correctly.',
 'dependency':'A necessary software library is missing or fails to install.',
 'audio':'Sound is absent, distorted or crackles during playback.',
 'network':'Online play cannot connect or multiplayer is unavailable.'}
TRAIN_QUERIES={
 'startup':'The game does not launch or fails to start.',
 'crash':'Crashing and freezing during the game.',
 'performance':'Low FPS and stuttering in the game.',
 'graphics':'Visual artifacts or missing geometry and textures.',
 'controls':'Mouse keyboard or controller input issues.',
 'dependency':'Missing libraries and failed component installation.',
 'audio':'Crackling or missing audio and sound.',
 'network':'Problems with connecting to multiplayer online.'}

def load_module(name):
 spec=importlib.util.spec_from_file_location('frx_v2_'+name,V2/(name+'.py'))
 m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m);return m

Retriever=load_module('retrieval_runtime').Retriever

def read(path):return [json.loads(s) for s in Path(path).read_text().splitlines() if s.strip()]
def write(path,value):Path(path).write_text(json.dumps(value,indent=2,ensure_ascii=False))
def jsonl(path,rows):Path(path).write_text(''.join(json.dumps(r,ensure_ascii=False)+'\n' for r in rows))
def sha(path):return hashlib.sha256(Path(path).read_bytes()).hexdigest()
def words(s):return re.findall(r'[a-z0-9_]+',s.lower())

def units(text):
 """Literal authored sentence/line spans; dots inside paths are preserved."""
 output=[]
 for line in re.finditer(r'[^\n]+',text):
  if line.group().lstrip().startswith(('>','<!--')):continue
  a=line.start()
  for stop in re.finditer(r'(?<=[.!?;])\s+(?=[A-Z])',line.group()):
   b=line.start()+stop.start()
   if b-a>4:output.append([a,b])
   a=line.start()+stop.end()
  if line.end()-a>4:output.append([a,line.end()])
 return output

def near(text,span,width=100):
 a,b=span;return text[max(0,a-width):a]+' [ACTION] '+text[a:b]+' [/ACTION] '+text[b:min(len(text),b+width)]

def cached_encode(root,key,encoder,texts):
 digest=hashlib.sha256(json.dumps(dict(model=encoder.meta['sha256'],texts=texts),ensure_ascii=False).encode()).hexdigest()
 dest=Path(root)/'features';dest.mkdir(exist_ok=True)
 p=dest/(key+'-'+digest[:16]+'.npy')
 if not p.exists():np.save(p,encoder.encode(texts))
 return np.load(p)

def preserve_fit(root):
 if (Path(root)/'frozen.json').exists():raise ValueError('sealed experiment; no fitting or selection')
