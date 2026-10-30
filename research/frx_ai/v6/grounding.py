"""Evidence-bound event identities. Rules are explicit, not a trained coreferencer."""
import re,copy
from bridge import digest,evidence,prior_events,prior_observations,children,family
from events import state as prior_state

EXTRA_OPERATIONS={'adjust':'configure','revisit':'configure','submit':'unknown','provide':'unknown','upload':'unknown','go':'switch','start':'unknown','boot':'unknown','play':'unknown','look':'unknown','click':'unknown'}

def additional_events(text,parsed,existing):
 result=[];words=parsed['words'];nodes={e['node'] for e in existing}
 for w in words:
  if w['i'] in nodes or w['lemma'].lower() not in EXTRA_OPERATIONS or w['pos']!='VERB':continue
  own=children(words,w['i']);subjects=[x for x in own if x['rel'].startswith('nsubj')]
  if subjects and not any(x['lemma'].lower() in ['i','we','you','he','she','they','friend'] for x in subjects):continue
  a,b=w['span'];sentence=parsed['sentences'][w['sentence']];near=text[a:min(sentence['span'][1],a+180)]
  if w['lemma'].lower() in ['adjust','revisit','go'] and not re.search(r'(?i)\b(?:setting|settings|driver|proton|wine|kernel|offline|\d+\.\d+|rate|bit|config|limit)\b',near):continue
  if w['lemma'].lower() in ['submit','provide','upload','look'] and not re.search(r'(?i)\b(?:log|report|output|information|diagnostic|system)\b',near):continue
  status,cue=prior_state(text,words,w)
  if status=='unknown' and (w['tag']=='VBD' or any(x['lemma'].lower()=='be' and x['tag']=='VBD' for x in own)) and not any(x['lemma'].lower() in ['not','never','if','unless','will','would','could'] for x in own):status='performed';cue=evidence(text,a,b)
  route=ancestors(words,w['i']);parent=words[w['head']] if w['head'] is not None else None
  if status=='unknown' and w['tag']=='VBG' and parent and parent['lemma'].lower() in ['fix','resolve','make'] and not any(x['lemma'].lower() in ['will','would','could','should','if'] for p in route[:2] for x in children(words,p['i'])):
   if any(x['lemma'].lower()=='by' for x in own) or re.search(r'(?i)\b(?:again|yesterday)\b',text[slice(*sentence['span'])]):status='performed';cue=evidence(text,a,b)
  args=[]
  for child in own:
   if child['rel'].split(':')[0] in ['obj','obl','xcomp']:
    parts=family(words,child['i']);left=min(x['span'][0] for x in parts);right=max(x['span'][1] for x in parts)
    if right-left<=180:args.append(evidence(text,left,right))
  result.append(dict(trigger=evidence(text,a,b),status=status,direction=EXTRA_OPERATIONS[w['lemma'].lower()],status_evidence=cue,arguments=args,target_identity=None,target_evidence=None,role='diagnostic' if w['lemma'].lower() in ['submit','provide','upload','look'] else 'reproduction' if w['lemma'].lower() in ['start','boot','play','click'] else 'corrective',node=w['i'],sentence=w['sentence'],lemma=w['lemma'].lower(),causal_effect_established=False,execution_authorized=False,reliability='experimental-typed-operation-proposal'))
 return result

def masked(text):
 # Preserve original offsets while allowing quoted events to be represented
 # as someone else's report rather than silently assigned to the document author.
 return re.sub(r'(?m)^(\s*)> ?',lambda m:' '*len(m[0]),text)

def literalize(value,text):
 if isinstance(value,dict):
  if 'span' in value and 'text' in value:
   a,b=value['span'];value.update(evidence(text,a,b))
  for v in list(value.values()):literalize(v,text)
 elif isinstance(value,list):
  for v in value:literalize(v,text)
 return value

def ancestors(words,node,limit=10):
 out=[];seen=set()
 while node is not None and node not in seen and len(out)<limit:
  seen.add(node);out.append(words[node]);node=words[node]['head']
 return out

def clause(text,parsed,node):
 word=parsed['words'][node];s=parsed['sentences'][word['sentence']];a,b=s['span'];cuts=[a]+[a+m.end() for m in re.finditer(r'(?i)\b(?:but|whereas|however|then)\b|;',text[a:b])]+[b]
 left=max(x for x in cuts if x<=word['span'][0]);right=min(x for x in cuts if x>word['span'][0]);return left,right

