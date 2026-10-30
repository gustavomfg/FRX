"""Bounded symbolic assertion verifier. It is not a learned language model.

Every accepted state has a literal cue; unknown arguments/results stay unknown.
These are reported events, not observed local trials or execution permissions.
"""
import re
from data import VERBS,span

ENV=re.compile(r'\b(?:PROTON_[A-Z0-9_]+|VKD3D_[A-Z0-9_]+|DXVK_[A-Z0-9_]+|RADV_[A-Z0-9_]+|WINE[A-Z0-9_]*)\b(?:\s*=\s*[^\s`]+)?')
PAST={'enabled','disabled','installed','uninstalled','removed','replaced','updated','upgraded','downgraded','switched','changed','copied','renamed','tried','used','set','deleted','added','cleaned','cleared','rebooted','shutdown','unplugged','ran','searched'}
PERSON=r"(?:I|we|he|she)(?:['’](?:ve|d|m))?"
DIRECT={'enable':'enable','disable':'disable','install':'install','uninstall':'remove','remove':'remove','delete':'remove','clean':'remove','clear':'remove','update':'update','upgrade':'update','downgrade':'update','replace':'replace','copy':'replace','switch':'switch','change':'configure','set':'configure','add':'configure','force':'configure','reduc':'configure','rename':'replace','unplug':'remove'}

def direction(word,local):
 w=word.lower()
 if re.search(r'turn\w*\s+off',w):return 'disable'
 if re.search(r'turn\w*\s+on',w):return 'enable'
 for prefix,result in DIRECT.items():
  if w.startswith(prefix):return result
 if re.search(r'\b(?:Proton|Wine|Wayland|Xorg|Pipewire|driver)\b',local,re.I) and w in ['use','using','used','try','tried']:return 'switch'
 if w in ['use','using','used','try','tried','launch','launching','launched'] or ENV.search(word):return 'configure'
 return 'unknown'

def sentence_bounds(text,a,b):
 # A dot inside version/file names is not a sentence boundary.
 separators=list(re.finditer(r'[.!?;](?=\s|$)|\n|\b(?:but|then|however)\b',text,re.I))
 left=max([m.end() for m in separators if m.end()<=a]+[0]);right=min([m.start() for m in separators if m.start()>=b]+[len(text)])
 return left,right

def quoted(text,a):
 line=text[text.rfind('\n',0,a)+1:a]
 return line.lstrip().startswith(('>','<!--'))

