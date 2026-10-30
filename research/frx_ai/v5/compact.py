"""Small upstream-trained parser comparison; normalized to the same word schema."""
import hashlib,json
from pathlib import Path
from common import digest,write

class CompactSyntax:
 def __init__(self,root):
  self.root=Path(root);self.meta=json.loads((self.root/'compact-manifest.json').read_text());self.cache=self.root/'compact-cache';self.cache.mkdir(exist_ok=True);self.pipeline=None
  self.directory=Path(self.meta['directory'])
  for name,sha in self.meta['files'].items():
   if hashlib.sha256((self.directory/name).read_bytes()).hexdigest()!=sha:raise ValueError('compact parser artifact changed')
 def parse(self,text):
  if not isinstance(text,str) or len(text)>16000:raise ValueError('bounded syntax input')
  key=digest(text);file=self.cache/(key+'.json')
  if file.exists():
   cached=json.loads(file.read_text())
   if cached.get('source_sha256')!=key or cached.get('parser_revision')!=self.meta['published_sha256']:raise ValueError('stale compact syntax cache')
   if any(text[slice(*w['span'])]!=w['text'] for w in cached['words']):raise ValueError('compact cache literal mismatch')
   return cached
  if self.pipeline is None:
   import spacy
   self.pipeline=spacy.load(self.directory,exclude=['ner']);self.pipeline.max_length=16001
  doc=self.pipeline(text);words=[];sentences=[]
  mapping={'ROOT':'root','dobj':'obj','auxpass':'aux:pass','nsubjpass':'nsubj:pass','csubjpass':'csubj:pass','neg':'advmod','relcl':'acl:relcl','acomp':'xcomp','attr':'xcomp','prt':'compound:prt'}
  for si,sentence in enumerate(doc.sents):
   start=len(words)
   for token in sentence:
    words.append(dict(i=token.i,text=token.text,lemma=token.lemma_,pos=token.pos_,tag=token.tag_,features=str(token.morph),span=[token.idx,token.idx+len(token.text)],head=None if token.head.i==token.i else token.head.i,rel=mapping.get(token.dep_,token.dep_),sentence=si))
   sentences.append(dict(i=si,first=start,last=len(words)-1,span=[sentence.start_char,sentence.end_char]))
  # Stanford dependencies attach prepositions to predicates; UD attaches the
  # content argument and treats the preposition as case/mark. Normalize only
  # this local tree transformation, never manufacture semantic relations.
  for token in doc:
   if token.dep_ in ['prep','agent']:
    objects=[x for x in token.children if x.dep_ in ['pobj','pcomp']]
    if len(objects)==1:
     obj=objects[0];words[obj.i]['head']=token.head.i;words[obj.i]['rel']='advcl' if obj.pos_=='VERB' else 'obl';words[token.i]['head']=obj.i;words[token.i]['rel']='mark' if obj.pos_=='VERB' else 'case'
  result=dict(source_sha256=key,words=words,sentences=sentences,parser_revision=self.meta['published_sha256']);write(file,result);return result
