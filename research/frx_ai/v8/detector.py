"""Isolated outcome detection: explicit rules, no parser, no fitted parameter.

Separates a state ("the game crashes") from an outcome claim: the effect of a change or an
attempt, as defined in research/frx_ai/benchmark/ANNOTATION-GUIDE.md. Nothing here says why.
"""
import re

ORIGIN = "rule"
CLASSES = ("resolved", "improved", "not_resolved", "worsened", "mixed", "no_claim", "unclear")
BODY = r"(?:issue\w*|problem\w*|bug\w*|crash\w*|freez\w*|stutter\w*|error\w*|hang\w*|glitch\w*|flicker\w*|lag|it|this|that|everything|all)"
SYMPTOM = r"(?:crash\w*|freez\w*|froze\w*|hang\w*|hung|stutter\w*|flicker\w*|black\s*screen\w*|fail\w*|error\w*|broken|stuck)"
RUN = r"(?:work|works|worked|working|run|runs|ran|running|launch|launches|launched|launching|start|starts|started|load|loads|loaded|boot|boots|booted|play|plays|played)"
GOOD = r"(?:fine|well|great|perfectly|flawlessly|correctly|properly|normally|smooth\w*|okay|ok|good|for\s+me|as\s+expected)"

# (class, pattern, needs a change or attempt nearby). Priority follows this order on overlaps.
CUES = [
    ("not_resolved", r"\b(?:didn['’]?t|doesn['’]?t|did\s+not|does\s+not|won['’]?t|don['’]?t|hasn['’]?t|has\s+not|haven['’]?t|not|never)\s+(?:\w+\s+){0,2}?(?:help\w*|fix\w*|solv\w*|resolv\w*|chang\w*|rectif\w*|make\s+(?:any|a)\s+difference)\b", False),
    ("not_resolved", r"\b(?:didn['’]?t|doesn['’]?t|did\s+not|does\s+not|won['’]?t)\s+(?:\w+\s+){0,2}?work\b", True),
    ("not_resolved", r"\bno\s+(?:luck|effect|change|difference|improvement|dice|success)\b", False),
    ("not_resolved", r"\b(?:made|makes|with)\s+(?:no|zero)\s+difference\b", False),
    ("not_resolved", r"\b(?:none|non|neither|nothing)(?:\s+of\s+(?:them|these|those|this|the\s+\w+))?\s+(?:\w+\s+){0,2}?(?:work\w*|help\w*|fix\w*|solv\w*|resolv\w*|chang\w*)\b", False),
    ("not_resolved", r"\b(?:all|both)\s+(?:have|has|show|gave|give)\s+(?:this|the\s+same|that)\b", False),
    ("not_resolved", r"\bstill\s+(?:the\s+)?same\b", False),
    ("not_resolved", r"\b(?:isn['’]?t|is\s+not|wasn['’]?t|not)\s+(?:\w+\s+)?(?:fixed|solved|resolved)\b", False),
    ("not_resolved", r"\bstill\s+(?:\w+\s+){0,2}?(?:" + SYMPTOM + r"|there|happen\w*|persist\w*|occur\w*|show\w*|not|doesn['’]?t|won['’]?t|can['’]?t)\b", True),
    ("not_resolved", r"\b(?:\w+\s+)?same\s+(?:\w+\s+)?(?:issue|problem|error|crash|freeze|bug|result|behaviou?r|symptom|black\s*screen)\b", True),
    ("not_resolved", r"\b(?:doesn['’]?t|does\s+not|won['’]?t|can['’]?t|cannot|fails?\s+to|not)\s+(?:\w+\s+)?(?:start|launch|run|boot|load|working)\b", True),
    ("worsened", r"\b(?:made|makes|got|gets|getting|became|even)\s+(?:\w+\s+){0,2}?worse\b", False),
    ("worsened", r"\b(?:broke|breaks|broken\s+by|regress(?:ed|es|ion)|side[- ]effect|introduc(?:es|ed))\b", False),
    ("worsened", r"\b(?:no\s+longer\s+(?:work|start|launch|run|load|boot)\w*|stopped\s+(?:working|launching|running|starting)|started\s+(?:crash|freez|hang)\w*)\b", False),
    ("worsened", r"\bcaus(?:es|ed|ing)\s+(?:\w+\s+){0,4}?" + SYMPTOM + r"\b", False),
    ("resolved", r"\b(?:fix(?:es|ed)|solv(?:es|ed)|resolv(?:es|ed)|cur(?:es|ed)|did\s+the\s+trick|got\s+rid\s+of)\b", False),
    ("resolved", r"\b" + BODY + r"\s+(?:\w+\s+){0,2}?(?:is|are|was|were|seems?|went|goes|has\s+been|have\s+been)\s+(?:\w+\s+){0,2}?(?:gone|away|fixed|solved|resolved)\b", False),
    ("resolved", r"\bno\s+longer\s+(?:crash|freez|hang|stutter|flicker|happen|occur)\w*\b", False),
    ("resolved", r"\b(?:doesn['’]?t|does\s+not|didn['’]?t|did\s+not)\s+(?:crash|freeze|hang|stutter)\s+any\s*more\b", False),
    ("resolved", r"\b" + RUN + r"\s+(?:\w+\s+){0,2}?(?:now|again)\b", False),
    ("resolved", r"\bnow\s+" + RUN + r"\b", False),
    ("improved", r"\b(?:helps?|helped|helping|improv(?:es|ed)|reduc(?:es|ed)|workaround\s+works)\b", False),
    ("improved", r"\b(?:much|way|a\s+lot|a\s+bit|slightly|somewhat)\s+(?:better|less\s+\w+|smoother)\b", False),
    ("improved", r"\bless\s+(?:often|frequent\w*|stutter\w*|crash\w*)\b", False),
    ("improved", r"\b(?:did\s+not|didn['’]?t)\s+(?:crash|freeze|hang)\b", True),
    ("resolved", r"\b" + RUN + r"\s+(?:\w+\s+){0,2}?" + GOOD + r"\b", True),
    ("resolved", r"\b(?:seems?\s+to\s+work|is\s+flawless|flawless(?:ly)?|it\s+works|managed\s+to\s+\w+)\b", True),
    ("resolved", r"\b(?:did|does)\s+work\b|\b(?:can|could)\s+(?:now\s+|finally\s+)?(?:get\s+(?:in|into)|play|launch|start|finish)\b|\bable\s+to\s+(?:now\s+)?(?:play|launch|start|finish|run)\b|\b(?:the\s+)?game\s+(?:now\s+)?(?:starts|launches|boots|loads)\b", True),
    ("resolved", r"\b(?:disappear\w*|went\s+away|back\s+to\s+normal|no\s+longer\s+blocks?)\b", False),
    ("not_resolved", r"\b(?:keep|keeps|kept)\s+(?:getting|crashing|freezing|failing)\b|\b(?:gives?|gave|throws?|threw)\s+(?:\w+\s+){0,2}?(?:error|exception|crash)\b|\bnothing\s+(?:\w+\s+)?(?:happen\w*|work\w*)\b|\bcrash\w*\s+(?:again|continu\w*)\b", True),
    ("not_resolved", r"\b(?:crash(?:es|ed)|freez(?:es)|froze|black\s*screen|hangs)\b", True),
]
COMPILED = [(k, re.compile("(?i)" + p), ctx) for k, p, ctx in CUES]

