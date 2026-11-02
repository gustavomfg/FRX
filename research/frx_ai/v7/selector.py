"""Next-check policies compared on D7. Trained only on maintainer demonstrations.

Rules, retrieval, models and the hybrid share one ranking interface so nothing
is scored differently. The test split is read once, after selection is written.
"""
import argparse
import hashlib
import json
import sys
from collections import Counter
from pathlib import Path

import numpy as np
from sklearn.feature_extraction.text import TfidfVectorizer
from sklearn.linear_model import LogisticRegression
from sklearn.model_selection import GroupKFold
from sklearn.neighbors import NearestNeighbors
from scipy.sparse import csr_matrix, hstack

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from demos import CHECKS  # noqa: E402

FLAGS = ["has_log", "has_info", "has_option", "has_version", "has_prefix", "has_verified", "has_scope", "has_terminal"]
# Evidence already in the thread makes asking for it again less useful.
ALREADY = {"collect-proton-log": "has_log", "collect-system-info": "has_info", "collect-terminal-output": "has_terminal", "verify-game-files": "has_verified"}


def flag_matrix(rows):
    return np.array([[float(r["flags"][k]) for k in FLAGS] for r in rows])


def targets(rows):
    return np.array([[float(c in r["labels"]) for c in CHECKS] for r in rows])


class Majority:
    name = "majority"
    def fit(self, rows):
        count = Counter(x for r in rows for x in r["labels"])
        self.order = [c for c, _ in count.most_common()] + [c for c in CHECKS if c not in count]
    def rank(self, rows):
        return [self.order for _ in rows]


class FixedRules:
    """Ask for what is missing, in the order a triager would; no fitted parameter."""
    name = "fixed-rules"
    def fit(self, rows):
        pass
    def rank(self, rows):
        out = []
        for r in rows:
            f = r["flags"]
            order = []
            if not f["has_log"]:
                order.append("collect-proton-log")
            if not f["has_info"]:
                order.append("collect-system-info")
            if f["has_option"]:
                order.append("review-launch-options")
            if not f["has_version"]:
                order.append("test-other-proton")
            order += ["check-driver-stack", "test-other-proton", "isolate-scope", "reset-prefix", "verify-game-files", "collect-terminal-output", "test-runtime-option", "collect-proton-log", "collect-system-info", "review-launch-options"]
            out.append(list(dict.fromkeys(order)))
        return out


def vectorizer():
    return TfidfVectorizer(ngram_range=(1, 2), sublinear_tf=True, min_df=2, max_features=20000, strip_accents="unicode", lowercase=True)


def mask(scores, rows):
    """Hard signal over learned rank: evidence present lowers its own request."""
    scores = scores.copy()
    for i, r in enumerate(rows):
        for j, c in enumerate(CHECKS):
            if c in ALREADY and r["flags"][ALREADY[c]]:
                scores[i, j] *= 0.25
    return scores


def order(scores):
    return [[CHECKS[j] for j in np.argsort(-row, kind="stable")] for row in scores]


class CaseRetrieval:
    """Rao & Daume's first step: what did experts ask in the most similar reports?"""
    def __init__(self, k=15, hybrid=False):
        self.k, self.hybrid = k, hybrid
        self.name = f"case-retrieval-k{k}" + ("+mask" if hybrid else "")
    def fit(self, rows):
        self.v = vectorizer()
        self.x = self.v.fit_transform(r["query"] for r in rows)
        self.y = targets(rows)
        self.nn = NearestNeighbors(n_neighbors=min(self.k, len(rows)), metric="cosine").fit(self.x)
    def scores(self, rows):
        d, idx = self.nn.kneighbors(self.v.transform(r["query"] for r in rows))
        return np.array([((1 - a)[:, None] * self.y[b]).sum(0) for a, b in zip(d, idx)])
    def rank(self, rows):
        s = self.scores(rows)
        return order(mask(s, rows) if self.hybrid else s)


