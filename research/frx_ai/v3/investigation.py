"""Typed read-only investigation with explicit unknowns.

This module is deterministic, not a trained root-cause model. Caller-supplied
facts remain caller-attributed; a source URL does not authenticate its content.
"""
import hashlib
import json
import re
from urllib.parse import urlsplit

CONTEXT=['app_id','gpu','driver','proton']
WINE_SOURCE='https://github.com/wine-mirror/wine/blob/master/dlls/ntdll/loader.c'
PROTON_SOURCE='https://github.com/ValveSoftware/Proton#runtime-config-options'
VULKAN_SOURCE='https://registry.khronos.org/vulkan/specs/latest/man/html/VkResult.html'

def context_id(context):
 return hashlib.sha256(json.dumps({k:context.get(k) for k in CONTEXT},sort_keys=True).encode()).hexdigest()

def compatible(current,other):
 if current.get('app_id')!=other.get('app_id'):return 'incompatible'
 missing=False
 for k in CONTEXT[1:]:
  if current.get(k) and other.get(k):
   if current[k].casefold()!=other[k].casefold():return 'incompatible'
  else:missing=True
 return 'unknown' if missing else 'compatible'

def source_url(value):
 try:
  u=urlsplit(value);port=u.port
 except (ValueError,TypeError):return False
 return bool(u.scheme=='https' and u.hostname and not u.username and not u.password and not port and
  u.hostname in ['github.com','docs.nvidia.com','gpuopen.com','www.intel.com','registry.khronos.org','gitlab.winehq.org'])

def observed_log(text):
 """Only recognize explicit loader failure, not arbitrary DLL presence."""
 if not isinstance(text,str) or len(text)>64000:raise ValueError('log size/type bound')
 result=[]
 pattern=r'(?im)^.*(?:err:module:import_dll|err:module:load_builtin_dll).*?Library\s+["\']?([A-Za-z0-9_.+-]{1,100}\.dll)["\']?[^\n]*\bnot found\b[^\n]*'
 for m in re.finditer(pattern,text):
  result.append(dict(kind='supplied-log-observation',subsystem='loader',component=m[1],
     predicate='reported-library-not-found',quote=text[m.start():m.end()],span=[m.start(),m.end()],
     source_sha256=hashlib.sha256(text.encode()).hexdigest(),technical_source=WINE_SOURCE))
 for m in re.finditer(r'(?im)^[^\n]*(?:err:vkd3d|warn:vkd3d|MESA-[A-Z]+: error:)[^\n]*(?:VK_ERROR_DEVICE_LOST|Device [^\n]*? is lost|Failed to submit queue)[^\n]*',text):
  result.append(dict(kind='supplied-log-observation',subsystem='graphics',component=None,
    predicate='reported-device-or-queue-failure',quote=text[m.start():m.end()],span=[m.start(),m.end()],
    source_sha256=hashlib.sha256(text.encode()).hexdigest(),technical_source=VULKAN_SOURCE))
 return result[:16]

def attempt_key(attempt):
 op=attempt.get('action',{})
 if not isinstance(op,dict):raise ValueError('action must be structured data')
 return json.dumps({k:op.get(k) for k in ['kind','target','direction','version']},sort_keys=True)

