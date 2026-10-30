"""Post-evaluation presentation guard; does not improve extraction metrics.

Thread AppID is not an event AppID. The blinded evaluation exposed cross-game
references within a valid source thread. Until event coreference is validated,
all reconstructed community events remain unassigned to a game/environment.
"""
import argparse,json,sys
from pathlib import Path
from lab import Intelligence

def guarded(engine,case):
 result=engine.analyze(case)
 for report in result['event_reports']:
  report['source_thread_app_id']=report.pop('app_id',None)
  report['event_app_id']=None
  report['event_environment_verified']=False
  report['scope']='Unresolved event attribution; a thread may discuss multiple games. Not a current-game result.'
  for event in report['events']:event['game_attribution']='unverified';event['environment_attribution']='unverified'
 result['post_evaluation_scope_guard']=True
 result['limitations'].append('Per-event game/coreference attribution is unresolved; thread identity never proves event identity. This guard was added after the final evaluation and has no new accuracy claim.')
 return result
def main():
 p=argparse.ArgumentParser();p.add_argument('root',type=Path);p.add_argument('old',type=Path);p.add_argument('--memory',type=Path);a=p.parse_args();engine=Intelligence(a.root,a.old,a.memory)
 for line in sys.stdin:
  try:
   if len(line)>300000:raise ValueError('request size bound')
   out=guarded(engine,json.loads(line))
  except (ValueError,TypeError,KeyError) as e:out=dict(status='invalid-request',error=str(e),execution_authorized=False)
  print(json.dumps(out,ensure_ascii=False),flush=True)
if __name__=='__main__':main()
