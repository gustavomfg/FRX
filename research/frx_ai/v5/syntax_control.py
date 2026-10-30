"""Atomic syntax-backed event proposals and literal modality reconstruction.

Upstream-trained dependencies supply grammar. These reconstruction decisions
are deterministic and provisional, not a learned FRX causal model.
"""
import re
from common import evidence

OPERATIONS={'enable':'enable','disable':'disable','install':'install','uninstall':'remove','remove':'remove','replace':'replace','update':'update','upgrade':'update','downgrade':'update','switch':'switch','change':'configure','copy':'replace','rename':'replace','set':'configure','unset':'remove','try':'configure','use':'configure','launch':'configure','run':'configure','test':'unknown','reboot':'unknown','restart':'unknown','clear':'remove','delete':'remove','add':'configure','lower':'configure','turn':'unknown','open':'unknown','attach':'unknown','share':'unknown','grab':'unknown','build':'unknown','purge':'remove','write':'configure','configure':'configure','drop':'replace','force':'configure'}
ENV=re.compile(r'\b(?:PROTON_[A-Z0-9_]+|VKD3D_[A-Z0-9_]+|DXVK_[A-Z0-9_]+|RADV_[A-Z0-9_]+|WINE[A-Z0-9_]*)\b(?:\s*=\s*[^\s`]+)?')
MODALS={'should','could','may','might','would','will','shall','can'}
PLANS={'plan','intend','recommend','suggest','want','hope','ask','need'}

def children(words,index):return [w for w in words if w['head']==index]
def family(words,index):
 selected={index};pending=[index]
 while pending:
  head=pending.pop()
  for w in children(words,head):
   if w['i'] not in selected:selected.add(w['i']);pending.append(w['i'])
 return [w for w in words if w['i'] in selected]

def governing(words,w):
 out=[w];seen={w['i']}
 while out[-1]['head'] is not None and len(out)<5:
  parent=words[out[-1]['head']]
  if parent['i'] in seen or out[-1]['rel'].split(':')[0] not in ['conj','xcomp','ccomp','advcl','csubj']:break
  out.append(parent);seen.add(parent['i'])
 return out

def operation(w,words,text):
 lemma=w['lemma'].lower();direction=OPERATIONS.get(lemma,'unknown')
 if lemma=='turn':
  particles=[x['lemma'].lower() for x in children(words,w['i'])]
  direction='disable' if 'off' in particles else 'enable' if 'on' in particles else 'unknown'
 if lemma in ['try','use']:
  clause=[x for x in words if x['sentence']==w['sentence']]
  if any(x['lemma'].lower() in ['proton','wine','driver'] for x in clause):direction='switch'
 return direction