def inspect(case):
 if not isinstance(case,dict):raise ValueError('case must be an object')
 context=case.get('context',{})
 if not isinstance(context,dict) or not re.fullmatch(r'[1-9]\d{1,9}',str(context.get('app_id',''))):raise ValueError('explicit AppID required')
 for k in CONTEXT[1:]:
  if context.get(k) is not None and not isinstance(context[k],str):raise ValueError('invalid environment field')
 evidence=case.get('evidence',[]);attempts=case.get('attempts',[]);facts=case.get('facts',[])
 if not all(isinstance(x,list) and len(x)<=64 for x in [evidence,attempts,facts]):raise ValueError('case collection bound')
 observations=observed_log(case.get('log',''));hypotheses=[];checks=[];rejected=[];sources=[];nodes=[];edges=[];seen=set();hypothesis_groups={}
 for i,obs in enumerate(observations):
  component=obs['component'];node='log:'+str(i);nodes.append(dict(id=node,type='observation',data=obs))
  group=(obs['subsystem'],component)
  if group in hypothesis_groups:
   hypotheses[hypothesis_groups[group]]['supports'].append(node)
   continue
  hypothesis_groups[group]=len(hypotheses)
  if obs['subsystem']=='graphics':
   hypotheses.append(dict(id='graphics:'+str(i),statement='The supplied log reports a graphics device or queue failure.',
     status='investigable-hypothesis',supports=[node],contradictions=[],limits=['Device loss does not uniquely identify driver, game, configuration or hardware as the cause.']))
   checks.append(dict(id='graphics-environment',question='Compare the actual GPU, driver, Proton and first graphics error with the cited incident; do not infer a repair from device loss alone.',read_only=True,requires_execution=False,source=VULKAN_SOURCE))
   continue
  hypotheses.append(dict(id='loader:'+component,statement='The supplied execution log reports an unavailable required module.',
    component=component,status='investigable-hypothesis',supports=[node],contradictions=[],
    limits=['A loader message does not identify the correct distributable or prove the cause of the complete incident.']))
  checks.append(dict(id='origin:'+component,question='Identify the module origin, required architecture/version and expected game component before proposing installation.',
    component=component,read_only=True,requires_execution=False,source=WINE_SOURCE))
 for i,item in enumerate(evidence):
  if not isinstance(item,dict):raise ValueError('evidence object required')
  env=dict(item.get('context',{}));env['app_id']=item.get('app_id');match=compatible(context,env)
  reason=None
  if match=='incompatible':reason='different-game-or-known-environment'
  elif not source_url(item.get('url','')):reason='invalid-or-unapproved-source-url'
  elif item.get('origin','').startswith('synthetic'):reason='synthetic-is-not-real-evidence'
  text=item.get('text')
  if not isinstance(text,str) or not text.strip() or len(text)>16000:reason='source-text-bound'
  if reason:rejected.append(dict(index=i,reason=reason));continue
  digest=hashlib.sha256(text.encode()).hexdigest();key=(item['app_id'],digest)
  if key in seen:continue
  seen.add(key);sources.append(dict(index=i,url=item['url'],text_sha256=digest,environment_match=match,provenance='caller-attributed'))
  nodes.append(dict(id='source:'+str(i),type='community-report',source=item['url'],sha256=digest,environment_match=match))
 # Facts must be explicit and scoped; hardware support never implies game use.
 technology={}
 for i,fact in enumerate(facts):
  if not isinstance(fact,dict):raise ValueError('fact object required')
  if fact.get('app_id')!=context['app_id'] or not source_url(fact.get('url','')):continue
  if fact.get('predicate')=='game-implements-technology':
   name=fact.get('technology')
   if not isinstance(name,str) or len(name)>80:continue
   technology.setdefault(name,[]).append(dict(value=fact.get('value'),source=fact['url'],provenance='caller-attributed-documentation'))
 for name,claims in technology.items():
  values={c['value'] for c in claims if type(c['value']) is bool}
  nodes.append(dict(id='technology:'+name,type='documented-game-technology',technology=name,
    status='conflicting' if len(values)>1 else 'reported-present' if values=={True} else 'reported-absent' if values=={False} else 'unknown',sources=claims))
 history=[];failed=set();known={};events=[]
 for i,attempt in enumerate(attempts):
  if not isinstance(attempt,dict):raise ValueError('attempt object required')
  env=attempt.get('context',{});match=compatible(context,env)
  if match!='compatible':
   rejected.append(dict(attempt=i,reason='attempt-context-not-fully-matched'));continue
  identity=attempt.get('id')
  if not isinstance(identity,str) or len(identity)>120:raise ValueError('stable attempt ID required')
  if identity in known:
   if known[identity]!=attempt:raise ValueError('contradictory duplicate attempt ID')
   continue
  known[identity]=attempt;key=attempt_key(attempt);outcome=attempt.get('outcome','unknown')
  if outcome not in ['unknown','not-resolved','positive','worsened','mixed']:raise ValueError('unknown attempt outcome')
  if outcome in ['not-resolved','worsened']:failed.add(key)
  history.append(dict(id=identity,action=attempt['action'],outcome=outcome,source_kind='caller-reported-attempt',context_fingerprint=context_id(context)))
  events.append(dict(id=identity,type='reported-attempt',outcome=outcome,individual_cause='unknown'))
  if i and history[-2:] and len(history)>1:
   edges.append(dict(source=history[-2]['id'],target=identity,relation='listed-before',temporal_order_confirmed=False))
 candidates=[]
 for proposal in case.get('candidate_actions',[])[:32]:
  if not isinstance(proposal,dict):raise ValueError('candidate action object required')
  match=compatible(context,proposal.get('context',{}));requirements=proposal.get('requirements',{})
  if not isinstance(requirements,dict):raise ValueError('requirements object required')
  missing=[];conflict=[]
  for k,v in requirements.items():
   if k not in CONTEXT:missing.append(k);continue
   if context.get(k) is None:missing.append(k)
   elif context[k]!=v:conflict.append(k)
  status='investigate'
  if match=='incompatible' or conflict:status='incompatible'
  elif attempt_key(proposal) in failed:status='previously-failed-same-context'
  elif match=='unknown' or missing:status='requirements-unknown'
  if not source_url(proposal.get('url','')):status='source-unverified'
  candidates.append(dict(action=proposal.get('action'),status=status,missing_requirements=missing,conflicting_requirements=conflict,
    source=proposal.get('url'),execution_authorized=False,efficacy_confirmed=False))
 if not observations:checks.append(dict(id='collect-relevant-log',question='Obtain an authorized bounded log and locate the first relevant failure; the symptom alone does not identify a cause.',read_only=True,requires_execution=False,source=PROTON_SOURCE))
 if not context.get('proton'):checks.append(dict(id='identify-proton',question='Identify the Proton build and version used in the failed execution.',read_only=True,requires_execution=False))
 return dict(status='research-investigation',context=context,context_fingerprint=context_id(context),sources=sources,rejected=rejected,
  observations=observations,graph=dict(nodes=nodes+events,edges=edges),hypotheses=hypotheses,next_checks=checks,
  attempts=history,candidate_actions=candidates,confirmed_causes=[],execution_authorized=False,
  limitations=['No numeric real-game diagnosis probability has been fitted.','Caller-attributed reports do not establish local performance or causality.','Absent facts remain unknown.'])
