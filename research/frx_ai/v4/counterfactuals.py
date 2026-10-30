"""Post-test synthetic audit of literal verifier, with supplied operation spans.

Not independent real-language accuracy and not additional training examples.
Reports failures without modifying the frozen system to fit these cases.
"""
import argparse,json
from pathlib import Path
from data import literal,write
from proof import verify,attach_observations

CASES=[
 ('I enabled OPTION_Z and the crash stopped.','enabled','performed','enable'),
 ('Enabling OPTION_Z caused a crash.','Enabling','performed','enable'),
 ('Disabling OPTION_Z fixed the crash.','Disabling','performed','disable'),
 ('Try enabling OPTION_Z.','enabling','suggested','enable'),
 ("I haven't tried OPTION_Z.",'tried','negated','configure'),
 ('OPTION_Z was already enabled before this incident.','enabled','preexisting','enable'),
 ('I disabled OPTION_Z and installed the missing library.','disabled','performed','disable'),
 ('I disabled OPTION_Z and installed the missing library.','installed','performed','install'),
]

def run(root):
 records=[]
 for text,word,status,direction in CASES:
  result=verify(text,literal(text,word));records.append(dict(origin='synthetic supplied-trigger stress fixture; not independent final data',text=text,trigger=word,expected_status=status,expected_direction=direction,actual_status=result['status'],actual_direction=result['direction'],status_correct=result['status']==status,direction_correct=result['direction']==direction,target_identity=result['target_identity']))
 result=dict(posthoc=True,model_modified=False,supplied_trigger_control=True,n=len(records),status_correct=sum(r['status_correct'] for r in records),direction_correct=sum(r['direction_correct'] for r in records),cases=records)
 write(root/'counterfactuals.json',result);print(json.dumps(result,indent=2))

if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('root',type=Path);run(p.parse_args().root)
