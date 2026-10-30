"""Read-only supplied-log checks. No learned root cause or repair selection."""
import re
from bridge import prior4,evidence,digest
previous=prior4('diagnosis')
def inspect(case):
 # Keep incompatible sources available as analogies in the caller's evidence
 # index; the old diagnostic graph is never allowed to make them local facts.
 result=previous.diagnose(dict(case,evidence=[],candidate_actions=[]));log=case.get('log','');extra=[]
 patterns=[('undefined-library-symbol',r'(?im)^.*(?:err:module:load_builtin_dll|error)[^\n]*?(?:([A-Za-z0-9_+.-]+\.so(?:\.\d+)*)[^\n]*?:\s*)?undefined symbol:\s*([A-Za-z0-9_]+)[^\n]*','verify-component-symbol-versions','Inspect the loaded library paths, architecture and required/exported symbol versions. A missing symbol does not identify a package that is safe to install.','https://www.gnu.org/software/libc/manual/html_node/Dynamic-Linker-Diagnostics.html'),('audio-format-initialization',r'(?im)^.*err:(?:pulse:pulse_spec_from_waveformat[^\n]*Invalid (?:channel mask|format)|dsound:DSOUND_ReopenDevice[^\n]*Initialize failed)[^\n]*','verify-audio-format-route','Inspect the actual audio backend/device, channel layout and first initialization error in the supplied execution. Do not infer a driver fix or install a library from the error alone.','https://github.com/wine-mirror/wine/tree/master/dlls/winepulse.drv')]
 for kind,pattern,check,question,url in patterns:
  for m in list(re.finditer(pattern,log))[:8]:
   quote=evidence(log,*m.span());record=dict(kind='supplied-log-observation',predicate=kind,quote=quote['text'],span=quote['span'],source_sha256=digest(log),technical_source=url,local_runtime_verified=False);extra.append(record)
   if not any(c['id']==check for c in result['next_checks']):result['next_checks'].insert(0,dict(id=check,question=question,read_only=True,requires_execution=False,source=url,evidence_span=quote['span']))
   if not any(h['id']==kind for h in result['hypotheses']):result['hypotheses'].append(dict(id=kind,statement='The supplied log reports '+kind.replace('-',' ')+'.',status='investigable-observation',supports=[record],contradictions=[],confirmed=False,limitations=['A literal error family is not a confirmed incident cause.','The log is supplied content, not independently observed local telemetry.']))
 if extra:
  result['observations']+=extra;result['next_checks']=[c for c in result['next_checks'] if c['id']!='collect-relevant-log']
 done=case.get('checked_items',[])
 if 'collect-relevant-log' in done and not result['observations']:
  missing=[k for k in ['gpu','driver','proton'] if not case.get('context',{}).get(k)]
  if missing:result['next_checks'].append(dict(id='identify-active-environment',question='Identify the GPU actually used, driver and Proton build for this supplied incident. Missing: '+', '.join(missing)+'. Detected hardware alone is not proof of runtime use.',read_only=True,requires_execution=False))
 result['next_checks']=[c for c in result['next_checks'] if c['id'] not in done];result['next_check']=result['next_checks'][0] if result['next_checks'] else None
 result['confirmed_causes']=[];result['cause_probability']=None;return result