ATTEMPT = re.compile(
    r"(?i)\b(?:tried|try(?:ing)?|tested|testing|using|used|with\s+(?:proton|ge|the|older|newer|latest|experimental|hotfix)|after|since|switch\w*|setting|enabl\w*|disabl\w*|install\w*|remov\w*|delet\w*|"
    r"clear\w*|revert\w*|downgrad\w*|upgrad\w*|updat\w*|chang\w*|add(?:ed|ing)|renam\w*|replac\w*|via|older|newer|fresh\s+prefix|clean\s+prefix|GE-Proton\S*|proton[- ]?\d\S*|experimental|hotfix)\b"
    r"|\b[A-Z][A-Z0-9_]{3,}=\S*|%command%"
)
HEDGE = re.compile(
    r"(?i)\b(?:if|whether|unless|should|would|might|may|maybe|perhaps|hope\w*|please|pls|could\s+you|can\s+you|try\s+to|see\s+if|to\s+see|"
    r"will|won['’]t\s+be|going\s+to|need(?:s|ed)?\s+to|want(?:s|ed)?\s+to|wish|supposed\s+to|expect\w*|not\s+sure|until|once)\b"
)
ADVICE = re.compile(r"(?i)^\W*(?:(?:@\S+\s+)?(?:just\s+)?try\b|you\s+(?:can|could|should|might|may|need)\b|to\s+anyone\b|consider\b|recommend\b|all\s+i\s+can\s+recommend\b|make\s+sure\b|check\b)")
THIRD = re.compile(r"(?i)\b(?:people|others?|users?|someone|somebody|he|she|friend|brother|they)\s+(?:\w+\s+){0,2}?(?:report\w*|say\w*|said|claim\w*|confirm\w*|told)\b|\baccording\s+to\b|\bapparently\b")
FIRST = re.compile(r"(?i)\b(?:i|i['’]\w+|my|me|we|our|for\s+me)\b")
LATER = re.compile(r"(?i)\b(?:edit|update|never\s*mind|nvm|finally|eventually|in\s+the\s+end|turns?\s+out|solution)\b")
TEMPLATE = re.compile(r"(?m)^\s*(?:#{1,6}\s.*|[-*]\s*\[[ xX]\].*|[-*]\s*(?:GPU|Video driver version|Kernel version|Proton version|Link to full system information report|Name of the game|Steam AppID)[^\n]*)$")
SPLIT = re.compile(r"(?<=[.!?])\s+|\n+|(?<=:)\s+(?=[A-Z`])")


