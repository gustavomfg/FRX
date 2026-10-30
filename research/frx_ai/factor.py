"""Third iteration: bounded sentence evidence instead of whole-comment vectors."""
import re
from core import clean,ENV
ACTION=re.compile(r'\b(?:installed|installing|replaced|replacing|renamed|copied|copying|updated|upgraded|downgraded|switched|switching|removed|removing|uninstalling|setting|changed|turning|enabled|disabled|trying|tried)\b',re.I)
RESULT=re.compile(r'\b(?:fix(?:ed|es)?|works?|worked|working|help(?:ed)?|crash(?:es|ed|ing)?|freeze|freezing|failed|resolved|solved|caused|worse|better|difference|persists|stable|successfully)\b',re.I)
def focus(row):
    text=clean(row['text']); units=[s.strip(' `\t\r\n-*') for s in re.split(r'(?<=[.!?])\s+|\n+',text) if s.strip()]
    units=[s for s in units if not re.match(r'^(?:OS:|GPU:|CPU:|Kernel|Driver|RAM:|https?:|#|<img|\[.*\]\(https?:)',s,re.I)]
    targets=[i for i,s in enumerate(units) if row['target'] in s] if row['target']!='<procedure>' else [i for i,s in enumerate(units) if ACTION.search(s)]
    candidates=[]
    for i in targets:
        if row['target']!='<procedure>' and '%command%' in units[i]:
            context=units[max(0,i-1):i+2]
        else: context=units[i:i+2]
        candidates.append(' '.join(context))
    if not candidates:return ''
    # Prefer explicit result adjacent to action, not a sentence's positive tone.
    candidates.sort(key=lambda s:(bool(RESULT.search(s)),bool(re.search(r'\b(?:I|my|me|we)\b',s)), -len(s)),reverse=True)
    return candidates[0][:950]
def represented(row):
    t=ENV.sub(lambda m:' TARGET ' if m.group()==row['target'] else ' OTHER ',focus(row))
    return re.sub(r'https?://\S+',' URL ',t).lower()
