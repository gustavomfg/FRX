"""Data contracts shared by the offline prototype; never authorizes execution."""
import hashlib
import re
from urllib.parse import urlsplit

MAX_TEXT=16000


def checked_span(text,span):
    if not isinstance(span,list) or len(span)!=2 or any(type(x) is not int for x in span):
        raise ValueError("span must contain two integer code-point offsets")
    a,b=span
    if not 0<=a<b<=len(text):raise ValueError("span outside original source")
    return dict(start=a,end=b,text=text[a:b],offset_encoding="unicode-codepoints",
                source_sha256=hashlib.sha256(text.encode()).hexdigest())


def check_evidence(context,item):
    if not isinstance(context,dict) or not isinstance(item,dict):return "invalid-evidence-object"
    app=context.get("app_id")
    if not isinstance(app,str) or not re.fullmatch(r"[1-9]\d{1,9}",app):return "invalid-request-appid"
    if item.get("app_id")!=app:return "different-or-unattributed-game"
    if item.get("origin","").startswith("synthetic"):return "synthetic-input-not-real-evidence"
    text=item.get("text")
    if not isinstance(text,str) or not text.strip() or len(text)>MAX_TEXT:return "source-text-size-or-type"
    try:
        u=urlsplit(item.get("url",""))
        port=u.port
    except (ValueError,TypeError):return "invalid-source-url"
    if u.scheme!="https" or u.hostname!="github.com" or u.username or u.password or port:
        return "unverified-source-url"
    if not re.fullmatch(r"/[\w.-]+/[\w.-]+/(issues|discussions)/\d+",u.path):return "unverified-source-path"
    # This validates caller attribution, not the live remote issue's identity.
    source_context=item.get("context",{})
    if not isinstance(source_context,dict):return "invalid-source-context"
    for key in ["gpu","driver","proton"]:
        current=context.get(key);source=source_context.get(key)
        if (current is not None and not isinstance(current,str)) or (source is not None and not isinstance(source,str)):
            return "invalid-context-field"
        if current and source and current.casefold()!=source.casefold():return "different-known-"+key
    return None


def graph(row,outcome,direction,modality,joint,confidence,result_span=None):
    action=checked_span(row["text"],row["action_span"])
    quote=checked_span(row["text"],result_span) if result_span else None
    return dict(app_id=row["app_id"],source=row["url"],
        context=row.get("context",{}),action=dict(action,direction_candidate=direction),
        result_quote=quote,outcome_candidate=outcome,modality_candidate=modality,
        association="reported-after-combination" if joint else "candidate-action-result-relation",
        individual_cause="unknown",causal_effect_established=False,
        combination=dict(literal_action_group=action,individual_cause="unknown") if joint else None,
        confidence_score=float(confidence),score_is_safety_probability=False,
        provenance_status="caller-attributed-source; not-network-verified-at-inference",
        accepted_automatically=False,execution_authorized=False,
        limitations=["Experimental learned interpretation requires review.",
                     "A community report is not a local benchmark or causal proof."])