def _blank(text, pattern, flags=0):
    return re.sub(pattern, lambda m: re.sub(r"[^\n]", " ", m[0]), text, flags=flags)


def clean(text):
    """Blank what the author did not assert, keeping every offset literal."""
    out = _blank(text, r"<!--.*?-->", re.S)
    out = _blank(out, r"```.*?(?:```|\Z)", re.S)
    out = _blank(out, r"~~.*?~~", re.S)
    out = _blank(out, r"(?m)^\s*>.*$")
    out = _blank(out, r"<img[^>]*>|!\[[^\]]*\]\([^)]*\)|\]\([^)]*\)|https?://\S+")
    return _blank(out, TEMPLATE.pattern, re.M)


def sentences(cleaned):
    start = 0
    out = []
    for m in SPLIT.finditer(cleaned):
        if cleaned[start:m.start()].strip():
            out.append((start, m.start()))
        start = m.end()
    if cleaned[start:].strip():
        out.append((start, len(cleaned)))
    return out


def _hedged(sentence, offset):
    before = sentence[:offset]
    return "?" in sentence or bool(HEDGE.search(before[-70:]))


def detect(text):
    cleaned = clean(text or "")
    parts = sentences(cleaned)
    claims, states = [], []
    for n, (a, b) in enumerate(parts):
        sentence = cleaned[a:b]
        previous = cleaned[parts[n - 1][0]:parts[n - 1][1]] if n else ""
        if ADVICE.search(sentence) and not FIRST.search(sentence):
            continue
        found = []
        for kind, pattern, needs in COMPILED:
            for m in pattern.finditer(sentence):
                if any(m.start() < y and x < m.end() for x, y, _, _ in found):
                    continue
                if _hedged(sentence, m.start()):
                    continue
                if needs:
                    bare = kind == "not_resolved" and re.match(r"(?i)crash|freez|froze|black|hang", m[0])
                    near = ATTEMPT.search(sentence) or (kind == "not_resolved" and not bare and ATTEMPT.search(previous))
                    if not near:
                        states.append(dict(span=[a + m.start(), a + m.end()], quote=m[0], polarity="negative" if kind == "not_resolved" else "positive", origin=ORIGIN))
                        continue
                found.append((m.start(), m.end(), kind, needs))
        for x, y, kind, needs in sorted(found):
            claims.append(dict(cls=kind, span=[a + x, a + y], quote=text[a + x:a + y], sentence=n, sentence_span=[a, b],
                               first_hand=not THIRD.search(sentence), needs_attempt=needs, origin=ORIGIN))
    return dict(overall=overall(claims, cleaned), role=role(cleaned, claims), claims=claims, states=states, causality_confirmed=False, origin=ORIGIN)


def overall(claims, cleaned):
    kinds = [c["cls"] for c in claims]
    if not kinds:
        return "no_claim"
    positive = {"resolved", "improved"} & set(kinds)
    negative = {"not_resolved", "worsened"} & set(kinds)
    if positive and negative:
        # "Still crashes, but much less often" reports one partial effect.
        same = {c["sentence"] for c in claims if c["cls"] == "improved"} & {c["sentence"] for c in claims if c["cls"] == "not_resolved"}
        if same and "worsened" not in kinds and "resolved" not in kinds:
            return "improved"
        # A later correction by the author decides the report.
        last = claims[-1]
        if last["cls"] in positive and LATER.search(cleaned[:last["span"][0]]) and claims[0]["cls"] in negative:
            return last["cls"]
        return "mixed"
    if positive:
        return "resolved" if "resolved" in positive else "improved"
    return "worsened" if "worsened" in negative else "not_resolved"


def role(cleaned, claims):
    text = cleaned.strip()
    if claims:
        return "first_hand" if any(c["first_hand"] for c in claims) else "second_hand"
    if not text:
        return "unclear"
    if text.endswith("?") or re.search(r"(?i)\b(?:can|could)\s+you\b|\bplease\b", text):
        return "request_or_question"
    if ADVICE.search(text):
        return "advice"
    return "first_hand" if FIRST.search(text) else "information"
