"""Literal, versioned event annotations; old labels are never overwritten."""
import hashlib
import json
from pathlib import Path
import re

STATUSES=['unknown','performed','suggested','negated','preexisting']
DIRECTIONS=['unknown','enable','disable','install','remove','update','replace','switch','configure']
VERBS=re.compile(r'\b(?:enabl\w*|disabl\w*|install\w*|uninstall\w*|replac\w*|remov\w*|updat\w*|upgrad\w*|downgrad\w*|switch\w*|chang\w*|cop(?:y|ied|ying)|renam\w*|set(?:ting)?|tried|try|trying|delete\w*|launch\w*|use|using|used|turn(?:ed|ing)?\s+(?:on|off))\b',re.I)

def read(path):return [json.loads(x) for x in Path(path).read_text().splitlines() if x.strip()]
def write(path,obj):Path(path).write_text(json.dumps(obj,indent=2,ensure_ascii=False))
def jsonl(path,rows):Path(path).write_text(''.join(json.dumps(x,ensure_ascii=False)+'\n' for x in rows))
def digest(text):return hashlib.sha256(text.encode()).hexdigest()

def literal(text,phrase,occurrence=None):
 if phrase is None:return None
 pattern=(r'(?<!\w)' if phrase[0].isalnum() else '')+re.escape(phrase)+(r'(?!\w)' if phrase[-1].isalnum() else '')
 matches=[m.start() for m in re.finditer(pattern,text)]
 index=0 if occurrence is None else occurrence
 if len(matches)<=index:raise ValueError('annotation phrase absent: '+repr(phrase))
 if occurrence is None and len(matches)>1:raise ValueError('ambiguous annotation phrase; supply occurrence: '+repr(phrase))
 a=matches[index];return [a,a+len(phrase)]

def span(text,value):
 if not isinstance(value,list) or len(value)!=2 or any(type(x) is not int for x in value):raise ValueError('two integer offsets required')
 a,b=value
 if not 0<=a<b<=len(text):raise ValueError('offset outside source')
 return dict(span=value,text=text[a:b],offset_encoding='unicode-codepoints',text_sha256=digest(text))

def authored_paragraphs(text):
 bounds=[0]+[m.end() for m in re.finditer(r'\r?\n\s*\r?\n',text)]
 output=[]
 for i,a in enumerate(bounds):
  b=bounds[i+1] if i+1<len(bounds) else len(text)
  while a<b and text[a].isspace():a+=1
  while b>a and text[b-1].isspace():b-=1
  if b-a<25 or text[a:b].lstrip().startswith(('>','<!--','# Compatibility','# System','## I confirm')):continue
  if b-a>1100:
   first=VERBS.search(text[a:b]);start=a+max(0,(first.start() if first else 0)-120);a=start;b=min(b,start+1100)
  output.append([a,b])
 return output

def validate(row):
 text=row['text'];events=row.get('events',[])
 for e in events:
  span(text,e['trigger'])
  if e.get('target'):span(text,e['target'])
  if e.get('status_span'):span(text,e['status_span'])
  if e['status'] not in STATUSES or e['direction'] not in DIRECTIONS:raise ValueError('annotation ontology')
 for o in row.get('observations',[]):
  span(text,o['span'])
  if any(type(i) is not int or i<0 or i>=len(events) for i in o['event_indices']):raise ValueError('observation association')
  if len(o['event_indices'])>1 and not o.get('joint'):raise ValueError('joint result requires joint flag')
 for edge in row.get('temporal',[]):span(text,edge['cue'])

def annotate(row,events,observations=(),temporal=(),note=''):
 """DSL tuples carry literal phrases, not predictions. See ANNOTATION.md."""
 r=dict(row,annotation_version='v4-events-1',annotation_provenance='provisional-agent; no independent human review',events=[],observations=[],temporal=[],audit_note=note)
 def locate(value):
  if isinstance(value,list):return value
  if isinstance(value,tuple):return literal(r['text'],value[0],value[1])
  return literal(r['text'],value)
 for trigger,status,direction,target,cue in events:
  cue_span=locate(cue) if cue else None
  def anchored(value):
   if cue_span and isinstance(value,str):
    try:
     local=literal(r['text'][cue_span[0]:cue_span[1]],value)
     return [local[0]+cue_span[0],local[1]+cue_span[0]]
    except ValueError:pass
   return locate(value)
  r['events'].append(dict(trigger=anchored(trigger),status=status,direction=direction,target=anchored(target) if target else None,status_span=cue_span))
 for quote,outcome,indices in observations:r['observations'].append(dict(span=locate(quote),outcome=outcome,event_indices=list(indices),joint=len(indices)>1))
 for before,after,cue in temporal:r['temporal'].append(dict(before=before,after=after,cue=locate(cue)))
 validate(r);return r
