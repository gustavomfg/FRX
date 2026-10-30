"""EXPLORATORY retrieval audit, labeled after outcome-model freeze. Not an independent acceptance test."""
from experiment import load
from encoder import Encoder
from core import clean
import numpy as np,json,pathlib,re,sys
QUERIES={'startup':'The game fails to launch or does not start.','crash':'The game crashes or freezes while playing.','performance':'Low frame rate, poor performance and stuttering.','graphics':'Broken graphics, distorted geometry, flickering or missing textures.','controls':'Mouse, keyboard or controller input does not work.','dependency':'A required library or software component cannot be installed or is missing.','audio':'Audio crackling, missing sound or distorted voices.'}
def main(root):
 rows=load(root/'final.jsonl');topics=json.loads((root/'retrieval-topics.json').read_text());unique={}
 for r in rows:
  if r['url'] not in unique:unique[r['url']]={'row':r,'topics':set()}
  unique[r['url']]['topics'].update(topics[r['id']])
 samples=list(unique.values());encoder=Encoder(root/'encoder');texts=[clean(s['row']['text']) for s in samples];emb=encoder.encode(texts);qemb=encoder.encode(list(QUERIES.values()));result=[]
 for app in sorted({s['row']['app_id'] for s in samples}):
  ids=[i for i,s in enumerate(samples) if s['row']['app_id']==app]
  for qi,(topic,query) in enumerate(QUERIES.items()):
   relevant={i for i in ids if topic in samples[i]['topics']}
   if not relevant:continue
   words=set(re.findall(r'\w+',query.lower()));lexical={i:len(words&set(re.findall(r'\w+',texts[i].lower()))) for i in ids};semantic={i:float(emb[i]@qemb[qi]) for i in ids};one={'app_id':app,'topic':topic,'relevant':len(relevant)}
   for name,scores in [('lexical',lexical),('semantic',semantic)]:
    order=sorted(ids,key=lambda i:(-scores[i],samples[i]['row']['url']))[:3];one[name]={'p_at_3':sum(i in relevant for i in order)/len(order),'top_urls':[samples[i]['row']['url'] for i in order]}
   result.append(one)
 summary={'status':'exploratory-not-held-out-acceptance','unique_comments':len(samples),'queries':len(result),'lexical_p3':float(np.mean([r['lexical']['p_at_3'] for r in result])),'semantic_p3':float(np.mean([r['semantic']['p_at_3'] for r in result])),'details':result}
 (root/'experiments/retrieval-audit.json').write_text(json.dumps(summary,indent=2));print({k:v for k,v in summary.items() if k!='details'})
if __name__=='__main__':main(pathlib.Path(sys.argv[1]))
