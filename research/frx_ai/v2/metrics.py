"""Task-specific metrics; no metric can authorize a fix."""
import math
import numpy as np
from scipy.special import softmax
from sklearn.metrics import classification_report, confusion_matrix, f1_score
from model import OUTCOMES, labels


def wilson(correct, total, z=1.96):
    if not total:
        return [0., 1.]
    p = correct / total; den = 1 + z*z/total
    center = (p + z*z/(2*total)) / den
    radius = z * math.sqrt(p*(1-p)/total + z*z/(4*total*total)) / den
    return [center-radius, center+radius]


def evaluate(rows, logits, threshold=.6, temperature=1.):
    gold = labels(rows).numpy()
    probs = [softmax(x / temperature, axis=1) for x in logits]
    pred = np.stack([x.argmax(1) for x in probs], 1)
    confidence = np.min(np.stack([x.max(1) for x in probs[:3]], 1), 1)
    # A claim is a non-unknown result attached to a tested, identified action.
    claims = (pred[:, 0] != 0) & (pred[:, 1] != 0) & (pred[:, 2] == 2)
    selected = claims & (confidence >= threshold)
    correct = (pred == gold).all(1)  # includes grouping; stricter than polarity alone
    count = int(selected.sum()); hits = int((selected & correct).sum())
    severe = selected & (((pred[:, 0] == 1) & (gold[:, 0] == 3)) |
                         ((pred[:, 0] == 3) & (gold[:, 0] == 1)) |
                         ((pred[:, 1] == 1) & (gold[:, 1] == 2)) |
                         ((pred[:, 1] == 2) & (gold[:, 1] == 1)))
    by_game = {}
    for game in sorted({r.get("app_id") or "unknown" for r in rows}):
        mask = np.array([(r.get("app_id") or "unknown") == game for r in rows])
        n = int((selected & mask).sum())
        by_game[game] = dict(n=int(mask.sum()), accepted=n,
                             correct=int((selected & mask & correct).sum()))
    ece = 0.
    relation_conf = probs[0].max(1); relation_ok = pred[:, 0] == gold[:, 0]
    for lo in np.linspace(0., .9, 10):
        m = (relation_conf > lo) & (relation_conf <= lo + .100001)
        if m.any():
            ece += m.mean() * abs(relation_conf[m].mean()-relation_ok[m].mean())
    return dict(n=len(rows), accepted=count, correct=hits,
                precision=hits/count if count else None, coverage=count/len(rows),
                wilson=wilson(hits,count), severe_inversions=int(severe.sum()),
                false_success=int((selected & (pred[:,0] == 1) & (gold[:,0] != 1)).sum()),
                relation_macro_f1=float(f1_score(gold[:,0],pred[:,0],labels=list(range(5)),average="macro",zero_division=0)),
                relation_accuracy=float(relation_ok.mean()),
                exact_all_accuracy=float(correct.mean()),
                direction_accuracy=float((pred[:,1]==gold[:,1]).mean()),
                modality_accuracy=float((pred[:,2]==gold[:,2]).mean()),
                grouping_accuracy=float((pred[:,3]==gold[:,3]).mean()),
                ece=float(ece), nll=float(-np.log(probs[0][np.arange(len(rows)),gold[:,0]].clip(1e-9)).mean()),
                confusion=confusion_matrix(gold[:,0],pred[:,0],labels=list(range(5))).tolist(),
                classes=classification_report(gold[:,0],pred[:,0],labels=list(range(5)),target_names=OUTCOMES,output_dict=True,zero_division=0),
                by_game=by_game)


def risk_curve(rows, logits, temperature=1.):
    return {str(t): evaluate(rows,logits,t,temperature) for t in [.0,.4,.5,.6,.7,.8,.9]}
