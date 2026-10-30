"""Bounded public source snapshots. Metadata does not establish event identity."""
import argparse,json,re,hashlib,subprocess,urllib.request,time
from pathlib import Path
from bridge import read,write,jsonl,digest,HERE

DEV=[('632360',2476),('637650',74),('412020',2414),('570940',1026),('1259420',4832),('1593500',5511)]
FINAL=[('2215430',7735),('1659420',6246),('418370',1029),('1172620',3931),('1687950',6255),('1888160',7045)]
ACTION=re.compile(r'(?i)\b(?:enabl\w*|disabl\w*|install\w*|uninstall\w*|replac\w*|remov\w*|updat\w*|upgrad\w*|downgrad\w*|switch\w*|chang\w*|cop(?:y|ied|ying)|set(?:ting)?|tried|try|trying|launch\w*|using|used|ran|restart\w*|reboot\w*|test\w*)\b')
RESULT=re.compile(r'(?i)\b(?:work\w*|fix\w*|resolv\w*|crash\w*|hang\w*|fail\w*|worse|better|stopp\w*|same|help\w*|success\w*|distort\w*|freez\w*)\b')
CONTEXT=re.compile(r'(?i)\b(?:another game|other games?|my friend|he said|she said|told me|NVIDIA.*AMD|AMD.*NVIDIA|Proton\s*[\d.]+.*Proton\s*[\d.]+|worked before|but (?:in|on)|on (?:Windows|Linux))\b|^>',re.M)
def public(root,url):
 dest=root/'public';dest.mkdir(exist_ok=True);file=dest/(digest(url)+'.json')
 if not file.exists():
  raw=subprocess.check_output(['gh','api',url],timeout=45)
  if len(raw)>8000000:raise ValueError('public page bound')
  file.write_bytes(raw)
 return json.loads(file.read_text())
def paragraphs(text):
 for m in re.finditer(r'[^\n].*?(?=\r?\n\s*\r?\n|\Z)',text,re.S):
  a,b=m.span()
  while a<b and text[a].isspace():a+=1
  while b>a and text[b-1].isspace():b-=1
  part=text[a:b]
  if 40<=len(part)<=1300 and not part.startswith(('<!--','```','![','<img','# System','## System','# Compatibility')):yield a,b
def collect(root):
 if (root/'reservation.json').exists():raise ValueError('preserve first reservation')
 groups={};meta=[];seen=set()
 for split,games in [('development',DEV),('acceptance',FINAL)]:
  rows=[]
  for app,number in games:
   issue=public(root,f'repos/ValveSoftware/Proton/issues/{number}');comments=public(root,f'repos/ValveSoftware/Proton/issues/{number}/comments?per_page=100')
   if app not in issue['title']:raise ValueError('thread identity mismatch')
   pool=[]
   for comment in comments:
    text=comment.get('body') or ''
    if len(text)>200000:continue
    choices=list(paragraphs(text))
    if not choices:continue
    a,b=max(choices,key=lambda p:(bool(CONTEXT.search(text[slice(*p)])),bool(ACTION.search(text[slice(*p)])) and bool(RESULT.search(text[slice(*p)])),len(ACTION.findall(text[slice(*p)]))))
    piece=text[a:b];key=digest(piece)
    if key in seen:continue
    pool.append(dict(item=comment,a=a,b=b,text=piece,key=digest('v6-'+str(comment['id'])),multi=bool(CONTEXT.search(piece)),rich=bool(ACTION.search(piece) and RESULT.search(piece)),request=bool(re.search(r'(?i)\b(?:please|could you|have you|can you|try|should)\b',piece))))
   pool.sort(key=lambda x:x['key']);chosen=[];counts=[('multi',2),('request',2 if split=='acceptance' else 1),('rich',6 if split=='acceptance' else 4)]
   for field,n in counts:
    options=[x for x in pool if x not in chosen and x[field]];chosen+=options[:n]
   n=12 if split=='acceptance' else 8;chosen += [x for x in pool if x not in chosen][:max(0,n-len(chosen))]
   for x in chosen:
    c=x['item'];body=c['body'];row=dict(id=split+'-'+str(len(rows)),source_thread_app_id=app,app_id=app,thread=number,title=issue['title'],url=c['html_url'],document_id=str(c['id']),reporter_id='github:'+str(c['user']['id']),reporter_login=c['user']['login'],source_sha256=digest(body),source_offset=x['a'],paragraph_length=len(x['text']),paragraph_sha256=digest(x['text']),published_at=c['created_at'],updated_at=c['updated_at'],stratum='multi' if x['multi'] else 'request' if x['request'] else 'rich' if x['rich'] else 'residual');meta.append(dict(split=split,**row));rows.append(dict(row,text=x['text'],source_text=body));seen.add(digest(x['text']))
   print(split,app,'reserved',len(chosen),'units; bodies not displayed',flush=True)
  groups[split]=rows;jsonl(root/(split+'-reserved.jsonl'),rows)
 write(root/'reservation.json',dict(policy='preregistered v6 bounded strata; one unit/comment; first100comments',records=meta))