def verify(text,trigger):
 a,b=trigger
 if quoted(text,a):return None
 left,right=sentence_bounds(text,a,b);clause=text[left:right];offset=a-left;prefix=clause[:offset];word=text[a:b];suffix=clause[offset+len(word):]
 if re.search(r'(?i)\b(?:my comment|this comment|any update|updates for my system|table at 0x|GPU reset|package lists|0 upgraded|ubuntu PPA)\b',clause):return None
 # Code/shell transcripts do not establish that the author executed a step.
 if re.match(r'\s*(?:sudo\b|apt\b|git\b|cmake\b|make\b|\[\d|[\d.]+:[\da-f]+:)',clause,re.I):return None
 evidence=None;status='unknown';around=clause[max(0,offset-120):min(len(clause),offset+len(word)+100)];around_offset=left+max(0,offset-120)
 patterns=[
  ('negated',r"(?:haven['’]?t|have not|hadn['’]?t|had not|never|didn['’]?t|did not|not yet|unable to|can['’]?t|cannot|not allowing me to|stay away from)\s+(?:\w+\s+){0,9}"+re.escape(word)),
  ('preexisting',r'(?:already\s+(?:been\s+)?|(?:is|are|was|were)\s+(?:now\s+)?(?:the only ones\s+)?)'+re.escape(word)+r'(?:\s+by default)?'),
  ('suggested',r"(?:you could|you can|you should|please|can you|could you|i['’]?ll|I will|will try|Tomorrow|decided to|plan to|intend to)[^.!?\n]{0,140}"),
 ]
 for label,pattern in patterns:
  m=re.search(pattern,around,re.I)
  if m:
   # A future edit after a past action must not negate the earlier event.
   if label=='suggested' and around_offset+m.start()>a:continue
   if label=='preexisting' and not (around_offset+m.start()<=a and around_offset+m.end()>=b):continue
   if label=='preexisting' and re.match(r'(?i)already',m[0]) and not re.search(r'(?i)\b(?:is|was|are|were|been)\b',prefix[-25:]) and not re.search(r'(?i)\b(?:before|months ago|years ago|since)\b',suffix):continue
   status=label;evidence=[around_offset+m.start(),around_offset+m.end()];break
 # Questions about whether an action happened are not reports that it did.
 if re.search(r'(?i)\b(?:Has anyone|did you|have you|does anyone|what should|if you)\b',prefix):status='unknown';evidence=None
 if status=='unknown':
  m=re.search(r"(?i)\b"+PERSON+r"\s+(?:(?:have|had|also|just|actually|already|did|am|was|been)\s+){0,5}"+re.escape(word)+r'\b',clause)
  if m and (word.lower() in PAST or word.lower() in ['trying'] or re.search(r'(?i)\bdid\s+',m[0])):status='performed';evidence=[left+m.start(),left+m.end()]
  elif re.search(r"(?i)\b"+PERSON+r"\s+(?:(?:have|had|also|just)\s+){0,3}tried\s+(?:to\s+)?(?:manually\s+)?$",prefix):status='performed';evidence=[left,b]
  elif word.lower() in PAST and not prefix.strip(' `*,:-') and not re.search(r'\bif\b',clause[:offset],re.I):status='performed';evidence=[a,b]
  elif word.lower() in PAST and re.fullmatch(r'(?i)\s*(?:just|yesterday|also)\s+',prefix):status='performed';evidence=[left,b]
  elif re.search(r'(?i)\b(?:I|we)\s+(?:had to|did a clean)\s+',prefix) and word.lower() in ['change','replace']:
   status='performed';evidence=[left,a+len(word)]
  elif word.lower() in PAST and re.search(r'(?i)\b'+PERSON+r'\s+(?:\w+\s+){0,3}(?:installed|disabled|enabled|removed|uninstalled|replaced|updated|changed|copied|tried|did a clean replace)\b[^.!?]{0,130}\band\b',prefix) and not re.search(r'(?i)\b(?:will|could|would|not|never|if)\b',prefix):
   status='performed';evidence=[left,b]
  elif re.search(r'(?i)\b(?:fixed|resolved|caused|made|was able|were able|started|stopped|worked)\b',suffix) and word.lower().endswith('ing') and not re.search(r'(?i)\b(?:if|try|would|could)\b',prefix):
   status='performed';evidence=[a,right]
  elif re.search(r'(?i)\b(?:fix|workaround) I (?:found|used) was\s*$',prefix) and word.lower().endswith('ing'):status='performed';evidence=[left,b]
  elif re.match(r'(?i)\s*(?:try\b|remove\b|disable\b|enable\b)',clause) and not word.lower() in PAST:
   status='suggested';evidence=[left,b]
 if status=='performed' and text[right:right+1]=='?':status='unknown';evidence=None
 # Literal identity only. Descriptive references never become invented DLLs.
 end=min(right,b+180);tail=text[b:end]
 stop=re.search(r'[,;.!?](?=\s|$)|\b(?:but|then|and|which|caused|fixed|resolved|made|introduces)\b',tail,re.I)
 if stop:tail=tail[:stop.start()]
 tail=tail.strip(' `:*')
 identity=None;identity_span=None
 for pattern in [ENV,re.compile(r'\b[\w.+-]+\.(?:dll|so|exe)\b',re.I)]:
  m=pattern.search(text[a:end])
  if m:identity=m[0];identity_span=[a+m.start(),a+m.end()];break
 return dict(trigger=span(text,trigger),status=status,direction=direction(word,clause),
   status_evidence=span(text,evidence) if evidence else None,
   argument_description=tail[:160] or None,target_identity=identity,target_evidence=span(text,identity_span) if identity_span else None,
   reported_result='unknown',causal_effect_established=False,execution_authorized=False,
   reliability='literal-rule-verified' if status!='unknown' else 'unverified-candidate')