class Linear:
    """One-vs-rest logistic over text and/or thread-state flags."""
    def __init__(self, text=True, flags=True, c=1.0, hybrid=False):
        self.text, self.flags, self.c, self.hybrid = text, flags, c, hybrid
        self.name = "logistic-" + "+".join(x for x, y in [("tfidf", text), ("state", flags)] if y) + f"-C{c}" + ("+mask" if hybrid else "")
    def features(self, rows, fit=False):
        parts = []
        if self.text:
            if fit:
                self.v = vectorizer()
                parts.append(self.v.fit_transform(r["query"] for r in rows))
            else:
                parts.append(self.v.transform(r["query"] for r in rows))
        if self.flags:
            parts.append(csr_matrix(flag_matrix(rows)))
        return hstack(parts).tocsr()
    def fit(self, rows):
        x, y = self.features(rows, True), targets(rows)
        self.models = []
        for j in range(y.shape[1]):
            if y[:, j].sum() == 0:
                self.models.append(None)
                continue
            self.models.append(LogisticRegression(C=self.c, class_weight="balanced", max_iter=2000, random_state=707).fit(x, y[:, j]))
    def scores(self, rows):
        x = self.features(rows)
        return np.column_stack([m.predict_proba(x)[:, 1] if m else np.zeros(x.shape[0]) for m in self.models])
    def rank(self, rows):
        s = self.scores(rows)
        return order(mask(s, rows) if self.hybrid else s)


class EmbeddingLinear:
    """Same head over the preserved FRX v2 encoder; nothing in the encoder is refit."""
    def __init__(self, encoder, c=1.0, hybrid=False):
        self.encoder, self.c, self.hybrid = encoder, c, hybrid
        self.name = f"logistic-v2-embedding+state-C{c}" + ("+mask" if hybrid else "")
        self.cache = {}
    def embed(self, rows):
        todo = [r for r in rows if r["id"] not in self.cache]
        if todo:
            for i in range(0, len(todo), 32):
                part = todo[i:i + 32]
                for r, v in zip(part, self.encoder.encode([r["query"][-1500:] for r in part])):
                    self.cache[r["id"]] = v
        return np.hstack([np.array([self.cache[r["id"]] for r in rows]), flag_matrix(rows)])
    def fit(self, rows):
        x, y = self.embed(rows), targets(rows)
        self.models = [LogisticRegression(C=self.c, class_weight="balanced", max_iter=2000, random_state=707).fit(x, y[:, j]) if y[:, j].sum() else None for j in range(y.shape[1])]
    def scores(self, rows):
        x = self.embed(rows)
        return np.column_stack([m.predict_proba(x)[:, 1] if m else np.zeros(x.shape[0]) for m in self.models])
    def rank(self, rows):
        s = self.scores(rows)
        return order(mask(s, rows) if self.hybrid else s)


def metrics(rows, ranks):
    n = len(rows)
    top1 = [ranks[i][0] in rows[i]["labels"] for i in range(n)]
    top3 = [bool(set(ranks[i][:3]) & set(rows[i]["labels"])) for i in range(n)]
    mrr = [next((1 / (k + 1) for k, c in enumerate(ranks[i]) if c in rows[i]["labels"]), 0) for i in range(n)]
    per = {}
    for c in CHECKS:
        gold = [i for i in range(n) if c in rows[i]["labels"]]
        pred = [i for i in range(n) if ranks[i][0] == c]
        per[c] = dict(support=len(gold), predicted=len(pred), top1_recall=sum(ranks[i][0] == c for i in gold) / len(gold) if gold else None, top1_precision=sum(c in rows[i]["labels"] for i in pred) / len(pred) if pred else None)
    seen = [v["top1_recall"] for v in per.values() if v["top1_recall"] is not None]
    return dict(n=n, top1=float(np.mean(top1)), top3=float(np.mean(top3)), mrr=float(np.mean(mrr)), macro_top1_recall=float(np.mean(seen)) if seen else None, classes_predicted=len({r[0] for r in ranks}), per_class=per), np.array(top1, float)


def strata(rows, ranks):
    first = [i for i, r in enumerate(rows) if r["first_response"]]
    later = [i for i, r in enumerate(rows) if not r["first_response"]]
    missing = [i for i, r in enumerate(rows) if r["flags"]["has_log"] and r["flags"]["has_info"]]
    return {name: {k: v for k, v in metrics([rows[i] for i in ids], [ranks[i] for i in ids])[0].items() if k != "per_class"} for name, ids in [("first-response", first), ("later", later), ("log-and-info-already-present", missing)] if ids}