def catalog(root):
 # Caller catalog entries must have verifiable official identity. Research
 # registry queries Steam itself; never matches approximate game names.
 apps={r['app_id'] for r in json.loads((root/'reservation.json').read_text())['records']}
 for file in [HERE.parent/'v5/snapshot/train-known.json',HERE.parent/'v5/snapshot/extra-audited.json',HERE.parent/'v5/snapshot/dev-known.json']:
  apps|={r['app_id'] for r in json.loads(file.read_text())}
 # Taxonomy only, not text or annotation labels from the forbidden v5 cohort.
 for file in HERE.parent.rglob('snapshot/*.json'):
  if file.stat().st_size>8000000:continue
  try:value=json.loads(file.read_text())
  except (ValueError,UnicodeError):continue
  def ids(x):
   if isinstance(x,dict):
    if str(x.get('app_id','')).isdigit():apps.add(str(x['app_id']))
    for v in x.values():ids(v)
   elif isinstance(x,list):
    for v in x:ids(v)
  ids(value)
 entries=[]
 for app in sorted(apps):
  file=root/'public'/('steam-'+app+'.json');url='https://store.steampowered.com/api/appdetails?appids='+app
  if not file.exists():
   with urllib.request.urlopen(url,timeout=30) as response:raw=response.read(1000001)
   if len(raw)>1000000:raise ValueError('Steam metadata bound')
   file.write_bytes(raw);time.sleep(.2)
  info=json.loads(file.read_text())[app]
  if not info.get('success'):continue
  data=info['data']
  if str(data.get('steam_appid'))!=app:raise ValueError('Steam catalog identity')
  entries.append(dict(app_id=app,name=data['name'],verified=True,source=url,sha256=digest(file.read_text()),aliases=[]))
 write(root/'catalog.json',entries);print('Official exact-name catalog entries',len(entries))
def challenge(root):
 if (root/'identity-challenge-reserved.jsonl').exists():raise ValueError('preserve first identity reservation')
 catalog=json.loads((root/'catalog.json').read_text());used={r['document_id'] for r in json.loads((root/'reservation.json').read_text())['records']};out=[]
 patterns=[]
 for c in catalog:
  words=re.findall(r'[A-Za-z0-9]+',c['name']);pattern=r'(?i)(?<!\w)'+r'[\s:™®\-–]*'.join(map(re.escape,words))+r'(?!\w)'
  if len(words)>=2:patterns.append((c['app_id'],re.compile(pattern)))
 for app,number in FINAL:
  issue=public(root,f'repos/ValveSoftware/Proton/issues/{number}');comments=public(root,f'repos/ValveSoftware/Proton/issues/{number}/comments?per_page=100');pool=[]
  for c in comments:
   if str(c['id']) in used:continue
   body=c.get('body') or ''
   for a,b in paragraphs(body):
    text=body[a:b];mentions=[g for g,p in patterns if p.search(text)]
    if mentions and ACTION.search(text):pool.append((any(g!=app for g in mentions),digest('identity-'+str(c['id'])),c,a,b));break
  pool.sort(key=lambda x:(not x[0],x[1]));seen=set()
  for other,key,c,a,b in pool:
   body=c['body'];text=body[a:b]
   if digest(text) in seen:continue
   seen.add(digest(text));out.append(dict(id='identity-'+str(len(out)),app_id=app,source_thread_app_id=app,thread=number,title=issue['title'],url=c['html_url'],document_id=str(c['id']),reporter_id='github:'+str(c['user']['id']),reporter_login=c['user']['login'],source_sha256=digest(body),source_offset=a,paragraph_length=len(text),paragraph_sha256=digest(text),published_at=c['created_at'],updated_at=c['updated_at'],stratum='explicit-catalog-name',text=text,source_text=body))
   if len(seen)>=4:break
 jsonl(root/'identity-challenge-reserved.jsonl',out);write(root/'identity-challenge-reservation.json',[{k:v for k,v in r.items() if k not in ['text','source_text']} for r in out]);print('Additional identity units reserved without body display:',len(out))
def context_challenge(root):
 if (root/'context-challenge-reserved.jsonl').exists():raise ValueError('preserve context challenge')
 used={r['document_id'] for r in json.loads((root/'reservation.json').read_text())['records']}|{r['document_id'] for r in read(root/'identity-challenge-reserved.jsonl')};out=[]
 pattern=re.compile(r'(?i)\b(?:other games?|another game|my friend|he said|she said|told me|in [A-Z][a-z]+ [A-Z][a-z]+)\b|^>',re.M)
 for app,number in FINAL:
  issue=public(root,f'repos/ValveSoftware/Proton/issues/{number}');comments=public(root,f'repos/ValveSoftware/Proton/issues/{number}/comments?per_page=100');pool=[]
  for c in comments:
   if str(c['id']) in used:continue
   body=c.get('body') or ''
   for a,b in paragraphs(body):
    text=body[a:b]
    if pattern.search(text) and ACTION.search(text):pool.append((digest('context-'+str(c['id'])),c,a,b));break
  pool.sort(key=lambda x:x[0])
  for key,c,a,b in pool[:4]:
   if len(out)>=22:break
   body=c['body'];text=body[a:b];out.append(dict(id='context-'+str(len(out)),app_id=app,source_thread_app_id=app,thread=number,title=issue['title'],url=c['html_url'],document_id=str(c['id']),reporter_id='github:'+str(c['user']['id']),reporter_login=c['user']['login'],source_sha256=digest(body),source_offset=a,paragraph_length=len(text),paragraph_sha256=digest(text),published_at=c['created_at'],updated_at=c['updated_at'],stratum='explicit-context-cue',text=text,source_text=body))
 jsonl(root/'context-challenge-reserved.jsonl',out);write(root/'context-challenge-reservation.json',[{k:v for k,v in r.items() if k not in ['text','source_text']} for r in out]);print('Additional context units reserved without body display:',len(out))
if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('root',type=Path);p.add_argument('--catalog',action='store_true');p.add_argument('--challenge',action='store_true');p.add_argument('--context-challenge',action='store_true');a=p.parse_args();context_challenge(a.root) if a.context_challenge else challenge(a.root) if a.challenge else catalog(a.root) if a.catalog else collect(a.root)
