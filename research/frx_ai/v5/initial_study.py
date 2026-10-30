"""Development-only structural controls; final reservation is never loaded."""
import argparse,json,time
from pathlib import Path
from common import read,write,jsonl,prior4
from syntax import Syntax
from syntax_control import extract
from assess import evaluate

def run(root,old4):
 rows=read(root/'dev-known.jsonl');parser=Syntax(root);pred=[];start=time.perf_counter()
 for i,row in enumerate(rows):
  parsed=parser.parse(row['text']);events=extract(row['text'],parsed);pred.append(dict(events=events,observations=[]))
  if i%10==0:print('Parsed development',i,flush=True)
 baseline=prior4('native').NativeEvents(old4/'runtime-events').predict(rows)
 lexical=[dict(events=prior4('proof').lexical(r['text']),observations=[]) for r in rows]
 results={name:evaluate(rows,p) for name,p in [('v4',baseline),('v4-lexical',lexical),('syntax-initial',pred)]};write(root/'initial-development.json',results);write(root/'initial-predictions.json',pred);write(root/'initial-timing.json',dict(seconds=time.perf_counter()-start))
 for name,r in results.items():print(name,{k:v for k,v in r.items() if k not in ['details']},flush=True)

if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('root',type=Path);p.add_argument('old4',type=Path);a=p.parse_args();run(a.root,a.old4)
