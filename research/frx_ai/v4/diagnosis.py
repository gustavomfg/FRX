"""Evidence-limited next-check planning; no fitted real-game probabilities."""
import importlib.util
from pathlib import Path
import re
import hashlib

V3=Path(__file__).resolve().parents[1]/'v3'
spec=importlib.util.spec_from_file_location('frx_v3_investigation',V3/'investigation.py');v3=importlib.util.module_from_spec(spec);spec.loader.exec_module(v3)
WINE='https://github.com/wine-mirror/wine/blob/master/dlls/ntdll/loader.c'
VULKAN='https://registry.khronos.org/vulkan/specs/latest/man/html/VkResult.html'

def diagnose(case):
 result=v3.inspect(case);log=case.get('log','');extra=[]
 patterns=[
  ('unimplemented-function',r'Unhandled exception:\s*unimplemented function\s+([A-Za-z0-9_.+-]+\.(?:dll|DLL))\.([A-Za-z0-9_@$?]+)',
   'verify-component-function-version','Check the exact DLL/function, its implementation in the installed Proton/Wine build, and documented upstream changes before proposing a file replacement.',WINE),
  ('vulkan-driver-rejected',r'\bVK_ERROR_INCOMPATIBLE_DRIVER\b','verify-active-vulkan-driver','Identify the Vulkan driver/ICD actually used by this game and compare its documented API requirements; GPU presence alone is insufficient.',VULKAN),
  ('vulkan-extension-missing',r'\bVK_ERROR_EXTENSION_NOT_PRESENT\b','identify-missing-vulkan-extension','Identify the requested extension and the runtime device/driver that rejected it; do not infer hardware incompatibility from the error name alone.',VULKAN),
 ]
 for kind,pattern,check,question,source in patterns:
  for m in re.finditer(pattern,log):
   # A bare error name in documentation/prose is not an execution log.
   line=log[log.rfind('\n',0,m.start())+1:log.find('\n',m.end()) if '\n' in log[m.end():] else len(log)]
   if kind!='unimplemented-function' and not re.search(r'(?i)\b(?:err:|error|failed|exception|vulkan:)\b',line):continue
   obs=dict(kind='supplied-log-observation',predicate=kind,quote=m[0],span=list(m.span()),source_sha256=hashlib.sha256(log.encode()).hexdigest(),technical_source=source)
   if m.lastindex and m.lastindex>=2:obs.update(component=m[1],function=m[2])
   extra.append(obs)
   result['next_checks'].insert(0,dict(id=check,question=question,read_only=True,requires_execution=False,source=source,evidence_span=list(m.span())))
   result['hypotheses'].append(dict(id=kind,statement='The supplied log reports '+kind.replace('-',' ')+'.',supports=[obs],status='investigable-observation',confirmed=False,
     limitations=['The complete incident cause is not established.','The correct component origin, architecture/version and current game context still need verification.']))
 if extra:
  result['observations']+=extra
  result['next_checks']=[x for x in result['next_checks'] if x['id']!='collect-relevant-log']
 # Questions about facts already supplied should not be repeatedly requested.
 known=case.get('checked_items',[])
 if not isinstance(known,list) or len(known)>64:raise ValueError('bounded checked-item identities required')
 seen=set();checks=[]
 for check in result['next_checks']:
  if check['id'] in seen or check['id'] in known:continue
  seen.add(check['id']);checks.append(check)
 result['next_checks']=checks;result['next_check']=checks[0] if checks else None
 # Thread groups prevent repeated comments being presented as confirmations.
 groups={}
 for source in result['sources']:
  thread=source['url'].split('#')[0];groups.setdefault(thread,[]).append(source['index'])
 result['evidence_thread_groups']=[dict(thread=k,source_indices=v,independence_established=False) for k,v in groups.items()]
 result['cause_probability']=None;result['confirmed_causes']=[]
 return result