def state(text,words,w,mode='syntax'):
 chain=governing(words,w);direct=children(words,w['i']);context=[x for x in words if x['sentence']==w['sentence']];sentence=text[context[0]['span'][0]:context[-1]['span'][1]];before=text[context[0]['span'][0]:w['span'][0]]
 # The action's own auxiliaries and trial governors carry modality; a negative
 # result in a different clause must not deny the preceding trial.
 controls=[x for x in chain if x==w or x['lemma'].lower() in ['try',*PLANS] or x['rel'].split(':')[0]=='conj'];related=[x for c in controls for x in children(words,c['i'])]
 subjects=[x for x in related if x['rel'].startswith('nsubj')];personal=any(x['lemma'].lower() in ['i','we'] for x in subjects)
 local=[x for x in related if x['rel'].split(':')[0] in ['aux','advmod','mark']]+controls
 cue_words=[x for x in local+subjects if x['span'][0]<=w['span'][1]]
 a=min([w['span'][0]]+[x['span'][0] for x in cue_words]);b=w['span'][1]
 cue=evidence(text,a,b)
 if '?' in sentence and (re.search(r'(?i)\b(?:have|did|has|anyone|you)\b',before) or not personal):return 'suggested' if any(x['lemma'].lower() in MODALS for x in local) else 'unknown',cue
 if any(x['lemma'].lower() in PLANS for x in chain[1:]) or any(x['lemma'].lower() in MODALS for x in related if x['rel'].startswith('aux')):return 'suggested',cue
 if re.search(r"(?i)\b(?:you (?:should|could|can|need)|please|let me know|tomorrow|plan to|will try)\b",before[-110:]):return 'suggested',cue
 denied=any(x['lemma'].lower() in ['not','never'] for x in related if x['rel'].split(':')[0] in ['advmod','det'])
 if denied:return 'negated',cue
 if any(x['lemma'].lower()=='already' for x in related) and not any(x['lemma'].lower()=='try' for x in chain):return 'preexisting',cue
 if any(x['rel'].endswith(':pass') for x in direct) or (w['tag']=='VBN' and any(x['lemma']=='be' for x in direct)) or (w['lemma'].lower()=='use' and w['tag']=='VBG' and personal):return 'preexisting',cue
 conditional=any(x['lemma'].lower() in ['if','unless'] for x in related)
 if conditional:return 'unknown',cue
 past=any('Tense=Past' in x['features'] or x['tag'] in ['VBD','VBN'] for x in controls)
 if any(x['lemma'].lower()=='try' and (x['tag']=='VBD' or personal) for x in chain):return 'performed',cue
 if past and personal:return 'performed',cue
 # Elliptical past reports are common in issue comments, unlike noun metadata.
 if past and not subjects and not before.strip(' \r\n-*`1234567890.():,') and w['pos']=='VERB':return 'performed',cue
 parent=words[w['head']] if w['head'] is not None else None
 if w['tag']=='VBG' and parent and w['rel'].split(':')[0] in ['csubj','advcl','obl'] and parent['lemma'].lower() in ['fix','resolve','cause','work','stop','help'] and 'Tense=Past' in parent['features']:return 'performed',cue
 if w['lemma'].lower()=='try' and not subjects and w['tag'] in ['VB','VBP']:return 'suggested',cue
 if not subjects and not before.strip(' \r\n-*`1234567890.():,') and w['tag']=='VB':return 'suggested',cue
 return 'unknown',cue

def extract(text,parsed,mode='syntax'):
 words=parsed['words'];events=[]
 for w in words:
  lemma=w['lemma'].lower()
  if lemma not in OPERATIONS or w['pos'] not in ['VERB','AUX','ADJ','NOUN']:continue
  a,b=w['span'];line=text[text.rfind('\n',0,a)+1:a]
  if line.lstrip().startswith(('>','```','sudo ','apt ','cp ','git ')):continue
  direct=children(words,w['i'])
  if lemma in ['try','use'] and any(x['lemma'].lower() in OPERATIONS and x['rel']=='xcomp' for x in direct):continue
  if lemma in ['open','attach','share','grab']:
   tail=text[b:b+110]
   if not re.search(r'(?i)\b(?:issue|log|report|output|console|terminal|file)\b',tail):continue
  if lemma in ['run','launch'] and any(x['rel'].startswith('nsubj') and x['lemma'].lower() not in ['i','we','you'] for x in direct):continue
  status,cue=state(text,words,w,mode)
  arguments=[]
  for child in direct:
   if child['rel'].split(':')[0] in ['obj','obl','xcomp','compound']:
    members=[x for x in family(words,child['i']) if x['pos'] not in ['PUNCT'] and x['lemma'].lower() not in OPERATIONS]
    if members:
     left=min(x['span'][0] for x in members);right=max(x['span'][1] for x in members)
     if right-left<=200:arguments.append(evidence(text,left,right))
  end=next((s['span'][1] for s in parsed['sentences'] if s['i']==w['sentence']),b)
  identity=None;identity_evidence=None
  for pattern in [ENV,re.compile(r'\b[\w.+-]+\.(?:dll|so|exe)\b',re.I)]:
   m=pattern.search(text[b:min(end,b+150)])
   if m:identity=m[0];identity_evidence=evidence(text,b+m.start(),b+m.end());break
  role='diagnostic' if lemma in ['attach','share','grab','open'] else 'reproduction' if lemma in ['launch','run','test','restart','reboot'] else 'corrective'
  events.append(dict(trigger=evidence(text,a,b),status=status,direction=operation(w,words,text),status_evidence=cue,arguments=arguments,target_identity=identity,target_evidence=identity_evidence,role=role,node=w['i'],sentence=w['sentence'],lemma=lemma,causal_effect_established=False,execution_authorized=False,reliability='experimental-syntactic-reconstruction'))
 return events
