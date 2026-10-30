"""Atomic syntax-backed event proposals and literal modality reconstruction.

Upstream-trained dependencies supply grammar. These reconstruction decisions
are deterministic and provisional, not a learned FRX causal model.
"""
import re
from common import evidence

OPERATIONS={'enable':'enable','disable':'disable','install':'install','uninstall':'remove','remove':'remove','replace':'replace','update':'update','upgrade':'update','downgrade':'update','switch':'switch','change':'configure','copy':'replace','rename':'replace','set':'configure','unset':'remove','try':'configure','use':'configure','launch':'configure','run':'configure','test':'unknown','reboot':'unknown','restart':'unknown','clear':'remove','delete':'remove','add':'configure','lower':'configure','turn':'unknown','open':'unknown','attach':'unknown','share':'unknown','grab':'unknown','build':'unknown','purge':'remove','write':'configure','retest':'unknown','experiment':'configure','move':'replace','reduce':'configure','limit':'configure','quit':'unknown','close':'unknown','attempt':'configure','clean':'remove','configure':'configure','drop':'replace','force':'configure'}
ENV=re.compile(r'\b(?:PROTON_[A-Z0-9_]+|VKD3D_[A-Z0-9_]+|DXVK_[A-Z0-9_]+|RADV_[A-Z0-9_]+|WINE[A-Z0-9_]*)\b(?:\s*=\s*[^\s`]+)?')
MODALS={'should','could','may','might','would','will','shall','can'}
PLANS={'plan','intend','recommend','suggest','want','hope','ask','need'}

def canonical(w):
 lemma=w['lemma'].lower()
 if lemma in OPERATIONS:return lemma
 word=w['text'].lower()
 forms={'enabl':'enable','disabl':'disable','retest':'test','reinstall':'install','uninstall':'uninstall','install':'install','replac':'replace','remov':'remove','updat':'update','upgrad':'upgrade','downgrad':'downgrade','switch':'switch','chang':'change','purge':'purge','configur':'configure'}
 for stem,base in forms.items():
  if word.startswith(stem) and word[len(stem):] in ['', 'e','ed','d','ing','s','es']:return base
 if word=='setting':return 'set'
 return lemma

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
  if parent['i'] in seen or out[-1]['rel'].split(':')[0] not in ['conj','xcomp','ccomp','advcl','csubj','acl','obl','obj','nmod']:break
  out.append(parent);seen.add(parent['i'])
 return out

def operation(w,words,text):
 lemma=canonical(w);direction=OPERATIONS.get(lemma,'unknown')
 if lemma=='turn':
  particles=[x['lemma'].lower() for x in children(words,w['i'])]
  direction='disable' if 'off' in particles else 'enable' if 'on' in particles else 'unknown'
 if lemma in ['try','use','attempt']:
  clause=[x for x in words if x['sentence']==w['sentence']]
  if any(x['lemma'].lower() in ['proton','wine','driver'] for x in clause):direction='switch'
 return direction

def initial_state(text,words,w,mode='syntax'):
 chain=governing(words,w);direct=children(words,w['i']);context=[x for x in words if x['sentence']==w['sentence']];sentence=text[context[0]['span'][0]:context[-1]['span'][1]];before=text[context[0]['span'][0]:w['span'][0]]
 # The action's own auxiliaries and trial governors carry modality; a negative
 # result in a different clause must not deny the preceding trial.
 controls=[x for x in chain if x==w or x['lemma'].lower() in ['try','attempt',*PLANS] or x['rel'].split(':')[0]=='conj'];related=[x for c in controls for x in children(words,c['i'])]
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
 if w['tag']=='VBG' and parent and w['rel'].split(':')[0] in ['csubj','advcl','obl'] and parent['lemma'].lower() in ['fix','resolve','solve','cause','work','stop','help','improve','eliminate'] and 'Tense=Past' in parent['features']:return 'performed',cue
 if w['lemma'].lower()=='try' and not subjects and w['tag'] in ['VB','VBP']:return 'suggested',cue
 if not subjects and not before.strip(' \r\n-*`1234567890.():,') and w['tag']=='VB':return 'suggested',cue
 return 'unknown',cue

