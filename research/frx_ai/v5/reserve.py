"""Reserve new-game source identities without displaying evaluation bodies."""
import argparse,hashlib,json,re,subprocess
from pathlib import Path

GAMES=[('480490',251),('39210',580),('1643320',8260),('2420110',7590),('1817070',6088),('1364780',6817)]
ACTION=re.compile(r'\b(?:enabl\w*|disabl\w*|install\w*|uninstall\w*|replac\w*|remov\w*|updat\w*|upgrad\w*|downgrad\w*|switch\w*|chang\w*|cop(?:y|ied|ying)|set(?:ting)?|tried|try|trying|launch\w*|using|used|ran|restart\w*|reboot\w*|test\w*)\b',re.I)
RESULT=re.compile(r'\b(?:work\w*|fix\w*|resolv\w*|crash\w*|hang\w*|fail\w*|worse|better|stopp\w*|same|help\w*|success\w*|distort\w*|freez\w*)\b',re.I)

def digest(text):return hashlib.sha256(text.encode()).hexdigest()
def blocks(text):
 out=[]
 for m in re.finditer(r'[^\n](?:.*?)(?=\r?\n\s*\r?\n|\Z)',text,re.S):
  a,b=m.span()
  while a<b and text[a].isspace():a+=1
  while b>a and text[b-1].isspace():b-=1
  piece=text[a:b]
  if not 35<=len(piece)<=1100 or piece.startswith(('>','```','<!--','![','<img','# Compatibility','# System')):continue
  out.append((a,b))
 return out

def reserve(root):
 if (root/'reservation.json').exists():raise ValueError('preserve first reservation')
 cache=root/'public';cache.mkdir(exist_ok=True);records=[];units=[]
 for app,issue in GAMES:
  meta=cache/f'{issue}-issue.json';comments=cache/f'{issue}-comments.json'
  for file,url in [(meta,f'repos/ValveSoftware/Proton/issues/{issue}'),(comments,f'repos/ValveSoftware/Proton/issues/{issue}/comments?per_page=100')]:
   if not file.exists():
    raw=subprocess.check_output(['gh','api',url],timeout=45)
    if len(raw)>8000000:raise ValueError('bounded public page')
    file.write_bytes(raw)
  info=json.loads(meta.read_text());assert app in info['title'];candidates=[];seen=set()
  for item in json.loads(comments.read_text()):
   body=item.get('body') or ''
   if not 35<len(body)<200000:continue
   choices=blocks(body)
   if not choices:continue
   a,b=max(choices,key=lambda x:(bool(ACTION.search(body[x[0]:x[1]])) and bool(RESULT.search(body[x[0]:x[1]])),len(ACTION.findall(body[x[0]:x[1]])),x[1]-x[0]));text=body[a:b]
   if digest(text) in seen:continue
   seen.add(digest(text));candidates.append(dict(item=item,a=a,b=b,text=text,action=bool(ACTION.search(text)),result=bool(RESULT.search(text)),key=digest('v5-'+str(item['id']))))
  candidates.sort(key=lambda x:x['key']);rich=[x for x in candidates if x['action'] and x['result']][:8];other=[x for x in candidates if x not in rich and x['action']][:2];control=[x for x in candidates if x not in rich+other][:2]
  for x in rich+other+control:
   item=x['item'];body=item['body'];r=dict(id='final-'+str(len(records)),app_id=app,thread=issue,url=item['html_url'],title=info['title'],source_sha256=digest(body),source_offset=x['a'],paragraph_length=len(x['text']),paragraph_sha256=digest(x['text']),published_at=item['created_at'],updated_at=item['updated_at'],stratum='action-result' if x in rich else 'action' if x in other else 'hash-control')
   records.append(r);units.append(dict(r,text=x['text'],source_text=body))
  print(app,'reserved',len(rich)+len(other)+len(control),'units; no evaluation bodies displayed')
 (root/'reservation.json').write_text(json.dumps(dict(policy='up to 8 action-result, 2 other action and 2 hash controls; one paragraph per comment, first 100 comments; event-enriched, not representative',records=records),indent=2))
 (root/'final-reserved.jsonl').write_text(''.join(json.dumps(r,ensure_ascii=False)+'\n' for r in units))

if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('root',type=Path);reserve(p.parse_args().root)
