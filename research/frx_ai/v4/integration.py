"""Post-evaluation CLI restart/isolation experiment, explicitly synthetic context.

Uses no game files, Steam settings, public report outcomes or real benchmarks.
"""
import argparse,json,subprocess,sys,tempfile
from pathlib import Path
from data import write

HERE=Path(__file__).resolve().parent

def run(root,old):
 context=dict(app_id='200260',gpu='synthetic-nvidia',driver='synthetic-driver-1',proton='synthetic-proton',game_version='synthetic-build-1',technology='synthetic-api',configuration={'synthetic':1})
 action=dict(kind='synthetic-research-action',target='fixture',direction='enable')
 attempt=dict(id='synthetic-attempt-A',context=context,action=action,outcome='not-resolved')
 proposal=dict(context=context,action=action,url='https://github.com/ValveSoftware/Proton#runtime-config-options')
 base=dict(origin='synthetic-isolated-memory-experiment',context=context,symptom='synthetic crash',candidate_actions=[proposal],evidence=[])
 with tempfile.TemporaryDirectory(prefix='frx-ai-v4-memory-') as memory:
  def request(case):
   raw=subprocess.check_output([sys.executable,str(HERE/'run.py'),str(root),str(old),'--memory',memory],input=(json.dumps(case)+'\n').encode(),timeout=45)
   result=json.loads(raw)
   assert result['execution_authorized'] is False and not result['confirmed_causes']
   return result
  first=request(dict(base,attempts=[attempt]));again=request(base)
  assert len(again['attempts'])==1 and again['candidate_actions'][0]['status']=='previously-failed-same-context'
  checks=[dict(scenario='restart-same-context',retained=len(again['attempts']),candidate=again['candidate_actions'][0]['status'])]
  for field,value in [('app_id','99999'),('gpu','synthetic-amd'),('driver','synthetic-driver-2'),('game_version','synthetic-build-2')]:
   other=dict(context,**{field:value});result=request(dict(base,context=other,candidate_actions=[dict(proposal,context=other)]))
   assert not result['attempts'];checks.append(dict(scenario='different-'+field,retained=0))
  result=request(dict(base,context=dict(context,game_version='synthetic-build-2'),attempts=[attempt],candidate_actions=[]))
  assert not result['attempts'] and result['context_intake_rejections'];checks.append(dict(scenario='explicit-mismatched-attempt',retained=0,rejected=True))
 result=dict(origin='synthetic research contract; not real telemetry or diagnostic accuracy',independent_process_per_request=True,checks=checks,steam_modified=False,execution_authorized=False)
 write(root/'integration.json',result);print(json.dumps(result,indent=2))

if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('root',type=Path);p.add_argument('old',type=Path);a=p.parse_args();run(a.root,a.old)