def state(text,words,w,mode='syntax'):
 value,cue=initial_state(text,words,w,mode)
 if value!='unknown':return value,cue
 chain=governing(words,w);direct=children(words,w['i']);sentence=[x for x in words if x['sentence']==w['sentence']];raw=text[sentence[0]['span'][0]:sentence[-1]['span'][1]]
 if '?' in raw:return value,cue
 # Scope auxiliaries to each predicate; an unrelated result's negation does
 # not retroactively deny an attempted action.
 for parent in chain:
  own=children(words,parent['i']);subjects=[x for x in own if x['rel'].startswith('nsubj')];personal=any(x['lemma'].lower() in ['i','we'] for x in subjects)
  neg=any(x['lemma'].lower() in ['not','never'] for x in own if x['rel'].split(':')[0] in ['advmod','det'])
  modal=any(x['lemma'].lower() in MODALS for x in own if x['rel'].startswith('aux'))
  if neg or modal:continue
  past=parent['tag'] in ['VBD','VBN'] or any(x['tag']=='VBD' for x in own if x['rel'].startswith('aux'))
  introduced_trial=canonical(parent) in ['try','attempt']
  if introduced_trial and past and not subjects:return 'performed',evidence(text,parent['span'][0],w['span'][1])
  if personal and ((past and (parent['i']==w['i'] or introduced_trial or w['rel']=='conj')) or introduced_trial):
   return 'performed',evidence(text,min(x['span'][0] for x in subjects),w['span'][1])
  if parent['i']!=w['i'] and parent['rel'] in ['root','conj'] and parent['tag']=='VBD' and not subjects and w['tag'] in ['VBD','VBN'] and w['rel']=='conj':
   return 'performed',evidence(text,parent['span'][0],w['span'][1])
 # A temporal subordinate explicitly preceding an asserted observation is
 # a reported event even without repeating the personal subject.
 marks=[x for x in direct if x['lemma'].lower() in ['after','by','with']]
 parent=words[w['head']] if w['head'] is not None else None
 # Parser repair: a gerund can be attached as an adjective of the named
 # setting (the subject), rather than as a clausal subject of the result.
 if w['tag']=='VBG' and parent and w['rel']=='amod' and parent['rel'].startswith('nsubj') and parent['head'] is not None:
  result=words[parent['head']]
  if result['lemma'].lower() in ['cause','fix','resolve','solve','worsen','improve'] and result['tag']=='VBD' and not any(x['lemma'].lower() in MODALS for x in children(words,result['i'])):
   return 'performed',evidence(text,w['span'][0],result['span'][1])
 # Local coordination repair for malformed technical names/abbreviations.
 # Require an asserted personal past operation, a conjunction, no new clause
 # boundary and no intervening operation or modality.
 if w['tag'] in ['VBD','VBN'] and w['rel']=='conj' and not any(x['lemma'].lower() in ['not','never',*MODALS] for x in direct):
  prior=[x for x in words if x['sentence']==w['sentence'] and x['span'][1]<w['span'][0] and canonical(x) in OPERATIONS and x['tag']=='VBD']
  if prior:
   anchor=prior[-1];gap=text[anchor['span'][1]:w['span'][0]]
   if re.search(r'(?i)\b(?:and|then)\b',gap) and not re.search(r'[.!?;]',gap) and any(x['lemma'].lower() in ['i','we'] and x['rel'].startswith('nsubj') for x in children(words,anchor['i'])):
    return 'performed',evidence(text,anchor['span'][0],w['span'][1])
 if marks and parent and canonical(w) in OPERATIONS and parent['lemma'].lower() in ['work','fix','resolve','solve','keep','get','start','be','remove','crash','hang'] and parent['tag'] in ['VBD','VBP','VBZ','VBN'] and not any(x['lemma'].lower() in MODALS for x in children(words,parent['i'])):
  return 'performed',evidence(text,min(x['span'][0] for x in marks),w['span'][1])
 if w['rel']=='conj' and parent and canonical(parent) in OPERATIONS and not any(x['lemma'].lower() in ['not','never',*MODALS] for x in direct):
  parent_status,parent_cue=state(text,words,parent,mode)
  if parent_status=='performed':return 'performed',evidence(text,parent_cue['span'][0],w['span'][1])
 # Reporting predicates can embed an attempted operation in present tense.
 if parent and parent['lemma'].lower() in ['confirm','try'] and not re.search(r"(?i)\b(?:not|never|if|should|could|would|will)\b",text[parent['span'][0]:w['span'][0]]):
  if any(x['lemma'].lower() in ['i','we'] and x['rel'].startswith('nsubj') for x in children(words,parent['i'])):return 'performed',evidence(text,parent['span'][0],w['span'][1])
 return value,cue

