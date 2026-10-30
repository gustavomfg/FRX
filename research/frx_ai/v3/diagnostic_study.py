"""Historical demonstrations and deterministic contract controls, not RCA gold."""
import argparse
from pathlib import Path
import json
from investigation import inspect,compatible
from common import write

def run(root):
 demonstrations=[]
 for source in json.loads((root/'historical-proposed.json').read_text()):
  result=inspect(dict(context={'app_id':source['app_id']},log=source['text']))
  demonstrations.append(dict(app_id=source['app_id'],url=source['url'],source_sha256=result['observations'][0]['source_sha256'],
    observations=len(result['observations']),hypotheses=len(result['hypotheses']),
    predicates=sorted({o['predicate'] for o in result['observations']}),
    components=sorted({o['component'] for o in result['observations'] if o['component']}),
    confirmed_causes=result['confirmed_causes'],next_checks=[c['id'] for c in result['next_checks']]))
 c=dict(app_id='200260',gpu='NVIDIA RTX 4060',driver='fixture-driver',proton='fixture-proton')
 variants=[dict(c),dict(c,app_id='12345'),dict(c,gpu='AMD'),dict(c,driver='other'),dict(c,proton='other'),{'app_id':'200260'}]
 expected=['compatible','incompatible','incompatible','incompatible','incompatible','unknown']
 actual=[compatible(c,v) for v in variants]
 write(root/'diagnostic-study.json',dict(historical_demonstrations=demonstrations,
   historical_scope='Parser-selected public log reports, manually inspected; demonstrations, NOT an independent sensitivity/precision test.',
   synthetic_context_contract=dict(cases=len(expected),expected=expected,observed=actual,passed=expected==actual),
   learned_diagnosis_validated_on_real_games=False))
 print('Historical source demonstrations:',len(demonstrations),'context contract:',actual==expected)

if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('root',type=Path);run(p.parse_args().root)
