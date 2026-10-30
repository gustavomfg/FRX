"""Comparable source/event metrics; identities unknown do not become successes."""
from common import prior4
legacy=prior4('metrics')

def evaluate(rows,predictions):
 result=legacy.evaluate(rows,predictions,strict=True)
 gold_performed=sum(e['status']=='performed' for r in rows for e in r['events'])
 gold_relations=sum(o['outcome']!='unknown' and bool(o['event_indices']) for r in rows for o in r.get('observations',[]))
 result.update(gold_performed=gold_performed,performed_recall=result['performed_correct']/gold_performed if gold_performed else None,gold_attributed_observations=gold_relations,result_recall=result['result_correct']/gold_relations if gold_relations else None)
 return result
