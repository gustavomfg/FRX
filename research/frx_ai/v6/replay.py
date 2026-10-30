"""Immutable incident snapshots; expected milestones never enter model input."""
import argparse,json,re,copy,tempfile
from pathlib import Path
from bridge import read,write,jsonl,prior4
from investigate import Investigator
from incidents import Replay
def make(root):
 rows=read(root/'development-gold-v2.jsonl');cases=[];labels=[]
 for index,identity,check in [(21,'historical-SDL-symbol','verify-component-symbol-versions'),(47,'historical-audio-format','verify-audio-format-route')]:
  r=rows[index];lines=[x for x in r['text'].splitlines() if 'err:' in x or 'undefined symbol:' in x];log='\n'.join(lines)
  case=dict(context=dict(app_id=r['app_id']),symptom='The game fails during initialization.',origin='historical-public-log-view; not-local-telemetry',evidence=[])
  cases.append(dict(id=identity,input=case,tool_cache={'collect-relevant-log':dict(kind='log',text=log)},source=dict(url=r['url'],source_sha256=r['source_sha256'])));labels.append(dict(id=identity,checks=['collect-relevant-log',check],unconfirmed_cause=True))
 r=rows[36];cases.append(dict(id='historical-unknown',input=dict(context=dict(app_id=r['app_id']),symptom='The game stopped working after a system update.',origin='historical-symptom-only-view',evidence=[]),tool_cache={'collect-relevant-log':dict(kind='log',text='return code 13568; no first failure or active environment supplied')},source=dict(url=r['url'],source_sha256=r['source_sha256'])));labels.append(dict(id='historical-unknown',checks=['collect-relevant-log','identify-proton'],unconfirmed_cause=True))
 ctx=dict(app_id='200260',gpu='fixture-amd',gpu_vendor='AMD',driver='fixture-driver',proton='fixture-proton',game_version='fixture-build',configuration='fixture-config',technology='fixture-tech');url='https://github.com/ValveSoftware/Proton/issues/1';action=dict(kind='env',target='OPTION_FIXTURE',direction='enable')
 failed=dict(context=ctx,symptom='fixture crash',origin='synthetic-policy-control',attempts=[dict(id='fixture-trial',context=ctx,action=action,outcome='not-resolved')],candidate_actions=[dict(context=ctx,action=action,url=url)],evidence=[])
 cases.append(dict(id='failed-attempt',input=failed,tool_cache={}));labels.append(dict(id='failed-attempt',checks=['collect-relevant-log'],candidate='previously-failed-same-exact-context',unconfirmed_cause=True))
 incompatible=dict(context=ctx,symptom='fixture crash',origin='synthetic-policy-control',candidate_actions=[dict(context=ctx,requirements=dict(gpu_vendor='NVIDIA'),action=action,url=url)],evidence=[dict(app_id='200260',context=dict(ctx,gpu='fixture-nvidia',gpu_vendor='NVIDIA'),url=url,text='I installed a component and the game worked.')]);cases.append(dict(id='hardware-analogy',input=incompatible,tool_cache={}));labels.append(dict(id='hardware-analogy',checks=['collect-relevant-log'],candidate='context-incompatible',analogy_retained=True,unconfirmed_cause=True))
 library=dict(context=ctx,symptom='fixture startup failure',log='0001:err:module:import_dll Library fixture.dll (which is needed by fixture.exe) not found',origin='synthetic-log-control',evidence=[]);cases.append(dict(id='missing-library',input=library,tool_cache={}));labels.append(dict(id='missing-library',checks=['origin:fixture.dll'],unconfirmed_cause=True))
 cases.append(dict(id='gpu-not-game-technology',input=dict(context=dict(ctx,gpu='RTX4060'),symptom='fixture crash',origin='synthetic-policy-control',evidence=[]),tool_cache={}));labels.append(dict(id='gpu-not-game-technology',checks=['collect-relevant-log'],no_physx_inference=True,unconfirmed_cause=True))
 jsonl(root/'replay-inputs.jsonl',cases);write(root/'replay-labels.json',labels);return cases,labels
def run(root,old):
 dest=root/'diagnostic-replay.json'
 if dest.exists():raise ValueError('preserve first replay')
 cases,labels=make(root);new=Investigator(root,old,verify=False);oldinspect=prior4('diagnosis').diagnose;records=[]
 for snap,label in zip(cases,labels):
  variants={}
  for name,fn in [('v5',oldinspect),('v6',new.analyze)]:
   replay=Replay(snap);steps=[]
   for expected in label['checks']:
    result=fn(replay.state);checks=[c['id'] for c in result['next_checks']];steps.append(dict(expected=expected,top1=checks[:1]==[expected],top3=expected in checks[:3],got=checks[:3],unconfirmed=not result['confirmed_causes'],evidence_closure=any(c.get('evidence_span') is not None for c in result['next_checks']) or bool(result['observations'])))
    replay.reveal(checks[0] if checks else '')
   candidate=any(c['status']==label.get('candidate') for c in result['candidate_actions']) if label.get('candidate') else None;retained=bool(result.get('ranked_evidence')) if label.get('analogy_retained') else None;variants[name]=dict(steps=steps,candidate_contract=candidate,analogy_retained=retained,confirmed_causes=result['confirmed_causes'])
  records.append(dict(id=snap['id'],kind=snap['input']['origin'],variants=variants))
 summary={name:dict(steps=sum(len(r['variants'][name]['steps']) for r in records),top1=sum(s['top1'] for r in records for s in r['variants'][name]['steps']),top3=sum(s['top3'] for r in records for s in r['variants'][name]['steps'])) for name in ['v5','v6']}
 write(dest,dict(status='known historical public log views plus explicit controls; not independent real-game diagnostic validation',records=records,summary=summary,learned_root_cause_model=False,probabilities_fitted=False,limitations=['Expected checks are provisional agent milestones.','SDL/audio templates were developed from these known reports.','No real game was executed or fixed.']));print(summary)
if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('root',type=Path);p.add_argument('old',type=Path);a=p.parse_args();run(a.root,a.old)