def game_mentions(text,catalog):
 out=[]
 for record in catalog:
  if not record.get('verified'):continue
  for name in [record['name']]+record.get('aliases',[]):
   tokens=re.findall(r'[A-Za-z0-9]+',name)
   if len(tokens)<2:continue  # Common one-word names are lexically ambiguous.
   pattern=r'(?<!\w)'+r'[\W_]*'.join(map(re.escape,tokens))+r'(?!\w)'
   for m in re.finditer(pattern,text,re.I):out.append(dict(app_id=record['app_id'],name=record['name'],evidence=evidence(text,*m.span()),basis='exact-verified-catalog-name',catalog_source=record['source']))
 for m in re.finditer(r'(?i)(?:\b(?:steam\s*)?app\s*id\s*[:=#]?\s*|https://store\.steampowered\.com/app/)([1-9]\d{1,9})\b',text):
  app=m[1];record=next((x for x in catalog if x['app_id']==app),None);out.append(dict(app_id=app,name=record['name'] if record else None,evidence=evidence(text,*m.span()),basis='explicit-source-AppID',catalog_source=record['source'] if record else None))
 # Literal names may identify a different context even when an abbreviation
 # cannot safely be linked to an AppID. No fuzzy ID inference.
 technical={'Proton','Wine','Linux','Windows','Steam','NVIDIA','AMD','Intel','Wayland','X11','DXVK','VKD3D','DX12','DX11','DX9','Lutris','Ubuntu','Arch','KDE','GNOME','Manjaro','CachyOS','Fedora','Bazzite','Gentoo','Debian','NixOS','Mint','SteamOS','Garuda','OpenSUSE','Vulkan','DirectX'}
 for m in re.finditer(r'\b(?i:in|on|playing|played|launching)\s+([A-Z][A-Za-z0-9]+(?:[ :\-]+[A-Z][A-Za-z0-9]+){0,4})',text):
  name=m[1].strip()
  if name.split()[0] in technical or len(name)<3:continue
  a,b=m.span(1)
  if any(min(b,x['evidence']['span'][1])>max(a,x['evidence']['span'][0]) for x in out):continue
  strong=bool(re.match(r'(?i)^(?:playing|played|launching)\b',m[0])) or name.startswith('Game ')
  out.append(dict(app_id=None,name=name,evidence=evidence(text,a,b),basis='literal-unlinked-game-name' if strong else 'unresolved-proper-name-candidate',catalog_source=None))
 unique={}
 for m in out:unique[(m['app_id'],m['name'],tuple(m['evidence']['span']))]=m
 return sorted(unique.values(),key=lambda x:x['evidence']['span'])

