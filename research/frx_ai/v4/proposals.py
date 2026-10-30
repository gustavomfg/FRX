"""A small data-derived event lexicon, compared with neural proposals.

This is a deterministic lexicon learned from annotation vocabulary, not an
encoder or a causal model. It never reads final annotations.
"""
import re
from data import VERBS,write,read
from proof import verify,attach_observations,temporal_edges

def stem(word):
 word=word.casefold()
 irregular={'tried':'try','trying':'try','ran':'run','used':'use','using':'use','copied':'copy','copying':'copy'}
 if word in irregular:return irregular[word]
 for ending in ['ing','ed']:
  if word.endswith(ending) and len(word)>len(ending)+2:
   result=word[:-len(ending)]
   if len(result)>2 and result[-1]==result[-2]:result=result[:-1]
   return result
 if word.endswith('e') and len(word)>3:return word[:-1]
 return word

def fit(rows):
 roots=set()
 for row in rows:
  for event in row['events']:
   text=row['text'][slice(*event['trigger'])]
   if re.fullmatch(r'[A-Za-z-]+',text):roots.add(stem(text))
 return dict(format=1,roots=sorted(roots),source='audited-training-trigger-vocabulary',neural_model=False)

def propose(text,lexicon=None):
 spans={tuple(m.span()) for m in VERBS.finditer(text)}
 if lexicon:
  for m in re.finditer(r'\b[A-Za-z][A-Za-z-]*\b',text):
   if stem(m[0]) in lexicon['roots']:spans.add(tuple(m.span()))
 events=[]
 for a,b in sorted(spans):
  # "tried installing" is one attempted installation, not two independent changes.
  if text[a:b].lower() in ['try','tried','trying'] and re.match(r'\s+(?:to\s+)?(?:manually\s+)?(?:install|enable|disable|replace|remove|chang|switch|updat|upgrad|downgrad)',text[b:],re.I):continue
  item=verify(text,[a,b])
  if item:events.append(item)
 return dict(events=events,observations=attach_observations(text,events),temporal=temporal_edges(text,events))
