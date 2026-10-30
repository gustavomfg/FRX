"""Bounded public collection; reserve identities/paragraphs without annotation."""
import argparse
from pathlib import Path
import subprocess,json,re,hashlib
from data import authored_paragraphs,VERBS,write,jsonl,digest

GAMES=[('1627720',8780),('2358720',8004),('1182900',6241),('1693980',6486),('1282100',6949)]

def reserve(root):
 out=root/'public';out.mkdir(exist_ok=True);records=[];bodies=[]
 for app,issue in GAMES:
  meta=out/f'{issue}-issue.json';file=out/f'{issue}-comments.json'
  if not meta.exists():meta.write_bytes(subprocess.check_output(['gh','api',f'repos/ValveSoftware/Proton/issues/{issue}'],timeout=45))
  info=json.loads(meta.read_text());assert app in info['title']
  if not file.exists():file.write_bytes(subprocess.check_output(['gh','api',f'repos/ValveSoftware/Proton/issues/{issue}/comments?per_page=100'],timeout=45))
  options=[]
  for row in json.loads(file.read_text()):
   text=row.get('body','')
   if not 40<len(text)<16000:continue
   paragraphs=authored_paragraphs(text)
   if not paragraphs:continue
   p=max(paragraphs,key=lambda s:(len(VERBS.findall(text[s[0]:s[1]])),s[1]-s[0]))
   options.append(dict(row=row,paragraph=p,eventful=bool(VERBS.search(text[p[0]:p[1]])),key=digest('v4-'+str(row['id']))))
  options.sort(key=lambda x:x['key']);rich=[x for x in options if x['eventful']][:10]
  remaining=[x for x in options if x not in rich][:4]
  for x in rich+remaining:
   row=x['row'];text=row['body'];a,b=x['paragraph'];entry=dict(id='final-'+str(len(records)),app_id=app,url=row['html_url'],thread=issue,title=info['title'],source_sha256=digest(text),source_offset=a,paragraph_sha256=digest(text[a:b]),published_at=row['created_at'],updated_at=row['updated_at'],stratum='eventful' if x in rich else 'random-control')
   records.append(entry);bodies.append(dict(entry,text=text[a:b],source_text=text))
  print(app,'reserved',len(rich)+len(remaining),'identities; no bodies printed')
 write(root/'reservation.json',dict(policy='10 eventful + 4 hash controls per game; one bounded authored paragraph per source; not a representative comment sample',records=records))
 jsonl(root/'final-paragraphs.jsonl',bodies)

if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('root',type=Path);reserve(p.parse_args().root)
