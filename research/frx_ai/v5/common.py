"""Small research-only IO, literal evidence and read-only prior-lab adapters."""
from pathlib import Path
import hashlib,importlib.util,json,sys

HERE=Path(__file__).resolve().parent
V4=HERE.parent/'v4'
def read(p):return [json.loads(x) for x in Path(p).read_text().splitlines() if x.strip()]
def write(p,obj):Path(p).write_text(json.dumps(obj,indent=2,ensure_ascii=False))
def jsonl(p,rows):Path(p).write_text(''.join(json.dumps(x,ensure_ascii=False)+'\n' for x in rows))
def digest(text):return hashlib.sha256(text.encode()).hexdigest()
def evidence(text,a,b):
 if not (type(a) is int and type(b) is int and 0<=a<b<=len(text)):raise ValueError('literal span bound')
 return dict(span=[a,b],text=text[a:b],text_sha256=digest(text),offset_encoding='unicode-codepoints')
def prior4(name):
 # Older scripts use sibling imports. Keep their directories after the v5 one.
 path=str(V4)
 if path not in sys.path:sys.path.append(path)
 spec=importlib.util.spec_from_file_location('v4_'+name,V4/(name+'.py'));m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m);return m

def development(root,old4):
 # Prior final data is now known development, never claimed independent again.
 train=read(old4/'train-audited.jsonl')+read(old4/'dev-audited.jsonl');dev=read(old4/'final-audited.jsonl')
 assert not {r['app_id'] for r in train}&{r['app_id'] for r in dev}
 jsonl(root/'train-known.jsonl',train);jsonl(root/'dev-known.jsonl',dev)
 return train,dev
