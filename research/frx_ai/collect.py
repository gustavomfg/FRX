"""Fetch bounded PUBLIC GitHub issue data via authenticated gh, cache outside Git."""
import argparse, json, pathlib, subprocess, hashlib, time
GAMES = ['Cyberpunk 2077','Elden Ring','Hogwarts Legacy','Baldur','Monster Hunter World','The Witcher 3','Control','Red Dead Redemption 2','Death Stranding','Horizon Zero Dawn','God of War','Resident Evil 4']
def get(path, root):
    file=root/(hashlib.sha256(path.encode()).hexdigest()+'.json')
    if file.exists(): return json.loads(file.read_text())
    raw=subprocess.check_output(['gh','api',path], timeout=45)
    if len(raw)>5_000_000: raise ValueError('response limit')
    data=json.loads(raw);file.write_text(json.dumps(data));time.sleep(.25);return data
if __name__=='__main__':
    from urllib.parse import quote
    p=argparse.ArgumentParser();p.add_argument('--cache',type=pathlib.Path,required=True);args=p.parse_args();args.cache.mkdir(parents=True,exist_ok=True)
    issues=[]
    for game in GAMES:
        result=get('search/issues?q='+quote('repo:ValveSoftware/Proton in:title "'+game+'"')+'&sort=comments&order=desc&per_page=1',args.cache)
        if result.get('items'):
            issue=result['items'][0];comments=get('repos/ValveSoftware/Proton/issues/'+str(issue['number'])+'/comments?per_page=100',args.cache)
            # At most 100 comments per issue; pagination is explicitly limited.
            issues.append({'game_query':game,'issue':issue,'comments':comments})
            print(game,issue['number'],issue['title'],len(comments),flush=True)
    (args.cache/'corpus.json').write_text(json.dumps(issues,ensure_ascii=False))