def identity(text,parsed,node,row,mentions):
 words=parsed['words'];w=words[node];a,b=clause(text,parsed,node);local=text[a:b];line=text[text.rfind('\n',0,w['span'][0])+1:w['span'][0]];quoted=line.lstrip().startswith('>') or any(m.start()<=w['span'][0]<m.end() for m in re.finditer(r'`[^`\n]+`|"[^"\n]+"',text));route=ancestors(words,node)
 # The first explicit grammatical subject controls the event. Do not use the
 # author of the web page as the actor without a supported first-person link.
 subjects=[]
 for parent in route:
  own=[x for x in children(words,parent['i']) if x['rel'].startswith('nsubj')]
  if own:subjects=own;break
 actor_kind='unknown';actor_id=None;actor_quote=None
 if quoted:actor_kind='quoted';actor_quote=evidence(text,text.rfind('\n',0,w['span'][0])+1,w['span'][1])
 elif subjects:
  subject=subjects[0];lex=subject['lemma'].lower();members=family(words,subject['i']);na=min(x['span'][0] for x in members);nb=max(x['span'][1] for x in members)
  if nb-na>100:na,nb=subject['span']
  actor_quote=evidence(text,na,nb)
  if lex in ['i','we']:actor_kind='reporter';actor_id=row.get('reporter_id')
  elif lex in ['you']:actor_kind='addressee'
  elif lex in ['he','she','they','friend','brother','wife','husband','someone'] or re.search(r'(?i)\b(?:my|a) (?:friend|brother|wife|husband)\b',text[na:nb]):actor_kind='third_party'
 for p in route[1:]:
  if p['lemma'].lower() not in ['say','tell','report','claim']:continue
  subject=[x for x in children(words,p['i']) if x['rel'].startswith('nsubj')]
  if subject and subject[0]['lemma'].lower() not in ['i','we'] and actor_kind=='reporter':actor_kind='third_party';actor_id=None;actor_quote=evidence(text,min(subject[0]['span'][0],w['span'][0]),max(subject[0]['span'][1],w['span'][1]))
 # A literal mention of another game overrides thread/page context. No page
 # fallback: even a valid Steam issue may compare several games.
 all_scoped=[m for m in mentions if a<=m['evidence']['span'][0]<b];scoped=[m for m in all_scoped if m['basis']!='unresolved-proper-name-candidate'];game=None;basis='unknown';ambiguity='unlinked-name-needs-review' if all_scoped and not scoped else None
 ids={m['app_id'] or 'name:'+m['name'].casefold() for m in scoped}
 if len(ids)==1:game=min(scoped,key=lambda m:abs(m['evidence']['span'][0]-w['span'][0]));basis='explicit-clause-mention'
 elif len(ids)>1:ambiguity='multiple-game-mentions-in-clause'
 elif not re.search(r'(?i)\b(?:this game|my game|another game|other games?)\b',local):
  prior=[m for m in mentions if m['evidence']['span'][1]<=a and a-m['evidence']['span'][1]<300]
  if prior and re.match(r'(?i)\s*[,.:]?\s*(?:it|the game|that game)\b',local) and len({m['app_id'] or 'name:'+m['name'].casefold() for m in prior})==1:
   game=prior[-1];basis='bounded-explicit-game-anaphora'
 # One explicit game in the source paragraph can supply discourse identity;
 # the title of the web page cannot. Any other-game or actor switch cancels it.
 contrast=bool(re.search(r'(?i)\b(?:but|whereas|however)\s*$',text[max(0,a-15):a])) and not re.match(r'(?i)^\s*(?:it|the game)\b',local)
 if game is None and not ambiguity and not contrast and not re.search(r'(?i)\b(?:other games?|another game|different game|my friend)\b',text) and not re.search(r'(?i)^\s*but\s+.*\bthis game\b',local):
  known=[m for m in mentions if m['app_id'] is not None]
  if known and len({m['app_id'] for m in known})==1 and actor_kind in ['reporter','quoted','unknown']:
   game=known[0];basis='single-explicit-game-discourse'
 # Extract literal context only; matching a GPU does not prove it was used.
 context={}
 for key,pattern in [('gpu',r'\b(?:(?:RTX|GTX)\s*\d{3,4}(?:\s*(?:Ti|SUPER|Mobile))?|RX\s*\d{3,4}(?:\s*(?:XT|XTX|Mobile))?|Arc\s*[AB]\d{3})\b'),('proton',r'\b(?:GE[- ]Proton|Proton(?:[- ]GE)?)\s*[- ]?\d[\w.\-]*'),('driver',r'\b(?:driver|drivers)\s*[:=]?\s*\d[\d.\w-]*')]:
  values=[evidence(text,a+m.start(),a+m.end()) for m in re.finditer(pattern,local,re.I)]
  context[key]=values[0] if len(values)==1 else None
  if len(values)>1:context[key+'_ambiguity']='multiple-values'
 return dict(document_id=row.get('document_id') or row.get('source_sha256') or digest(text),thread_id=row.get('thread') or row.get('url','').split('#')[0],reporter_id=row.get('reporter_id'),actor_kind=actor_kind,actor_id=actor_id,actor_evidence=actor_quote,game_app_id=game['app_id'] if game else None,game_name=game['name'] if game else None,game_evidence=game['evidence'] if game else None,game_basis=basis,game_ambiguity=ambiguity,unlinked_name_candidates=[m['evidence'] for m in all_scoped if m['basis']=='unresolved-proper-name-candidate'],technical_context=context,local_runtime_verified=False)

def execution_guard(text,parsed,event):
 e=copy.deepcopy(event);words=parsed['words'];w=words[e['node']];route=ancestors(words,e['node']);a,b=clause(text,parsed,e['node']);before=text[a:w['span'][0]];own=children(words,w['i']);reason=None
 if e['status'] in ['unknown','performed'] and re.search(r'(?i)\b(?:(?:told|asked|advised)\s+(?:me|us)|(?:was|were|been)\s+(?:told|asked|advised))\s+to\b',before):e['status']='suggested';reason='received-instruction'
 if e['status']=='preexisting' and any(x['lemma'].lower()=='after' and x['rel']=='mark' for x in own) and any(x['lemma'].lower()=='be' and x['tag']=='VBD' for x in own):e['status']='performed';reason='explicit-completed-passive-change'
 if e['status']=='performed':
  # Hypothetical trial, instruction received, or configuration already in use
  # does not assert that the source performed this modification.
  if re.search(r'(?i)\b(?:if|unless)\b',before) or any(x['lemma'].lower() in ['if','unless'] for p in route[:3] for x in children(words,p['i']) if x['rel']=='mark'):
   e['status']='unknown';reason='conditional-action-not-asserted'
  elif re.search(r'(?i)\b(?:told|asked|advised)\s+(?:me|us)\s+to\b',before):e['status']='suggested';reason='received-instruction'
  elif reason!='explicit-completed-passive-change' and w['tag'] in ['VBN','JJ'] and not any(x['rel'].startswith('nsubj') and x['lemma'].lower() in ['i','we'] for x in own) and (any(x['rel'].endswith(':pass') or x['lemma'].lower()=='be' for x in own) or re.search(r'(?i)\b(?:with|having)\b[^.!?;]*$',before)):
   e['status']='preexisting';reason='configuration-state-not-change'
 if e['status']=='unknown' and any(p['lemma'].lower()=='have' and p['tag']=='VBD' for p in route[1:]) and re.search(r"(?i)\bit(?: is|'s) fixed now\b",text[b:]):e['status']='performed';reason='retrospective-necessity-with-asserted-result'
 if e['status'] in ['unknown','suggested'] and w['tag']=='VB' and '?' not in text[a:b] and not re.search(r'(?i)\b(?:told|should|could|would|will|might|try)\b',text[a:b]) and re.search(r'(?i)\bthank (?:you|u)\b',text[a:b]) and re.search(r'(?i)\b(?:fix|fixed|solve|solved) my (?:issue|problem)\b',text[a:b]):e['status']='performed';reason='retrospective-elliptical-result-report'
 if reason:e['status_guard_reason']=reason
 return e

