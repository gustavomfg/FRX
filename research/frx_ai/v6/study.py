"""Known/new development only; no acceptance-body access."""
import argparse,json,time
from pathlib import Path
from bridge import read,write,immutable,CompactSyntax,Syntax,prior_events,prior_reconstruct
from evaluate import evaluate
from engine import Interpreter
def run(root,name='initial',gold='development-gold.jsonl'):
 immutable(root);dest=root/('development-'+name+'.json')
 if dest.exists():raise ValueError('preserve initial comparison')
 rows=read(root/gold);catalog=json.loads((root/'catalog.json').read_text());small=CompactSyntax(root);large=Syntax(root);results={};preds={};start=time.perf_counter()
 for name,parser in [('v5-stanza',large),('v5-compact',small)]:
  preds[name]=[prior_reconstruct(r['text'],p,prior_events(r['text'],p)) for r in rows for p in [parser.parse(r['text'])]]
 for guard in [False,True]:
  for threshold in [.5,1.5,2.5]:
   engine=Interpreter(small,catalog,threshold=threshold,guard=guard);name=f'v6-structural-guard{guard}-t{threshold}';preds[name]=[engine.analyze(r) for r in rows]
 results={k:evaluate(rows,p) for k,p in preds.items()};write(dest,dict(results=results,seconds=time.perf_counter()-start,gold=gold));write(root/('development-'+name+'-predictions.json'),preds)
 for k,m in results.items():print(k,{x:m[x] for x in ['performed_correct','performed','gold_performed','false_performed','relation_correct','relations','gold_relations','false_success','game_correct','game_claims','gold_game_known','actor_correct','actor_claims']},flush=True)
if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('root',type=Path);p.add_argument('--name',default='initial');p.add_argument('--gold',default='development-gold.jsonl');a=p.parse_args();run(a.root,a.name,a.gold)
