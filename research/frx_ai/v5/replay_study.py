"""Immutable historical/control incident replay with separate milestone labels.

The language examples reuse known public demonstrations, never local observed
telemetry. Telemetry masks and environment mutations are explicit controls.
"""
import argparse,copy,json,tempfile
from pathlib import Path
from common import read,write,jsonl,prior4
from lab import Intelligence
from run import guarded
from incidents import Replay

def make(root,old4):
 demos=read(old4/'demos.jsonl');cases=[];expected=[]
 for c in demos:
  snap=dict(id=c['id'],kind=c.get('origin'),input=copy.deepcopy(c),tool_cache={})
  if c['id']=='dependency-function':
   log=snap['input'].pop('log');snap['tool_cache']['collect-relevant-log']=dict(kind='log',text=log)
   exp=dict(checks=['collect-relevant-log','verify-component-function-version'],unknown_cause=True)
  else:exp=dict(checks=['collect-relevant-log'],unknown_cause=True)
  cases.append(snap);expected.append(dict(id=c['id'],**exp))
 # Logical controls: explicitly fake environment, no claimed observed game run.
 ctx=dict(app_id='200260',gpu='fixture-nvidia',driver='fixture-driver',proton='fixture-proton',game_version='fixture-build',technology='fixture-tech',configuration='fixture-config')
 action=dict(kind='env',target='PROTON_NO_ESYNC',direction='enable');url='https://github.com/ValveSoftware/Proton/issues/1'
 c=dict(id='failed-context-memory',origin='synthetic-policy-control',context=ctx,symptom='fixture crash',evidence=[],attempts=[dict(id='fixture-A',action=action,outcome='not-resolved',context=ctx)],candidate_actions=[dict(action=action,context=ctx,url=url)])
 cases.append(dict(id=c['id'],kind=c['origin'],input=c,tool_cache={}));expected.append(dict(id=c['id'],checks=['collect-relevant-log'],candidate_status='previously-failed-same-exact-context',unknown_cause=True))
 other=copy.deepcopy(c);other['id']='hardware-mismatch';other['attempts']=[];other['candidate_actions'][0]['context']=dict(ctx,gpu='fixture-amd')
 cases.append(dict(id=other['id'],kind=other['origin'],input=other,tool_cache={}));expected.append(dict(id=other['id'],checks=['collect-relevant-log'],candidate_rejection=True,unknown_cause=True))
 # Literal loader line is an explicit synthetic telemetry control.
 missing=dict(id='missing-library-control',origin='synthetic-log-control',context=ctx,symptom='fixture startup failure',evidence=[],log='0001:err:module:import_dll Library fixture.dll (which is needed by fixture.exe) not found')
 cases.append(dict(id=missing['id'],kind=missing['origin'],input=missing,tool_cache={}));expected.append(dict(id=missing['id'],checks=['origin:fixture.dll'],unknown_cause=True))
 jsonl(root/'replay-inputs.jsonl',cases);write(root/'replay-labels.json',expected)
 return cases,expected
def run(root,old,old4):
 dest=root/'incident-replay.json'
 if dest.exists():raise ValueError('preserve replay')
 cases,expected=make(root,old4);engine=Intelligence(root,old);records=[];outputs=[]
 for snap,label in zip(cases,expected):
  state=Replay(snap);steps=[]
  for expected_check in label['checks']:
   result=guarded(engine,state.state);checks=[x['id'] for x in result['next_checks']];steps.append(dict(expected=expected_check,top1=checks[:1]==[expected_check],top3=expected_check in checks[:3],got=checks[:3],unknown_cause=not result['confirmed_causes']))
   state.reveal(checks[0] if checks else '')
  candidate_ok=None
  if label.get('candidate_status'):candidate_ok=any(x['status']==label['candidate_status'] for x in result['candidate_actions'])
  if label.get('candidate_rejection'):candidate_ok=bool(result['context_intake_rejections']) and not result['candidate_actions']
  records.append(dict(id=snap['id'],kind=snap['kind'],steps=steps,candidate_contract=candidate_ok));outputs.append(result)
 with tempfile.TemporaryDirectory(dir=root,prefix='memory-replay-') as d:
  first=next(s['input'] for s in cases if s['id']=='failed-context-memory');guarded(Intelligence(root,old,Path(d)),first)
  again=copy.deepcopy(first);again['attempts']=[];after=guarded(Intelligence(root,old,Path(d)),again)
  restart=after['candidate_actions'][0]['status']=='previously-failed-same-exact-context'
  again['context']=dict(again['context'],driver='new-fixture-driver');changed=guarded(Intelligence(root,old,Path(d)),again);isolated=not changed['attempts']
 allsteps=[s for r in records for s in r['steps']]
 write(dest,dict(status='known-source demonstrations plus controlled telemetry/context replay; not new real-game validation',records=records,steps=len(allsteps),next_check_top1=sum(s['top1'] for s in allsteps),next_check_top3=sum(s['top3'] for s in allsteps),unknown_cause_preserved=all(s['unknown_cause'] for s in allsteps),restart_retains_failed_attempt=restart,driver_change_isolated=isolated,learned_diagnosis=False,probabilities_fitted=False,limitations=['Next-check selector is reused deterministic v3/v4 log logic.','Expected checks are agent provisional and intentionally limited.','Memory suppresses an identical reported failed attempt; it does not prove another alternative works.']))
 jsonl(root/'demos-v5-output.jsonl',outputs);print(json.dumps(json.loads(dest.read_text()),indent=2))
if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('root',type=Path);p.add_argument('old',type=Path);p.add_argument('old4',type=Path);a=p.parse_args();run(a.root,a.old,a.old4)