def deconstruct(row,parser,catalog,guard=True):
 text=row['text'];clean=masked(text);parsed=parser.parse(clean);events=prior_events(clean,parsed);events+=additional_events(clean,parsed,events);excluded=[m.span() for m in re.finditer(r'```[\s\S]*?(?:```|\Z)|~~[\s\S]*?~~',text)];events=[e for e in events if not any(a<=e['trigger']['span'][0]<b for a,b in excluded)];events=sorted(literalize(events,text),key=lambda x:x['trigger']['span']);mentions=game_mentions(text,catalog)
 if guard:events=[execution_guard(text,parsed,e) for e in events]
 labels=[m.span() for m in re.finditer(r'"[^"\n]{1,110}"',text) if not re.search(r'(?i)\b(?:i|we|you|he|she|they|please|should|could|would)\b',m[0])]
 events=[e for e in events if not any(a<=e['trigger']['span'][0]<b for a,b in labels)]
 for e in events:
  e['identity']=identity(text,parsed,e['node'],row,mentions);e['event_id']=digest(str(e['identity']['document_id'])+':'+str(e['trigger']['span'])+':'+digest(text));e['assertion_type']='external-source-report';e['automatic_approval']=False
  # The old extractor searched a fixed suffix. A literal belonging to the
  # next action must not become this action's identified component.
  target=e.get('target_evidence');boundary=min([f['trigger']['span'][0] for f in events if f['trigger']['span'][0]>e['trigger']['span'][0]]+[len(text)]);ca,cb=clause(text,parsed,e['node'])
  if target and (target['span'][1]>boundary or not ca<=target['span'][0]<cb):e['target_identity']=None;e['target_evidence']=None;e['target_ambiguity']='literal-belongs-to-another-event-or-clause'
 observed=literalize(prior_observations(clean,parsed),text)
 for m in re.finditer(r'(?i)\b(?:all|both)\s+(?:still\s+)?have\s+(?:the\s+)?same\s+(?:issue|problem|result)\b|\b(?:still\s+)?(?:the\s+)?same\s+(?:issue|problem|result|performance)\b',text):
  if any(a<=m.start()<b for a,b in excluded):continue
  if any(min(m.end(),o['evidence']['span'][1])>max(m.start(),o['evidence']['span'][0]) for o in observed):continue
  hits=[w for w in parsed['words'] if m.start()<=w['span'][0]<m.end()]
  if not hits:continue
  sentence=parsed['sentences'][hits[0]['sentence']];raw=text[slice(*sentence['span'])]
  if '?' in raw or re.search(r'(?i)\b(?:would|could|should|might)\b',raw):continue
  head=next((w for w in hits if w['head'] not in {x['i'] for x in hits}),hits[0]);observed.append(dict(evidence=evidence(text,*m.span()),outcome='persistent',node=head['i'],sentence=head['sentence'],lemma=head['lemma'],modality='reported',causality_confirmed=False))
 observed.sort(key=lambda x:x['evidence']['span'])
 for obs in observed:obs['identity']=identity(text,parsed,obs['node'],row,mentions)
 temporal=[]
 for i,e in enumerate(events):
  if e['status']!='performed':continue
  for j,f in enumerate(events[i+1:],i+1):
   if f['status']!='performed':continue
   gap=text[e['trigger']['span'][1]:f['trigger']['span'][0]];cue=re.search(r'(?i)\bthen\b|\bafter that\b',gap)
   if cue:
    a=e['trigger']['span'][1]+cue.start();temporal.append(dict(before=i,after=j,cue=evidence(text,a,a+len(cue[0])),scope='reported-textual-sequence',actual_time_verified=False,causality_confirmed=False));break
 return dict(events=events,observations=observed,temporal=temporal,parsed=parsed,game_mentions=mentions,source_sha256=row.get('source_sha256',digest(text)),unit_sha256=digest(text))
