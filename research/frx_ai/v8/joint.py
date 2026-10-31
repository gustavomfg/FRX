"""Procedure representation: a set of interventions, their relation and one reported result.

Explicit rules. A group never credits one member unless the author does. "Joint" states that
actions were reported together; it does not say that all were needed, or that any was.
"""
import re

from detector import clean, detect

ORIGIN = "rule"
RELATIONS = ("single", "joint", "alternatives", "sequence", "unclear", "not_applicable")

ASSIGNMENT = re.compile(r"\b[A-Z][A-Z0-9_]{2,}=[^\s`]*")
LAUNCH_LINE = re.compile(r"(?m)^.*%command%.*$|`[^`\n]*%command%[^`\n]*`")
VERSION = re.compile(r"(?i)\b(?:GE-Proton[\w.-]*|proton[- ]?(?:experimental|hotfix|\d[\w.-]*)|\d+\.\d+(?:-\w+)?(?=\s*(?:,|and|or|\.|$))|experimental|hotfix)\b")
VERB = (
    r"tried|try|trying|tested|testing|installed|installing|install|reinstalled|reinstalling|uninstalled|uninstalling|removed|removing|remove|deleted|deleting|delete|cleared|clearing|cleaned|"
    r"verified|verifying|verify|set|setting|added|adding|switched|switching|updated|updating|upgraded|upgrading|downgraded|downgrading|disabled|disabling|enabled|enabling|changed|changing|"
    r"replaced|replacing|renamed|renaming|reverted|reverting|rebuilt|recompiled|recompiling|copied|copying|used|using|ran(?!\s+into)|running|launched|restarted|rebooted|selected|lowered"
)
ACTION = re.compile(r"(?i)\b(?:" + VERB + r")\b[^.,;:\n!?]{0,70}")
SPLITTER = re.compile(r"(?i)\s+(?:then|and\s+then|but|while|because|so|when)\s+")
SEQUENCE = re.compile(r"(?i)\b(?:then|after\s+that|afterwards?|next|followed\s+by|first|finally|once\s+done)\b")
ALTERNATIVE = re.compile(r"(?i)\b(?:none|non|neither|nor|any\s+of|all\s+of|each\s+of|every|they\s+all|both|individually|separately|one\s+by\s+one|many|various|different)\b")
ISOLATED = re.compile(r"(?i)\b(?:only|alone|solely|the\s+key|the\s+fix\s+was|what\s+(?:fixed|did\s+it)|that\s+did\s+it|by\s+itself|on\s+its\s+own|was\s+enough)\b")
VERB_START = re.compile(r"(?i)\b(?:" + VERB + r")\b")
LIST_ITEM = re.compile(r"(?i)\s*(?:,\s*(?:and\s+|or\s+)?|\s+and\s+|\s+or\s+)((?:(?!then\b|but\b|it\b|they\b|i\b|the\s+game\b|none\b|nothing\b)[\w.`=-]+\s*){1,3}?)(?=,|\s+and\s|\s+or\s|[.;:!?\n]|$)")


def _clip(text, a, b):
    m = SPLITTER.search(text, a, b)
    return a, m.start() if m else b


def _list_items(cleaned, end, b):
    """Short items that continue an action's object: "tried A, B and C"."""
    items, pos = [], end
    while True:
        m = LIST_ITEM.match(cleaned, pos, b)
        if not m:
            return items
        items.append(m.span(1))
        pos = m.end()


def interventions(text, cleaned, a, b):
    """Candidate actions inside one literal window."""
    found = []
    for m in LAUNCH_LINE.finditer(text, a, b):
        found.append(dict(span=list(m.span()), text=m[0], kind="launch-line", assignments=ASSIGNMENT.findall(m[0])))
    versions = [m for m in VERSION.finditer(cleaned, a, b)]
    for m in ACTION.finditer(cleaned, a, b):
        x, y = _clip(cleaned, m.start(), m.end())
        if any(x < s[1] and s[0] < y for s in (f["span"] for f in found)):
            continue
        for start, end in _coordinated(cleaned, x, y):
            found.append(dict(span=[start, end], text=text[start:end].strip(), kind="action"))
        for start, end in _list_items(cleaned, y, b):
            found.append(dict(span=[start, end], text=text[start:end].strip(), kind="list-item"))
    for m in versions:
        if not any(m.start() < f["span"][1] and f["span"][0] < m.end() for f in found):
            found.append(dict(span=list(m.span()), text=m[0], kind="version"))
    found.sort(key=lambda f: f["span"][0])
    return found


