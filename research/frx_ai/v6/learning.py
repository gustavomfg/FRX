"""Train claim verifiers, with explicit real text versus constructed claims.

Old sparse annotations are positive evidence, not exhaustive negative labels.
Only explicit wrong complete-owner sets or deliberately inverted claim polarity
produce training negatives. Every such construction is marked in provenance.
"""
import argparse,json,copy,time,math
from pathlib import Path
import numpy as np
from sklearn.linear_model import LogisticRegression
from sklearn.tree import DecisionTreeClassifier
from bridge import read,write,jsonl,digest,evidence,CompactSyntax,immutable,retriever
from grounding import deconstruct,identity,masked
from pairs import candidates,verify_literal,FEATURES
from engine import Interpreter
from evaluate import evaluate

def synthetic():
 rows=[]
 targets=['OPTION_ALPHA','the runtime','the configuration file','the Proton version','the graphics setting','the legacy library']
 for j,target in enumerate(targets):
  for v in range(6):
   if v==0:
    text=f'I installed {target} and the game worked.';events=[('installed','performed','install')];obs=[('the game worked','improved',[0])]
   elif v==1:
    text=f'I disabled OPTION_BETA and installed {target}. Then the game worked.';events=[('disabled','performed','disable'),('installed','performed','install')];obs=[('the game worked','improved',[0,1])]
   elif v==2:
    text=f'I tried OPTION_BETA, but the game still crashed. Then I installed {target} and it worked.';events=[('tried','performed','configure'),('installed','performed','install')];obs=[('the game still crashed','persistent',[0]),('it worked','improved',[1])]
   elif v==3:
    text=f'I installed {target}, but it did not fix the crash.';events=[('installed','performed','install')];obs=[('did not fix the crash','persistent',[0])]
   elif v==4:
    text=f'Before I installed {target}, the game worked. It still crashes now.';events=[('installed','performed','install')];obs=[('the game worked','improved',[]),('It still crashes now','persistent',[0])]
   else:
    text=f'I enabled OPTION_BETA and replaced {target}. The game worked, but performance got worse.';events=[('enabled','performed','enable'),('replaced','performed','replace')];obs=[('The game worked','improved',[0,1]),('performance got worse','regression',[0,1])]
   es=[]
   for trigger,status,direction in events:
    a=text.index(trigger);es.append(dict(trigger=[a,a+len(trigger)],status=status,direction=direction))
   os=[]
   for quote,outcome,owners in obs:
    a=text.index(quote);os.append(dict(span=[a,a+len(quote)],outcome=outcome,event_indices=owners,joint=len(owners)>1))
   rows.append(dict(id=f'synthetic-{j}-{v}',app_id='synthetic',text=text,events=es,observations=os,origin='explicit-synthetic-training; not a real report',annotation_complete=True))
 return rows

def oracle_bundle(row,parser,catalog):
 text=row['text'];b=deconstruct(row,parser,catalog);parsed=b['parsed'];words=parsed['words'];events=[];observed=[]
 for g in row['events']:
  a,z=g['trigger'];hits=[w for w in words if min(z,w['span'][1])>max(a,w['span'][0])]
  if not hits:raise ValueError('gold event token absent')
  w=hits[0];events.append(dict(trigger=evidence(text,a,z),status=g['status'],direction=g['direction'],node=w['i'],sentence=w['sentence'],identity=identity(text,parsed,w['i'],row,b['game_mentions'])))
 for g in row.get('observations',[]):
  a,z=g['span'];hits=[w for w in words if min(z,w['span'][1])>max(a,w['span'][0])]
  if not hits:continue
  w=next((w for w in hits if w['pos']=='VERB'),hits[0]);observed.append(dict(evidence=evidence(text,a,z),outcome=g['outcome'],node=w['i'],sentence=w['sentence'],identity=identity(text,parsed,w['i'],row,b['game_mentions']),gold_owners=g['event_indices']))
 return dict(b,events=events,observations=observed)

def pair_text(row,b,p):
 text=row['text'];obs=b['observations'][p['observation']];es=[b['events'][i] for i in p['event_indices']];marks=[]
 for e in es:marks.append((e['trigger']['span'][0],'[ACTION] '));marks.append((e['trigger']['span'][1],' [/ACTION]'))
 marks.append((obs['evidence']['span'][0],'[RESULT] '));marks.append((obs['evidence']['span'][1],' [/RESULT]'))
 left=max(0,min(x[0] for x in marks)-110);right=min(len(text),max(x[0] for x in marks)+110);parts=[];cursor=left
 for at,tag in sorted(marks,key=lambda x:x[0]):
  parts.extend([text[cursor:at],tag]);cursor=at
 parts.append(text[cursor:right]);claim='Reported outcome='+obs['outcome']+'; complete action group='+','.join(e['trigger']['text'] for e in es)+'; game='+str(es[0]['identity']['game_name'])+'; actor='+es[0]['identity']['actor_kind']
 return claim+' [SEP] '+''.join(parts)

