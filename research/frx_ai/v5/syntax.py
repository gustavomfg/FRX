"""Cached pretrained syntax. This parser is upstream-trained, not FRX-fine-tuned."""
from pathlib import Path
import hashlib,json
from common import digest,write

class Syntax:
 def __init__(self,root):
  self.root=Path(root);self.cache=self.root/'syntax-cache';self.cache.mkdir(exist_ok=True);self.pipeline=None;self.meta=json.loads((self.root/'syntax-manifest.json').read_text())
  for item in self.meta['files']:
   if hashlib.sha256((self.root/'syntax-models'/item['path']).read_bytes()).hexdigest()!=item['sha256']:raise ValueError('syntax artifact changed')
 def parse(self,text):
  if not isinstance(text,str) or len(text)>16000:raise ValueError('bounded syntax input required')
  key=digest(text);file=self.cache/(key+'.json')
  if file.exists():
   cached=json.loads(file.read_text())
   if cached.get('source_sha256')!=key or cached.get('parser_revision')!=self.meta['revision']:raise ValueError('stale syntax cache')
   if any(text[slice(*w['span'])]!=w['text'] for w in cached['words']):raise ValueError('syntax cache literal mismatch')
   return cached
  if self.pipeline is None:
   import stanza,torch
   torch.set_num_threads(4);self.pipeline=stanza.Pipeline('en',dir=str(self.root/'syntax-models'),processors=self.meta['processors'],package=None,download_method=None,use_gpu=False,verbose=False)
  doc=self.pipeline(text);words=[];sentences=[]
  for si,sentence in enumerate(doc.sentences):
   base=len(words)
   for word in sentence.words:
    a,b=word.start_char,word.end_char
    if a is None or b is None:raise ValueError('parser lacks literal offsets')
    words.append(dict(i=base+word.id-1,text=text[a:b],lemma=word.lemma or word.text.lower(),pos=word.upos,tag=word.xpos,features=word.feats or '',span=[a,b],head=base+word.head-1 if word.head else None,rel=word.deprel,sentence=si))
   if sentence.words:sentences.append(dict(i=si,first=base,last=len(words)-1,span=[words[base]['span'][0],words[-1]['span'][1]]))
  result=dict(source_sha256=key,words=words,sentences=sentences,parser_revision=self.meta['revision']);write(file,result);return result