def extract(text,parsed,mode='syntax'):
 words=parsed['words'];events=[]
 for w in words:
  lemma=canonical(w)
  if lemma not in OPERATIONS or w['pos'] not in ['VERB','AUX','ADJ','NOUN']:continue
  a,b=w['span'];line=text[text.rfind('\n',0,a)+1:a]
  if line.lstrip().startswith(('>','```','sudo ','apt ','cp ','git ')):continue
  if any(m.start()<=a<m.end() for m in re.finditer(r'```[\s\S]*?(?:```|\Z)|~~[\s\S]*?~~',text)):continue
  direct=children(words,w['i'])
  if lemma in ['try','use','attempt'] and any(canonical(x) in OPERATIONS and x['pos']=='VERB' and any(a['i']==w['i'] for a in governing(words,x)[1:]) and any(a['rel'].split(':')[0] in ['xcomp','ccomp','acl'] for a in governing(words,x)[:-1]) for x in family(words,w['i']) if x['i']!=w['i']):continue
  if lemma in ['open','attach','share','grab']:
   tail=text[b:b+110]
   if not re.search(r'(?i)\b(?:issue|log|report|output|console|terminal|file)\b',tail):continue
  if lemma in ['run','launch'] and any(x['rel'].startswith('nsubj') and x['lemma'].lower() not in ['i','we','you'] for x in direct):continue
  status,cue=state(text,words,w,mode)
  clause=text[a:min(len(text),a+120)]
  # Explicitly reported Windows trials are retained separately by the caller,
  # not accepted as current Linux attempts. Wine Windows-version settings differ.
  if lemma in ['test','run','launch','try'] and re.search(r'(?i)\bon Windows\s*\d',clause):continue
  nominal=any(x['lemma'].lower()=='do' and x['tag']=='VBD' and any(c['lemma'].lower() in ['i','we'] for c in children(words,x['i']) if c['rel'].startswith('nsubj')) for x in governing(words,w)[1:])
  if nominal and status=='unknown':
   actor=next(x for x in governing(words,w)[1:] if x['lemma'].lower()=='do');status='performed';cue=evidence(text,min(x['span'][0] for x in children(words,actor['i']) if x['rel'].startswith('nsubj')),b)
  if w['pos']=='NOUN' and not nominal and lemma not in ['set','restart','reboot'] and not (w['text'].lower().endswith('ing') and any(canonical(x)=='try' for x in governing(words,w)[1:])):continue
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
  role='diagnostic' if lemma in ['attach','share','grab','open'] else 'reproduction' if lemma in ['launch','run','test','restart','reboot','quit','close'] else 'corrective'
  events.append(dict(trigger=evidence(text,a,b),status=status,direction=operation(w,words,text),status_evidence=cue,arguments=arguments,target_identity=identity,target_evidence=identity_evidence,role=role,node=w['i'],sentence=w['sentence'],lemma=lemma,causal_effect_established=False,execution_authorized=False,reliability='experimental-syntactic-reconstruction'))
 return events
