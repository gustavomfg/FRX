"""Post-evaluation intake guard around the unchanged frozen prototype.

This engineering guard is not an additional trained model or an independently
measured improvement in language accuracy. It protects explicitly supplied
history/candidate context as well as the existing persisted memory namespaces.
"""
import argparse,json,sys
from pathlib import Path
from prototype import Intelligence

FIELDS=['app_id','gpu','driver','proton','game_version','technology','configuration']

def matched(current,other):
 if not isinstance(other,dict):return False,'missing-environment'
 for field in FIELDS:
  a=current.get(field);b=other.get(field)
  if field in ['app_id','gpu','driver','proton'] and (not a or not b):return False,'incomplete-environment'
  # Optional metadata must not silently match a known value against unknown.
  if a!=b:return False,'different-or-unknown-'+field
 return True,None

def intake(case):
 if not isinstance(case,dict) or not isinstance(case.get('context'),dict):raise ValueError('case context required')
 out=dict(case);rejected=[]
 for key in ['attempts','candidate_actions']:
  values=case.get(key,[])
  if not isinstance(values,list) or len(values)>64:raise ValueError('bounded context collection required')
  accepted=[]
  for i,value in enumerate(values):
   if not isinstance(value,dict):raise ValueError('structured history/candidate required')
   okay,reason=matched(case['context'],value.get('context'))
   if okay:accepted.append(value)
   else:rejected.append(dict(collection=key,index=i,reason=reason))
  out[key]=accepted
 return out,rejected

class GuardedIntelligence(Intelligence):
 def analyze(self,case):
  clean,rejected=intake(case);result=super().analyze(clean)
  result['context_intake_rejections']=rejected;result['post_evaluation_context_guard']=True
  return result

def main():
 p=argparse.ArgumentParser();p.add_argument('root',type=Path);p.add_argument('old',type=Path);p.add_argument('--memory',type=Path);p.add_argument('--experimental-relations',action='store_true');a=p.parse_args();engine=GuardedIntelligence(a.root,a.old,a.memory,a.experimental_relations)
 for line in sys.stdin:
  try:
   if len(line)>300000:raise ValueError('request size bound')
   result=engine.analyze(json.loads(line))
  except (ValueError,TypeError,KeyError) as e:result=dict(status='invalid-request',error=str(e),execution_authorized=False)
  print(json.dumps(result,ensure_ascii=False),flush=True)

if __name__=='__main__':main()