def lexical(text):
 found=[]
 for m in VERBS.finditer(text):
  if quoted(text,m.start()):continue
  item=verify(text,[m.start(),m.end()])
  if item:found.append(item)
 return found

def attach_observations(text,events):
 """Conservative local edges; no individual credit for coordinated actions."""
 observations=[];last_group=[];last_end=0
 segments=[]
 for line in re.finditer(r'[^\n]+',text):
  start=line.start()
  for stop in re.finditer(r'[.!?](?=\s|$)',line[0]):
   end=line.start()+stop.end();segments.append((start,end));start=end
  if start<line.end():segments.append((start,line.end()))
 expanded=[]
 for start,end in segments:
  cursor=start
  for separator in re.finditer(r'\b(?:but|however)\b',text[start:end],re.I):
   boundary=start+separator.start();expanded.append((cursor,boundary));cursor=start+separator.end()
  if cursor<end:expanded.append((cursor,end))
 for segstart,segend in expanded:
  chunk=text[segstart:segend]
  if quoted(text,segstart):continue
  result=None;outcome=None
  local_events=[i for i,e in enumerate(events) if segstart<=e['trigger']['span'][0]<segend]
  for label,pattern in [('persistent',r'\b(?:did not work|didn[’\x27]t work|did not change anything|no change|no difference|no luck|still crash\w*|problem persists|same system hang)\b'),
    ('regression',r'\b(?:caused[^.!?]{0,60}(?:crash\w*|artifact\w*)|introduces the issue|broke it|worse[^.!?]{0,35}(?:fps|frames|performance)|performance got worse)\b'),
    ('improved',r'\b(?:fixed[^.!?]{0,60}|resolved[^.!?]{0,60}|made the game work again|game loads and works|things work|start running for me)\b')]:
   m=re.search(pattern,chunk,re.I)
   if m:result=[segstart+m.start(),segstart+m.end()];outcome=label;break
  if not result:
   if local_events:
    last_group=local_events;last_end=segend
   continue
  owners=[i for i,e in enumerate(events) if e['status']=='performed' and segstart<=e['trigger']['span'][0]<result[0]]
  # A single causal cue/within-sentence report is required; adjacency alone is insufficient.
  if not owners and last_group and segstart-last_end<120 and re.match(r'(?i)\s*(?:It|That|This)\b',chunk) and all(events[i]['status']=='performed' for i in last_group):owners=list(last_group)
  if not owners:continue
  earlier=[e for e in events if segstart<=e['trigger']['span'][0]<result[0]]
  if any(e['status']!='performed' for e in earlier):continue
  if re.search(r'(?i)\b(?:not sure|can[’\x27]t say|might|maybe|if|would|could)\b',chunk):continue
  observations.append(dict(evidence=span(text,result),outcome=outcome,event_indices=owners,joint=len(owners)>1,
    individual_cause='unknown',causality_confirmed=False,scope='literal-local-reported-association'))
  if local_events:last_group=local_events;last_end=segend
 return observations

def temporal_edges(text,events):
 edges=[]
 ordered=sorted(enumerate(events),key=lambda p:p[1]['trigger']['span'][0])
 for (i,a),(j,b) in zip(ordered,ordered[1:]):
  if a['status']!='performed' or b['status']!='performed':continue
  left=a['trigger']['span'][1];right=b['trigger']['span'][0];m=re.search(r'\bthen\b',text[left:right],re.I)
  if m:edges.append(dict(before=i,after=j,cue=span(text,[left+m.start(),left+m.end()]),relation='explicit-reported-before',causality_confirmed=False))
 return edges
