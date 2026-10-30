"""Deterministic high-recall action proposals, NOT a learned entity extractor.

Offsets always point to the original input. Proposals carry no outcome label.
An action word in a log or quotation is not proof of a tested action.
"""
import re

ENV = re.compile(r"\b(?:PROTON_[A-Z0-9_]+|VKD3D_[A-Z0-9_]+|DXVK_[A-Z0-9_]+|WINE[A-Z0-9_]*|RADV_[A-Z0-9_]+)\b(?:\s*=\s*[^\s`]+)?")
VERB = re.compile(r"\b(?:enabl\w*|disabl\w*|install\w*|uninstall\w*|replac\w*|remov\w*|updat\w*|upgrad\w*|downgrad\w*|switch\w*|chang\w*|cop(?:y|ied|ying)|renam\w*|turn(?:ed|ing)?\s+(?:on|off)|set(?:ting)?)\b",re.I)


def propose(text, limit=12):
    if len(text)>16000:
        raise ValueError("comment exceeds research input bound")
    found=[]
    for match in re.finditer(r"[^\n]+",text):
        line=match.group()
        if line.lstrip().startswith(">") or line.lstrip().startswith("<!--"):
            continue
        if re.match(r"\s*\d+\.\d+:|\s*(?:0x|[0-9a-f]{4}:)",line):
            continue
        envs=list(ENV.finditer(line));verbs=list(VERB.finditer(line))
        if not envs and not verbs:
            continue
        # A code line is kept as a group; prose action spans stop before a
        # likely consequence, while context still contains the consequence.
        start=envs[0].start() if envs else verbs[0].start()
        if envs and verbs and verbs[0].start()<start:
            start=verbs[0].start()
        end=min(len(line),start+200)
        consequence=re.search(r"[.!?;]|\b(?:but|which|that fixed|and now|and it|caused|resolved)\b",line[start:end],re.I)
        if consequence and consequence.start()>8:
            end=start+consequence.start()
        while end>start and line[end-1].isspace():end-=1
        if end-start<4:
            continue
        span=[match.start()+start,match.start()+end]
        if span not in found:found.append(span)
        if len(found)>=limit:break
    return found
