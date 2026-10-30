"""Action/result edge proposals and training-only annotated edge construction.

Negative edges are provisional incomplete-annotation negatives, not causal
truth. They receive lower training weight. No final annotation is read here.
"""
import re
import numpy as np
from scipy.special import softmax

RESULT = re.compile(r"\b(?:work\w*|fix\w*|solv\w*|resolv\w*|crash\w*|freez\w*|froz\w*|worse|better|improv\w*|help\w*|issue|error|problem|flicker\w*|stutter\w*|artifact\w*|launch\w*|no change|no luck)\b",re.I)


def result_proposals(text, action_span, limit=6):
    candidates=[]
    for match in re.finditer(r"[^\n.!?;]+(?:[.!?;]|$)",text):
        chunk=match.group();start=match.start()
        if chunk.lstrip().startswith(">") or len(chunk)>450:continue
        if not RESULT.search(chunk):continue
        candidates.append([start,match.end()])
    # The bounded nearest set is a proposal policy, not a result decision.
    a,b=action_span
    candidates.sort(key=lambda s:(min(abs(s[0]-b),abs(s[1]-a)),s[0]))
    return candidates[:limit]


def overlaps(a,b):
    return bool(a and b and min(a[1],b[1])>max(a[0],b[0]))


def training_pairs(rows):
    output=[]
    for row in rows:
        proposals=result_proposals(row["text"],row["action_span"],4)
        gold=row.get("result_span")
        if gold:proposals=[gold]+[x for x in proposals if x!=gold]
        if not proposals:proposals=[None]
        for span in proposals:
            positive=overlaps(span,gold)
            r=dict(row,candidate_result_span=span)
            if not positive:
                r["outcome"]="unknown"
                if gold:r["edge_weight"]=.35
            output.append(r)
    return output


def inference_pairs(rows):
    output=[];groups=[]
    for row in rows:
        start=len(output)
        spans=result_proposals(row["text"],row["action_span"])
        for span in spans or [None]:output.append(dict(row,candidate_result_span=span))
        groups.append((start,len(output)))
    return output,groups


def reduce_predictions(logits,groups):
    """Select one edge without reading its gold outcome or result span."""
    probs=[softmax(x,axis=1) for x in logits]
    score=np.min(np.stack([p.max(1) for p in probs[:3]],1),1)
    predicted=np.stack([p.argmax(1) for p in probs],1)
    factual=(predicted[:,0]!=0)&(predicted[:,2]==2)
    selected=[]
    for start,end in groups:
        eligible=np.flatnonzero(factual[start:end])+start
        selected.append(int(eligible[np.argmax(score[eligible])]) if len(eligible) else start+int(np.argmax(probs[0][start:end,0])))
    return [x[selected] for x in logits],selected