def examples(rows,parser,catalog):
 out=[];miss=[]
 for r in rows:
  b=oracle_bundle(r,parser,catalog);pool=candidates(r['text'],b)
  for oi,o in enumerate(b['observations']):
   if not o['gold_owners'] or o['outcome']=='unknown':continue
   choices=[p for p in pool if p['observation']==oi];positive=[p for p in choices if sorted(p['event_indices'])==sorted(o['gold_owners'])]
   if not positive:miss.append(dict(id=r['id'],observation=oi,reason='complete owner group not proposed even with supplied spans'));continue
   for p in choices:
    correct=p in positive;label=1 if correct else 0;origin='annotated-reported-group' if correct else 'constructed-alternative-owner-claim'
    out.append(dict(id=r['id'],app_id=r['app_id'],x=p['features'],text=pair_text(r,b,p),y=label,origin=origin,source_origin=r.get('origin','real-public-agent-provisional'),url=r.get('url')))
   # Polarity inversion is a synthetic claim about unchanged real evidence,
   # never a relabeling of the source's actual outcome.
   for p in positive:
    changed=copy.deepcopy(b);changed['observations'][oi]['outcome']='persistent' if o['outcome']=='improved' else 'improved';x=list(p['features']);x[13:16]=[float(changed['observations'][oi]['outcome']=='persistent'),float(changed['observations'][oi]['outcome']=='improved'),0.]
    out.append(dict(id=r['id'],app_id=r['app_id'],x=x,text=pair_text(r,changed,p),y=2,origin='constructed-contradicted-polarity-claim',source_origin=r.get('origin','real-public-agent-provisional'),url=r.get('url')))
 return out,miss

def linear(est):return dict(kind='linear',classes=est.classes_.tolist(),coefficients=est.coef_.tolist(),intercept=est.intercept_.tolist())
def predict_linear(model,x):
 z=np.asarray(x)@np.array(model['coefficients']).T+np.array(model['intercept']);z-=z.max(axis=-1,keepdims=True);p=np.exp(z);return p/p.sum(axis=-1,keepdims=True)
def choose(root,rows,parser,catalog,verifier,name,thresholds):
 readouts=[];best=None;score=-1
 for threshold in thresholds:
  engine=Interpreter(parser,catalog,verifier,threshold,mode=name);pred=[engine.analyze(r) for r in rows];m=evaluate(rows,pred);precision=m['relation_precision'] or 0;recall=m['relation_recall'] or 0;f=.0 if not precision+recall else 1.25*precision*recall/(.25*precision+recall);eligible=m['relation_correct']>=8 and precision>=.8 and recall>=.25 and m['false_success']<=1;value=(10 if eligible else 0)+f-2*m['false_success']/max(1,m['relations']);rec=dict(threshold=threshold,metrics=m,eligible=eligible);readouts.append(rec)
  if value>score:score=value;best=dict(rec,name=name);winning=pred
 return dict(comparisons=readouts,selection=best),winning

def run(root,old):
 immutable(root);dest=root/'learned-development.json'
 if dest.exists():raise ValueError('preserve learned study')
 start=time.perf_counter();parser=CompactSyntax(root);catalog=json.loads((root/'catalog.json').read_text());old5=Path(str(old).replace('v2-20261029','v5-20261030'));train=read(old5/'train-known.jsonl')+read(old5/'extra-audited.jsonl');real,missing=examples(train,parser,catalog);synth,smissing=examples(synthetic(),parser,catalog);jsonl(root/'training-pairs.jsonl',real+synth);write(root/'pair-data-audit.json',dict(real_text_claims=len(real),synthetic_text_claims=len(synth),real_sources=len(train),real_classes=np.bincount([x['y'] for x in real],minlength=3).tolist(),synthetic_classes=np.bincount([x['y'] for x in synth],minlength=3).tolist(),unproposable=missing,synthetic_unproposable=smissing,dataset_sha256=digest(json.dumps(real+synth,sort_keys=True)),labels='agent-provisional originals plus explicitly constructed claims; not human truth'))
 dev=read(root/'development-gold-v2.jsonl');out=root/'verifiers';out.mkdir(exist_ok=True);results={};encoder=None
 for name,added,semantic in [('L1-structured-real-text',False,False),('L2-structured-contrast',True,False),('L3-semantic-contrast',True,True)]:
  data=real+(synth if added else []);x=np.array([r['x'] for r in data]);y=np.array([r['y'] for r in data]);before=time.perf_counter()
  if semantic:
   encoder=retriever(old);x=np.column_stack([x,encoder.encode([r['text'] for r in data])])
  est=LogisticRegression(C=.5,class_weight='balanced',max_iter=1500,random_state=606).fit(x,y);model=linear(est);model.update(semantic=semantic,features=FEATURES,training_sha256=digest(json.dumps(data,sort_keys=True)))
  def verifier(row,b,pool):
   if not pool:return []
   z=np.array([p['features'] for p in pool])
   if semantic:z=np.column_stack([z,encoder.encode([pair_text(row,b,p) for p in pool])])
   return predict_linear(model,z)[:,model['classes'].index(1)]
  rec,pred=choose(root,dev,parser,catalog,verifier,name,[.3,.4,.5,.6,.7,.8]);rec.update(seconds=time.perf_counter()-before,training_examples=len(data));results[name]=rec;write(out/(name+'.json'),model);write(root/(name+'-predictions.json'),pred);print(name,{k:rec['selection']['metrics'][k] for k in ['relation_correct','relations','false_success']},rec['selection']['eligible'],flush=True)
 write(dest,dict(models=results,seconds=time.perf_counter()-start))
if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('root',type=Path);p.add_argument('old',type=Path);a=p.parse_args();run(a.root,a.old)