def candidates(encoder):
    out = [Majority(), FixedRules(), CaseRetrieval(15), CaseRetrieval(15, hybrid=True), Linear(text=False), Linear(flags=False), Linear(), Linear(hybrid=True), Linear(c=10.0), Linear(c=10.0, hybrid=True)]
    if encoder is not None:
        out += [EmbeddingLinear(encoder), EmbeddingLinear(encoder, hybrid=True)]
    return out


def cross_validate(rows, encoder):
    groups = [r["group"] for r in rows]
    result = {}
    for model in candidates(encoder):
        ranks = [None] * len(rows)
        for a, b in GroupKFold(5).split(rows, groups=groups):
            model.fit([rows[i] for i in a])
            for i, rk in zip(b, model.rank([rows[i] for i in b])):
                ranks[i] = rk
        m, _ = metrics(rows, ranks)
        m["strata"] = strata(rows, ranks)
        result[model.name] = m
        print(f"{model.name:44} top1 {m['top1']:.3f} top3 {m['top3']:.3f} mrr {m['mrr']:.3f} macro {m['macro_top1_recall']:.3f} classes {m['classes_predicted']} | later top1 {m['strata']['later']['top1']:.3f} | log+info present top1 {m['strata'].get('log-and-info-already-present', dict(top1=float('nan')))['top1']:.3f}", flush=True)
    return result


def load_encoder(old):
    if old is None:
        return None
    sys.path.insert(0, str(HERE.parent / "v6"))
    from bridge import retriever
    return retriever(old)


def main():
    p = argparse.ArgumentParser()
    p.add_argument("root", type=Path)
    p.add_argument("--old", type=Path)
    p.add_argument("--final", action="store_true")
    a = p.parse_args()
    rows = [json.loads(x) for x in (a.root / "d7.jsonl").read_text().splitlines()]
    train = [r for r in rows if r["split"] == "train"]
    test = [r for r in rows if r["split"] == "test"]
    encoder = load_encoder(a.old)
    if not a.final:
        cv = cross_validate(train, encoder)
        (a.root / "selector-cv.json").write_text(json.dumps(cv, indent=1))
        return
    dest = a.root / "selector-final.json"
    if dest.exists():
        raise SystemExit("preserve first D7 test read")
    selection = json.loads((a.root / "selector-selection.json").read_text())
    results, tops = {}, {}
    fitted = {}
    for model in candidates(encoder):
        model.fit(train)
        ranks = model.rank(test)
        m, tops[model.name] = metrics(test, ranks)
        m["strata"] = strata(test, ranks)
        results[model.name] = m
        fitted[model.name] = model
        print(f"{model.name:44} top1 {m['top1']:.3f} top3 {m['top3']:.3f} mrr {m['mrr']:.3f} classes {m['classes_predicted']} | later {m['strata']['later']['top1']:.3f}", flush=True)
    groups = sorted({r["group"] for r in test})
    index = {g: [i for i, r in enumerate(test) if r["group"] == g] for g in groups}
    rng = np.random.default_rng(707)
    def interval(a_name, b_name, ids=None):
        deltas = []
        for _ in range(4000):
            sample = rng.choice(groups, len(groups))
            take = np.concatenate([index[g] for g in sample])
            deltas.append(tops[a_name][take].mean() - tops[b_name][take].mean())
        return np.quantile(deltas, [0.025, 0.975]).tolist()
    chosen = selection["selected"]
    comparisons = {b: interval(chosen, b) for b in results if b != chosen}
    out = dict(status="first read of the D7 test split", selection=selection, train=len(train), test=len(test), test_groups=len(groups), results=results, top1_gain_of_selected_by_group=comparisons, sha256=hashlib.sha256((a.root / "d7.jsonl").read_bytes()).hexdigest(), limitation="A maintainer's request shows practice, not proven diagnostic utility; labels come from deterministic functions.")
    with dest.open("x") as f:
        json.dump(out, f, indent=1)
    print(json.dumps(dict(selected=chosen, comparisons=comparisons), indent=1))


if __name__ == "__main__":
    main()
