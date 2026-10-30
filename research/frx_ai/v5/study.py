"""Versioned development controls, leaving the final reservation unread."""
import argparse,time
from pathlib import Path
from common import read,write,prior4
from syntax import Syntax
from events import extract
from relations import reconstruct
from assess import evaluate

def run(root,name,backend="stanza"):
 if (root/'frozen.json').exists():raise ValueError('sealed; no selection changes')
 dest=root/(name+'-results.json')
 if dest.exists():raise ValueError('preserve experiment record')
 rows=read(root/'dev-known.jsonl');parser=Syntax(root) if backend=='stanza' else __import__('compact').CompactSyntax(root);pred=[];same=[];start=time.perf_counter()
 for row in rows:
  parsed=parser.parse(row['text']);events=extract(row['text'],parsed);pred.append(reconstruct(row['text'],parsed,events));same.append(reconstruct(row['text'],parsed,events,'same-clause'))
 results={'syntax-reconstruction':evaluate(rows,pred),'without-anaphora':evaluate(rows,same)}
 write(dest,dict(name=name,backend=backend,seconds=time.perf_counter()-start,metrics=results));write(root/(name+'-predictions.json'),pred)
 for key,m in results.items():print(key,{k:v for k,v in m.items() if k!='details'})

if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('root',type=Path);p.add_argument('name');p.add_argument('--backend',default='stanza',choices=['stanza','compact']);a=p.parse_args();run(a.root,a.name,a.backend)