def _coordinated(cleaned, a, b):
    """"Installed X and removed Y" holds two actions; "installed X and Y" holds one."""
    starts = [m.start() for m in VERB_START.finditer(cleaned, a, b)]
    if len(starts) < 2:
        return [(a, b)]
    return [(s, (starts[i + 1] if i + 1 < len(starts) else b)) for i, s in enumerate(starts)]


def relation(items, window, claim):
    if not items:
        return "not_applicable"
    lines = [i for i in items if i["kind"] == "launch-line"]
    if len(items) == 1:
        one = items[0]
        return "joint" if len(one.get("assignments", [])) >= 2 else "single"
    versions = [i for i in items if i["kind"] == "version"]
    listed = [i for i in items if i["kind"] == "list-item"]
    negative = claim in ("not_resolved",)
    if SEQUENCE.search(window):
        return "sequence"
    if len(versions) + len(listed) >= 2 and len(versions) + len(listed) >= len(items) - 1:
        return "alternatives"
    if (listed or versions) and negative:
        return "alternatives"
    if ALTERNATIVE.search(window) and negative:
        return "alternatives"
    if lines and any(len(i.get("assignments", [])) >= 2 for i in lines):
        return "joint"
    # Two actions and one result without a connective do not show that they went together.
    return "joint" if re.search(r"(?i)\b(?:and|plus|together|along\s+with|with|as\s+well)\b|,", window) else "unclear"


def _previous_sentence(cleaned, a):
    head = cleaned[:a].rstrip()
    if not head or head.endswith("\n\n") or cleaned[len(head):a].count("\n") > 1:
        return None
    cut = max(head.rfind("\n"), head.rfind(". "), head.rfind("! "), head.rfind("? "))
    return cut + 1 if cut >= 0 else 0


def _window(text, cleaned, a, b):
    """Own sentence; earlier sentences of the paragraph only when it has no action, or narrates steps."""
    start = a
    for _ in range(3):
        own = cleaned[start:b]
        has_action = bool(interventions(text, cleaned, start, b))
        if has_action and not re.search(r"(?i)^\W*(?:then|and\s+then|after\s+that|afterwards?|next|finally)\b", own):
            break
        before = _previous_sentence(cleaned, start)
        if before is None or before == start or not VERB_START.search(cleaned, before, start):
            break
        start = before
    return start


def procedures(text, detected=None):
    detected = detected or detect(text)
    cleaned = clean(text or "")
    out = []
    seen = set()
    for claim in detected["claims"]:
        a, b = claim["sentence_span"]
        # The attempt often sits one sentence earlier: "Tried A and B. Same crash."
        window_a = _window(text, cleaned, a, b)
        items = [i for i in interventions(text, cleaned, window_a, b) if not (i["span"][0] < claim["span"][1] and claim["span"][0] < i["span"][1] and i["kind"] == "action" and i["span"][1] - i["span"][0] < 12)]
        key = (tuple(tuple(i["span"]) for i in items), claim["cls"])
        if key in seen:
            continue
        seen.add(key)
        kind = relation(items, cleaned[window_a:b], claim["cls"])
        credited = [i["span"] for i in items if ISOLATED.search(cleaned[max(window_a, i["span"][0] - 25):min(b, i["span"][1] + 25)])] if kind == "joint" else []
        out.append(dict(interventions=items, relation=kind, result=dict(cls=claim["cls"], span=claim["span"], quote=claim["quote"]),
                        attribution="single-reported" if kind == "single" else "author-isolated" if credited else "group-only",
                        individual_cause="unknown", origin=ORIGIN))
    return out


def unit_relation(text, detected=None):
    """Unit reading: the relation of the procedure that carries the unit's decisive result."""
    detected = detected or detect(text)
    procs = procedures(text, detected)
    if not procs:
        return "not_applicable", []
    wanted = detected["overall"]
    chosen = [p for p in procs if p["result"]["cls"] == wanted and p["interventions"]] or [p for p in procs if p["interventions"]] or procs
    # With a correction later in the text, the last matching report is the decisive one.
    p = chosen[-1]
    return p["relation"], p["interventions"]
