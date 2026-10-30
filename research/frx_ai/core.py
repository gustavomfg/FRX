"""Data-only action evidence representation. No execution or filesystem mutation."""
import re
ENV=re.compile(r'\b(?:PROTON_[A-Z0-9_]+|VKD3D_[A-Z0-9_]+|DXVK_[A-Z0-9_]+|WINE[A-Z0-9_]*|__GL_[A-Z0-9_]+)\b')
LABELS=['unknown','positive','not-resolved','worsened','mixed']
def clean(text):
    text=re.sub(r'<!--.*?-->','',text,flags=re.S)
    return '\n'.join(l for l in text.splitlines() if not l.lstrip().startswith('>'))
def window(row):
    text=clean(row['text']);target=row['target']
    if target=='<procedure>':
        # Entire bounded authored text: no invented component identity.
        return text[:2600]
    found=list(re.finditer(re.escape(target),text))
    if not found:return ''
    chunks=[]
    for m in found[:4]:
        # Include introduction/consequence around a code block. Keep contrast.
        chunks.append(text[max(0,m.start()-240):min(len(text),m.end()+360)])
    return '\n'.join(dict.fromkeys(chunks))
def representation(row,local=True):
    text=window(row) if local else row['text']
    target=row['target']
    text=ENV.sub(lambda m:' TARGET ' if m.group()==target else ' OTHER ',text)
    text=re.sub(r'https?://\S+',' URL ',text)
    text=re.sub(r'@[\w-]+',' USER ',text)
    return text.lower()
def state_of(row):
    text=window(row).lower();target=row['target']
    if target!='<procedure>':
        # A local action verb must refer to this target, not another variable.
        pos=text.find(target.lower());near=text[max(0,pos-65):pos+len(target)+15]
        if re.search(re.escape(target.lower())+r'\s*=\s*[\'\"]?0\b',near) or re.search(r'\b(disabl\w*|remov\w*|without|unset|turned off)\b',near):return 'disable'
        return 'enable'
    for rx,label in [(r'\b(replac\w*|renam\w*|cop(?:y|ied|ying)|overwrit\w*)\b','replace'),(r'\b(uninstall\w*|remov\w*)\b','remove'),(r'\b(updat\w*|upgrad\w*|downgrad\w*)\b','update'),(r'\bswitch\w*\b','switch'),(r'\binstall\w*\b','install'),(r'\b(setting|set|changed?|changing|turning)\b','configure')]:
        if re.search(rx,text):return label
    return 'unknown'
def guards(row):
    """Conservative invariant gates, independent of outcome vocabulary."""
    text=window(row)
    if not text:return ['no-authored-target']
    if row['target']!='<procedure>' and row['target'] in {'PROTON_LOG','WINEDEBUG','VKD3D_DEBUG','VKD3D_LOG_FILE'}:return ['diagnostic-variable-not-a-fix']
    if state_of(row)=='unknown':return ['action-unidentified']
    return []
def extra_features(row):
    t=representation(row);target=row['target']
    return [float(target=='<procedure>'),float('?' in t),float('>' in row['text']),float('```' in t),float('other' in t),float('target' in t),float(bool(re.search(r'\b(try|could|should|please)\b',t))),float(bool(re.search(r'\b(i|my|me|we)\b',t))),float(bool(re.search(r'\b(not|never|without|no)\b',t))),float(bool(re.search(r'\b(but|however|although|edit)\b',t)))]
