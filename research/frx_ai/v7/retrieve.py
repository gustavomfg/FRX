"""Specialised local evidence search with provenance. No network at query time.

TF-IDF shortlists the public corpus snapshot; the preserved FRX v2 encoder reranks.
A result keeps who said it, when, about which game and with which hardware.
"""
import json
import re
import sys
from pathlib import Path

import numpy as np
import scipy.sparse as sp
from sklearn.feature_extraction.text import TfidfVectorizer

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from demos import MAINTAINER, TITLE, reported  # noqa: E402
from reserve import paragraphs  # noqa: E402

GPU = {"nvidia": r"(?i)\b(?:nvidia|geforce|gtx|rtx)\b", "amd": r"(?i)\b(?:amd|radeon|radv|rx ?\d{3,4})\b", "intel": r"(?i)\b(?:intel arc|intel uhd|anv)\b"}
PROTON_MAJOR = re.compile(r"(?i)\bproton[- ]?(\d+)(?:\.\d+)?\b|\bGE-Proton(\d+)")


def build(root, exclude_apps):
    issues = {}
    for line in (root / "corpus/issues.jsonl").read_text().splitlines():
        i = json.loads(line)
        if not i.get("pull_request"):
            issues[i["number"]] = i
    meta, texts = [], []
    def add(body, number, url, who, when):
        issue = issues[number]
        m = TITLE.search(issue["title"] or "")
        app = m[1] if m else None
        if app in exclude_apps:
            return
        clean = reported(body)
        for a, b in list(paragraphs(clean))[:6]:
            text = clean[a:b]
            if re.search(r"(?i)please add \[?`?PROTON_LOG", text):
                continue
            meta.append(dict(url=url, thread=number, title=issue["title"], app_id=app, association=who, created_at=when, text=text))
            texts.append(text)
    for number, i in issues.items():
        add(i.get("body"), number, i["html_url"], i.get("author_association"), i["created_at"])
    for line in (root / "corpus/comments.jsonl").read_text().splitlines():
        c = json.loads(line)
        number = int(c["issue_url"].rsplit("/", 1)[1])
        if number in issues and c["user"]["type"] != "Bot":
            add(c.get("body"), number, c["html_url"], c["author_association"], c["created_at"])
    v = TfidfVectorizer(ngram_range=(1, 2), sublinear_tf=True, min_df=3, max_features=120000, strip_accents="unicode", lowercase=True, dtype=np.float32)
    x = v.fit_transform(texts)
    d = root / "evidence-index"
    d.mkdir(exist_ok=True)
    sp.save_npz(d / "tfidf.npz", x)
    (d / "meta.jsonl").write_text("".join(json.dumps(m, ensure_ascii=False) + "\n" for m in meta))
    (d / "vocab.json").write_text(json.dumps(dict(vocabulary={k: int(i) for k, i in v.vocabulary_.items()}, idf=[float(x) for x in v.idf_])))
    print(dict(paragraphs=len(meta), vocabulary=len(v.vocabulary_)))


def applicability(item, case):
    """Keep why a source may not apply; relevance never implies it does."""
    context = case.get("context", {})
    notes = []
    relation = "unknown-game"
    if item["app_id"] and context.get("app_id"):
        relation = "same-game" if item["app_id"] == str(context["app_id"]) else "other-game"
    if relation == "unknown-game" and case.get("game") and case["game"].split(":")[0].lower() in (item.get("title") or "").lower():
        # A name in the title is a lead for a person to check, not an identity.
        relation = "name-in-title-unverified"
    text = item["text"]
    vendor = (context.get("gpu_vendor") or "").lower()
    mentioned = [k for k, p in GPU.items() if re.search(p, text)]
    hardware = "not-stated"
    if mentioned:
        hardware = "same-vendor" if vendor in mentioned else "other-vendor" if vendor else "stated-unknown-case"
        if hardware == "other-vendor":
            notes.append(f"relato cita {'/'.join(mentioned)}; o caso usa {vendor}")
    year = int(item["created_at"][:4])
    majors = [int(a or b) for a, b in PROTON_MAJOR.findall(text)]
    current = PROTON_MAJOR.search(context.get("proton") or "")
    version = "not-stated"
    if majors:
        version = "stated"
        if current and max(majors) + 3 <= int(current[1] or current[2]):
            version = "much-older"
            notes.append(f"cita Proton {max(majors)}, bem anterior ao do caso")
    if year <= 2020:
        notes.append(f"publicado em {year}; pode estar desatualizado")
    authority = "maintainer" if item["association"] in MAINTAINER else "community"
    return dict(game=relation, hardware=hardware, version=version, authority=authority, year=year, notes=notes)


class Evidence:
    def __init__(self, root, encoder=None):
        d = Path(root) / "evidence-index"
        self.x = sp.load_npz(d / "tfidf.npz")
        self.meta = [json.loads(line) for line in (d / "meta.jsonl").read_text().splitlines()]
        v = json.loads((d / "vocab.json").read_text())
        self.vec = TfidfVectorizer(ngram_range=(1, 2), sublinear_tf=True, strip_accents="unicode", lowercase=True, dtype=np.float32, vocabulary=v["vocabulary"])
        self.vec.idf_ = np.array(v["idf"], dtype=np.float32)
        self.encoder = encoder

    def search(self, query, case, k=5, shortlist=40):
        q = self.vec.transform([query])
        scores = (self.x @ q.T).toarray().ravel()
        app = str(case.get("context", {}).get("app_id") or "")
        boost = np.array([1.5 if m["app_id"] == app and app else 1.0 for m in self.meta], dtype=np.float32) if app else 1.0
        order = np.argsort(-(scores * boost))[:shortlist]
        order = [i for i in order if scores[i] > 0]
        if not order:
            return []
        if self.encoder is not None:
            qv = self.encoder.encode([query[:1500]])
            dv = self.encoder.encode([self.meta[i]["text"][:1500] for i in order])
            rerank = (qv @ dv.T)[0]
            final = [float(r) * (1.15 if self.meta[i]["app_id"] == app and app else 1.0) for r, i in zip(rerank, order)]
            method = "tfidf+frx-v2-encoder"
        else:
            final, method = [float(scores[i]) for i in order], "tfidf"
        out, seen = [], set()
        for s, i in sorted(zip(final, order), key=lambda x: -x[0]):
            m = self.meta[i]
            # One source per discussion: copies of a thread are not independent evidence.
            key = m["thread"]
            if key in seen:
                continue
            seen.add(key)
            out.append(dict(m, score=s, score_is_probability=False, method=method, applicability=applicability(m, case), content_authenticated=False))
            if len(out) == k:
                break
        return out
