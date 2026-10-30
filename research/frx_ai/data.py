"""Bounded candidate preparation. No label is inferred by this module."""
import json, re, hashlib, pathlib
ENV=re.compile(r'\b(?:PROTON_[A-Z0-9_]+|VKD3D_[A-Z0-9_]+|DXVK_[A-Z0-9_]+|WINE[A-Z0-9_]*|__GL_[A-Z0-9_]+)\b')
ACTION=re.compile(r'\b(?:install(?:ed|ing)?|replac(?:e|ed|ing)|updat(?:e|ed|ing)|switch(?:ed|ing)?|downgrad(?:e|ed|ing)|winetricks|protontricks)\b',re.I)
def prepare(root):
    public=json.loads((root/'public/corpus.json').read_text()); previous=json.loads((root/'cached-public.json').read_text()); rows=[]
    groups=[(x['issue'],x['comments']) for x in public]
    for url,issue in previous['issues'].items():
        if '9595' in url or url.endswith('/309'):
            groups.append((issue,[x for u,x in previous['comments'].items() if u.split('#')[0]==url]))
    final={607,4125,8107,6641}; dev={5613,4243,4069}
    for issue,comments in groups:
        title=issue['title']; app=re.search(r'\((\d{3,})\)',title)
        if not app: continue
        split='final' if issue['number'] in final else 'dev' if issue['number'] in dev else 'train'
        for x in comments:
            text=x.get('body') or ''
            if not text or len(text)>2600:continue
            options=sorted(set(ENV.findall(text)))
            # Also retain a single generic procedural action where no variable is named.
            targets=options or (['<procedure>'] if ACTION.search(text) else [])
            for target in targets[:5]:
                h=hashlib.sha256((x['html_url']+'|'+target).encode()).hexdigest()[:12]
                rows.append(dict(id=h,split=split,thread=issue['html_url'],game=title,app_id=app[1],url=x['html_url'],text=text,target=target,published_at=x.get('created_at'),origin='real-public-github'))
    # Stable random order and bounded sample. Annotation sees complete comments.
    rows.sort(key=lambda x:hashlib.sha256(('frx20261029'+x['id']).encode()).hexdigest())
    selected=[]
    for split,n in [('train',100),('dev',50),('final',80)]:
        candidates=[x for x in rows if x['split']==split]
        # At most two targets per comment, to limit dependence.
        counts={}
        for x in candidates:
            if counts.get(x['url'],0)>=2:continue
            selected.append(x);counts[x['url']]=counts.get(x['url'],0)+1
            if sum(r['split']==split for r in selected)>=n:break
    (root/'candidates.json').write_text(json.dumps(selected,ensure_ascii=False,indent=2))
    print({s:sum(x['split']==s for x in selected) for s in ['train','dev','final']})
if __name__=='__main__':
    import sys;prepare(pathlib.Path(sys.argv[1]))
