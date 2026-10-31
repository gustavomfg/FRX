"""Candidate discovery and procedure structure. Explicit rules over a pretrained
parser; no parameter here was fitted, and nothing is a causal claim.

v6 removed what it could not assert before proposing. v7 proposes first and keeps
modality as an attribute, so each later decision is measurable.
"""
import re
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
V6 = HERE.parent / "v6"
for path in [str(V6)]:
    if path not in sys.path:
        sys.path.insert(0, path)

from bridge import children, evidence, digest  # noqa: E402
from grounding import ancestors, deconstruct, identity  # noqa: E402
from events import state as prior_state  # noqa: E402

# Verbs for things people do to a game or system, beyond the v5/v6 operations.
EXTRA_ACTIONS = {
    "patch", "sync", "hide", "bypass", "initialize", "initialise", "finish", "connect", "verify",
    "edit", "pass", "reproduce", "login", "log", "fiddle", "tinker", "apply", "mod", "rollback",
    "revert", "reinstall", "play", "load", "boot", "launch", "start", "open", "click", "go",
    "adjust", "tweak", "toggle", "select", "swap", "wipe", "recompile", "compile", "rebuild",
    "override", "mount", "export", "import", "unplug", "plug", "generate", "bisect",
}
GENERIC_SUBJECT = {"i", "we", "you", "he", "she", "they", "someone", "somebody", "people", "friend", "user", "users"}
HYPOTHETICAL = re.compile(
    r"(?i)\b(?:if|whether|unless|would|should|might|may|hope|hopefully|wish|see if|to see|wonder\w*|please|could you|can you|"
    r"try to|want(?:ed)? to|need(?:s|ed)? to|supposed to|expect\w*|until)\b"
)
NEGATOR = re.compile(r"(?i)(?:\b(?:not|never|no|none|nothing|neither|nor|without|cannot|unable|fail(?:s|ed)?\s+to)\b|n['’]t\b)")

GOOD_STATE = r"(?:fine|well|perfectly|great|good|ok|okay|normally|flawlessly|correctly|properly|smooth\w*|again|now)"
RUN_VERB = r"(?:work|works|worked|working|run|runs|ran|running|launch|launches|launched|launching|load|loads|loaded|loading|boot|boots|booted|start|starts|started|play|plays|played|playable)"
SYMPTOM = (
    r"(?:crash\w*|freez\w*|froze\w*|hang\w*|hung|stutter\w*|flicker\w*|lock(?:s|ed)?\s+up|black\s*screen\w*|error\w*|"
    r"artifact\w*|glitch\w*|segfault\w*|fault\w*|issue\w*|problem\w*|bug\w*|trouble\w*|desync\w*)"
)

# (class, pattern). Order matters only for ties; negation is checked afterwards.
PATTERNS = [
    # Persistent results come first because they carry their own negation.
    ("persistent", r"\bstill\b[^.!?;\n]{0,40}?\b(?:" + SYMPTOM + r"|same|not|n['’]t|gives?|has|have|happen\w*|persist\w*|occur\w*)\b[^.!?;\n]{0,40}"),
    ("persistent", r"\b(?:the\s+|same\s+|exact\s+)*same\s+(?:\w+\s+){0,2}?(?:issue|error|problem|crash|thing|result|behaviou?r|performance|message|freeze)\b"),
    ("persistent", r"\bno\s+(?:luck|difference|change|effect|improvement|success|dice|joy|fix|help)\b[^.!?;\n]{0,20}"),
    ("persistent", r"\b(?:nothing|none)\b[^.!?;\n]{0,30}?\b(?:work\w*|chang\w*|help\w*|happen\w*|fix\w*|to show|show\w*)\b[^.!?;\n]{0,20}"),
    ("persistent", r"\bno\s+\w+(?:\s+of\s+\w+)?[^.!?;\n]{0,40}?\b(?:get|gets|got|make|makes|made|seem|seems)\b[^.!?;\n]{0,10}?\b(?:work\w*|run\w*|start\w*|launch\w*)\b[^.!?;\n]{0,30}"),
    ("persistent", r"(?:\b(?:didn['’]?t|doesn['’]?t|don['’]?t|did\s+not|does\s+not|do\s+not|won['’]?t|wont|can['’]?t|cannot|couldn['’]?t|could\s+not|not\s+able\s+to|unable\s+to|fail(?:s|ed|ing)?\s+to)\s+(?:even\s+|seem\s+to\s+|really\s+)?(?:work|help|fix|change|start|launch|run|load|boot|hear|get|detect|solve|resolve|make|open|write|save|connect|meet)\b)[^.!?;\n]{0,40}"),
    ("persistent", r"\bnot\s+(?:work\w*|running|launching|starting|loading|detect\w*|fix\w*|help\w*)\b[^.!?;\n]{0,30}"),
    ("persistent", r"\b(?:results?\s+in|ends?\s+up|gives?|gave|giving|throws?|threw|shows?|showed|says?|said|got|get|gets|getting|hit|hits|kept|keeps?|experienc\w+|encounter\w*|seeing)\b[^.!?;\n]{0,50}?\b(?:" + SYMPTOM + r"|fail\w*)\b[^.!?;\n]{0,20}"),
    ("persistent", r"\b(?:stuck|unresponsive|hangs?|black\s*screen|fail(?:s|ed|ing|ure)?\s+to\s+\w+|reverts?\s+to|no\s+effect|nothing\s+to\s+show)\b[^.!?;\n]{0,40}"),
    ("persistent", r"\b(?:it|game|this|that|the\s+\w+|\w+\s+game)\s+(?:\w+\s+){0,3}?(?:crash(?:es|ed|ing)?|freez(?:es|ing)|froze|hangs?|hung|stutters?|stuttering|flicker\w*|lags?|segfaults?|errors?)\b[^.!?;\n]{0,50}"),
    ("persistent", r"\b(?:i|we)\s+(?:\w+\s+){0,3}?(?:crash(?:es|ed|ing)?|freez(?:es|ing)|froze)\b[^.!?;\n]{0,50}"),
    ("persistent", r"\b(?:crash(?:es)?|freezes?)\s+(?:like\s+that\s+)?(?:happen\w*|tend\w*|occur\w*)\b[^.!?;\n]{0,40}"),
    ("persistent", r"\b(?:does\s+not|doesn['’]?t)\s+meet\b[^.!?;\n]{0,40}"),
    ("persistent", r"\berror\s+with\b[^.!?;\n]{0,30}"),
    # Regressions.
    ("regression", r"\b(?:made|makes|make|got|gets|getting|became|becomes|even|much|way)\s+(?:\w+\s+){0,2}?worse\b[^.!?;\n]{0,20}"),
    ("regression", r"\b(?:broke|breaks|broken|regress\w*|no\s+longer\s+(?:work\w*|start\w*|launch\w*|run\w*|load\w*|boot\w*)|stopped\s+working|started\s+(?:crash\w*|freez\w*)|n['’]t\s+even\s+start\w*(?:\s+anymore)?)\b[^.!?;\n]{0,30}"),
    ("regression", r"\b(?:became|become|becomes)\s+(?:\w+\s+){0,2}?(?:stutter\w*|unplayable|laggy|sluggish|choppy|slow\w*)\b"),
    ("regression", r"\b(?:caused?|causes|causing)\b[^.!?;\n]{0,40}?\b(?:" + SYMPTOM + r"|distort\w*)\b[^.!?;\n]{0,20}"),
    # Improvements.
    ("improved", r"\b(?:no|without|zero|any\s+more)\s+(?:\w+\s+){0,2}?" + SYMPTOM + r"\b"),
    ("improved", r"\b(?:not|never|haven['’]?t|hasn['’]?t|didn['’]?t|don['’]?t)\s+(?:had|have|see|seen|got|get|experienced?)\s+(?:any\s+|the\s+|a\s+)?(?:\w+\s+){0,2}?" + SYMPTOM + r"\b[^.!?;\n]{0,20}"),
    ("improved", r"\bno\s+longer\s+(?:crash\w*|freez\w*|hang\w*|stutter\w*|flicker\w*|get\w*)\b[^.!?;\n]{0,30}"),
    ("improved", r"\b(?:fixe[sd]|fixing|solve[sd]|solving|resolve[sd]|resolving|cure[sd]|improve[sd]?|improving|help(?:s|ed)|disappear\w*|gone|went\s+away|better|smoother|solution|workaround)\b[^.!?;\n]{0,50}"),
    ("improved", r"\b(?:was|were|am|are|is)\s+(?:finally\s+|now\s+|then\s+)?able\s+to\s+(?:\w+\s+)?(?:play|start|launch|finish|get|run|load|reach|complete|continue|beat|connect|overcome|progress|enter|join|save)\b[^.!?;\n]{0,50}"),
    ("improved", r"\b(?:could|can)\s+(?:now\s+|finally\s+)?(?:play|start|launch|finish|get|run|load|continue|reach)\b[^.!?;\n]{0,40}"),
    ("improved", r"\b(?:allow(?:s|ed)?|let(?:s)?|lets)\s+(?:me|us|it)\b[^.!?;\n]{0,50}"),
    ("improved", r"\b(?:gets?|got|getting)\s+(?:\S+\s+){0,5}?(?:past|farther|further)\b[^.!?;\n]{0,40}"),
    ("improved", r"\b" + RUN_VERB + r"\b\s+(?:\w+\s+){0,2}?" + GOOD_STATE + r"\b[^.!?;\n]{0,25}"),
    ("improved", r"\b(?:it|game|this|that|everything|the\s+\w+)\s+(?:now\s+|does\s+|finally\s+|seems?\s+to\s+|also\s+|still\s+)*(?:work|works|worked|runs?|ran|launch(?:es|ed)?|loads?|loaded|boots?|booted|starts?|started)\b[^.!?;\n]{0,20}"),
    ("improved", r"\bworking\s+(?:perfectly|fine|well|great|for\s+me|now|again)\b[^.!?;\n]{0,20}"),
    ("improved", r"\b(?:was|is)\s+working\b[^.!?;\n]{0,20}"),
]
COMPILED = [(k, re.compile("(?i)" + p)) for k, p in PATTERNS]
PURPOSE = re.compile(r"(?i)\b(?:to|in order to)\s+$")


def masked_regions(text):
    return [m.span() for m in re.finditer(r"```[\s\S]*?(?:```|\Z)|~~[\s\S]*?~~|<!--[\s\S]*?-->|https?://\S+", text)]


def inside(pos, regions):
    return any(a <= pos < b for a, b in regions)


def sentence_of(parsed, pos):
    for s in parsed["sentences"]:
        if s["span"][0] <= pos < s["span"][1]:
            return s
    return parsed["sentences"][-1] if parsed["sentences"] else None


def clause_bounds(text, sentence, pos):
    a, b = sentence["span"]
    cuts = [a] + [a + m.end() for m in re.finditer(r"(?i)\b(?:but|however|whereas|though|although|while)\b|;|,\s+(?=(?:it|the|i|we)\b)", text[a:b])] + [b]
    left = max(x for x in cuts if x <= pos)
    right = min((x for x in cuts if x > pos), default=b)
    return left, right


def head_word(parsed, a, b):
    hits = [w for w in parsed["words"] if a <= w["span"][0] < b]
    if not hits:
        return None
    verbs = [w for w in hits if w["pos"] in ("VERB", "AUX", "ADJ")]
    return (verbs or hits)[0]


def outcomes(text, parsed, row, mentions):
    """Every clause-level result statement. Hedged or questioned ones are not results."""
    found, regions = [], masked_regions(text)
    for kind, pattern in COMPILED:
        for m in pattern.finditer(text):
            a, b = m.span()
            # The result ends where a connective introduces the action or next clause.
            tail = re.search(r"(?i)[,:]|\s(?:when|whenever|after|by|using|with|while|before|since|if|because|so|but|and\s+(?:i|it|the|then))\b", m[0][3:])
            if tail:
                b = a + 3 + tail.start()
            b = a + len(text[a:b].rstrip(" ,:-"))
            if b <= a or inside(a, regions):
                continue
            s = sentence_of(parsed, a)
            if s is None:
                continue
            la, lb = clause_bounds(text, s, a)
            raw = text[slice(*s["span"])]
            # Negation and hedges do not cross a sentence mark or a struck-through part.
            before = re.split(r"[.!?:;~\n]", text[la:a])[-1]
            if "?" in text[la:lb] or HYPOTHETICAL.search(before[-45:]) and not re.search(r"(?i)\bit works if\b", text[la:b]):
                continue
            if PURPOSE.search(before):
                continue
            # A subordinate time clause sets the scene; it does not report a result.
            if kind == "improved" and re.search(r"(?i)\b(?:before|when|whenever|once|until|after|while)\s+(?:\w+\s+){0,3}$", before):
                continue
            outcome = kind
            # A negator just before an improvement that does not encode its own turns it around.
            if kind == "improved" and not re.match(r"(?i)(?:no|without|zero|not|never|haven|hasn|didn|don)\b", m[0]):
                if NEGATOR.search(before[-32:]):
                    outcome = "persistent"
            if kind == "persistent" and re.search(r"(?i)\b(?:no|not|never|without)\s+(?:\w+\s+)?$", before[-24:]) and not re.match(r"(?i)(?:still|same)", m[0]):
                outcome = "improved"
            word = head_word(parsed, a, b)
            if word is None:
                continue
            found.append(dict(evidence=evidence(text, a, b), outcome=outcome, node=word["i"], sentence=word["sentence"], lemma=word["lemma"], modality="reported", causality_confirmed=False, source="clause-pattern", raw_sentence_question="?" in raw))
    found.sort(key=lambda o: (o["evidence"]["span"][0], -(o["evidence"]["span"][1] - o["evidence"]["span"][0])))
    kept = []
    for o in found:
        a, b = o["evidence"]["span"]
        clash = [k for k in kept if min(b, k["evidence"]["span"][1]) > max(a, k["evidence"]["span"][0])]
        if not clash:
            kept.append(o)
        elif all(c["outcome"] != o["outcome"] for c in clash) and b - a > 12 and all(a >= c["evidence"]["span"][1] - 4 for c in clash):
            kept.append(o)
    for o in kept:
        o["identity"] = identity(text, parsed, o["node"], row, mentions)
    return kept


def light_verb(words, w):
    """`tried fiddling` is one action; its content verb identifies it."""
    if w["lemma"].lower() not in ("try", "attempt", "manage", "able"):
        return None
    for c in children(words, w["i"]):
        if c["rel"] in ("xcomp", "ccomp") and c["pos"] == "VERB":
            return c
    return None


def assert_status(text, words, w, status):
    """Completed-action constructions the older state rules leave unknown."""
    a = w["span"][0]
    before = text[max(0, a - 42):a]
    if status in ("unknown", "suggested", "preexisting"):
        if re.search(r"(?i)\b(?:was|were|am)\s+(?:finally\s+)?able\s+to\s+$|\bmanaged\s+to\s+$|\bended\s+up\s+$|\bfinally\s+$", before):
            return "performed"
        if re.search(r"(?i)\bno\s+(?:trouble|problem|issue)s?\s+$", before):
            return "performed"
        if re.search(r"(?i)\b(?:tried|tested)\s+(?:to\s+)?$", before):
            return "performed"
    return status


def mentions(text, bundle, row):
    """All action mentions with modality kept as an attribute."""
    parsed = bundle["parsed"]
    words = parsed["words"]
    regions = masked_regions(text)
    out = []
    for e in bundle["events"]:
        # Perceiving or possessing is not an intervention.
        if e["lemma"] in ("see", "have", "get", "know"):
            continue
        own = children(words, e["node"])
        subject = [x for x in own if x["rel"].startswith("nsubj")]
        if subject and subject[0]["pos"] == "NOUN" and subject[0]["lemma"].lower() not in GENERIC_SUBJECT and e["status"] != "performed":
            continue
        out.append(dict(e, origin="v6-grounding"))
    taken = {e["node"] for e in out}
    for w in words:
        lemma = w["lemma"].lower()
        token = w["text"].lower()
        stem = next((x for x in EXTRA_ACTIONS if token.startswith(x) and token[len(x):] in ("", "e", "es", "s", "ed", "d", "ing", "ting", "ping", "ging", "ning", "ted", "ped", "ged", "ned")), None)
        if w["i"] in taken or inside(w["span"][0], regions):
            continue
        gerund = token.endswith("ing") or token == "login"
        if not (lemma in EXTRA_ACTIONS and w["pos"] == "VERB" or stem and (w["pos"] == "VERB" or w["pos"] == "NOUN" and gerund)):
            continue
        if re.match(r"(?i)\s+options?\b", text[w["span"][1]:w["span"][1] + 10]):
            continue
        own = children(words, w["i"])
        subjects = [x for x in own if x["rel"].startswith("nsubj")]
        # `the game forces you` and `the game loads` are not people acting.
        if subjects and subjects[0]["lemma"].lower() not in GENERIC_SUBJECT:
            continue
        status, cue = prior_state(text, words, w)
        out.append(dict(
            trigger=evidence(text, *w["span"]), status=status, direction="unknown", status_evidence=cue, arguments=[],
            target_identity=None, target_evidence=None, role="corrective", node=w["i"], sentence=w["sentence"], lemma=lemma,
            causal_effect_established=False, execution_authorized=False, reliability="experimental-typed-operation-proposal",
            origin="v7-extended-lexicon",
            identity=identity(text, parsed, w["i"], row, bundle["game_mentions"]),
            event_id=digest(str(row.get("document_id")) + ":" + str(w["span"]) + ":" + digest(text)),
            assertion_type="external-source-report", automatic_approval=False,
        ))
        taken.add(w["i"])
    canon = []
    for e in out:
        w = words[e["node"]]
        content = light_verb(words, w)
        e = dict(e)
        e["status"] = assert_status(text, words, w, e["status"])
        if content is not None and content["i"] not in taken:
            a, b = content["span"]
            # The span names the content verb; the light verb stays available.
            e["light_trigger"] = e["trigger"]
            e["trigger"] = evidence(text, a, b)
            e["node"] = content["i"]
            e["lemma"] = content["lemma"].lower()
            taken.add(content["i"])
        elif content is not None:
            # Both were extracted: the light verb asserts the content verb's execution.
            for other in out:
                if other["node"] == content["i"] and other["status"] != "performed" and w["tag"] in ("VBD", "VBN"):
                    other["status"] = "performed"
            continue
        canon.append(e)
    canon.sort(key=lambda e: e["trigger"]["span"])
    return canon


LIST_ITEM = re.compile(r"(?m)^\s*(?:[-*•]|\d+[.)])\s+")
ALTERNATIVE = re.compile(r"(?i)\b(?:none|neither|nor|any\s+of|all\s+of|each\s+of|both|individually|separately|one\s+by\s+one|or\b|also\s+tried|as\s+well)\b")
SEQUENCE = re.compile(r"(?i)\b(?:then|after\s+that|afterwards?|next|followed\s+by|before\s+that|finally|first)\b")
ISOLATED = re.compile(r"(?i)\b(?:only|alone|just|solely|the\s+key|the\s+fix\s+was|what\s+(?:fixed|did\s+it)|that\s+did\s+it|by\s+itself|on\s+its\s+own|was\s+enough)\b")


def structure(text, group, obs, events):
    """How several actions relate to one result, read only from the text."""
    if len(group) == 1:
        return "single"
    members = [events[i] for i in group]
    a = min(e["trigger"]["span"][0] for e in members)
    b = max(e["trigger"]["span"][1] for e in members)
    oa, ob = obs["evidence"]["span"]
    gaps = text[a:b]
    around = text[max(0, min(a, oa)):min(len(text), max(b, ob) + 20)]
    result = text[oa:ob]
    if re.search(r"(?i)\b(?:none|neither|all\s+(?:of\s+)?(?:them|these|those)|any\s+of|each\s+of|both)\b", around) or (LIST_ITEM.search(gaps) and obs["outcome"] != "improved") or re.search(r"(?i)\b(?:also\s+tried|or\b)", gaps):
        return "alternatives"
    if SEQUENCE.search(gaps) or len({e["sentence"] for e in members}) > 1:
        return "sequence"
    return "joint"


def attribution(text, group, obs, events, shape):
    """Never individual credit from co-occurrence or temporal order."""
    if shape == "single":
        return dict(kind="single-reported", individual_evidence=[], individual_cause="unknown")
    if shape == "alternatives":
        # Separate tries with one reported outcome: the report is about each try,
        # but negative or positive efficacy for this environment stays unverified.
        return dict(kind="distributive-attempts", individual_evidence=[], individual_cause="unknown")
    isolated = []
    oa = obs["evidence"]["span"][0]
    for i in group:
        s = events[i]["trigger"]["span"]
        window = text[max(0, s[0] - 25):min(len(text), s[1] + 25)]
        if ISOLATED.search(window) and abs(s[0] - oa) < 120:
            isolated.append(i)
    return dict(kind="joint-unattributed", individual_evidence=isolated, individual_cause="unknown")


def proposals(text, events, observed, parsed, window=2, cap=4):
    """Bounded candidates per result: singles, conjunct clusters, and the attempt run."""
    words = parsed["words"]
    out = []
    for oi, o in enumerate(observed):
        a, b = o["evidence"]["span"]
        near = [i for i, e in enumerate(events) if abs(e["sentence"] - o["sentence"]) <= window and abs(e["trigger"]["span"][0] - a) < 650 and e["node"] != o["node"] and e["status"] not in ("negated",) and not a <= e["trigger"]["span"][0] < b]
        if not near:
            continue
        previous = max([x["evidence"]["span"][1] for x in observed if x["evidence"]["span"][1] <= a], default=0)
        run = [i for i in near if previous <= events[i]["trigger"]["span"][0] < a]
        options = {(i,) for i in near}
        if 1 < len(run) <= 8:
            options.add(tuple(run))
        firm = tuple(i for i in run if events[i]["status"] in ASSERTED)
        if 1 < len(firm) <= 8:
            options.add(firm)
        # Conjoined or comma-listed mentions of the same clause form a cluster.
        ordered = sorted(near, key=lambda i: events[i]["trigger"]["span"][0])
        cluster = []
        for i in ordered:
            if cluster and (events[i]["sentence"] != events[cluster[-1]]["sentence"] or not re.search(r"(?i)\b(?:and|then|plus|also)\b|,|\+|&", text[events[cluster[-1]]["trigger"]["span"][1]:events[i]["trigger"]["span"][0]])):
                if len(cluster) > 1:
                    options.add(tuple(cluster))
                cluster = []
            cluster.append(i)
        if len(cluster) > 1:
            options.add(tuple(cluster))
        scored = sorted(options, key=lambda g: -structural_score(text, g, o, events, words, observed))
        for g in scored[:cap]:
            out.append(dict(observation=oi, event_indices=list(g), score=structural_score(text, g, o, events, words, observed)))
    return out


ASSERTED = {"performed"}
REPORTED_STATE = {"preexisting", "unknown"}


def structural_score(text, group, obs, events, words, observed):
    members = [events[i] for i in group]
    a, b = obs["evidence"]["span"]
    nearest = min(abs(e["trigger"]["span"][0] - a) for e in members)
    left = min([a] + [e["trigger"]["span"][0] for e in members])
    right = max([b] + [e["trigger"]["span"][1] for e in members])
    span = text[left:right]
    between = any(left < o["evidence"]["span"][0] < right and o is not obs for o in observed)
    direct = sum(any(p["i"] == obs["node"] for p in ancestors(words, e["node"])[1:]) for e in members) / len(members)
    governs = sum(any(p["i"] == e["node"] for p in ancestors(words, obs["node"])[1:]) for e in members) / len(members)
    same = all(e["sentence"] == obs["sentence"] for e in members)
    before = sum(e["trigger"]["span"][0] < a for e in members) / len(members)
    cue = bool(re.search(r"(?i)\b(?:after|by|since|using|with|when|once|fixed by|solved by)\b", span))
    asserted = sum(e["status"] in ASSERTED for e in members) / len(members)
    plausible = sum(e["status"] in ASSERTED | REPORTED_STATE for e in members) / len(members)
    return 2.0 * direct + 1.0 * governs + 1.2 * same + 0.8 * before + 0.6 * cue + 1.2 * asserted + 0.8 * plausible - 0.8 * nearest / 200 - 1.5 * between + 0.35 * (len(members) - 1) * same


def decide(text, bundle, row, threshold=2.6):
    """One candidate per result, or abstention. Text-level association only."""
    events, observed, parsed = bundle["events"], bundle["observations"], bundle["parsed"]
    pool = proposals(text, events, observed, parsed)
    accepted, rejected = [], []
    by = {}
    for p in pool:
        by.setdefault(p["observation"], []).append(p)
    for oi, ps in by.items():
        o = observed[oi]
        ps = sorted(ps, key=lambda p: -p["score"])
        best = None
        for p in ps:
            members = [events[i] for i in p["event_indices"]]
            if any(e["status"] == "suggested" for e in members):
                rejected.append(dict(observation=oi, event_indices=p["event_indices"], reason="suggestion-not-an-attempt"))
                continue
            if any(e["identity"].get("actor_kind") == "addressee" for e in members):
                rejected.append(dict(observation=oi, event_indices=p["event_indices"], reason="addressee-not-reporter"))
                continue
            if any(e["identity"].get("game_app_id") and o["identity"].get("game_app_id") and e["identity"]["game_app_id"] != o["identity"]["game_app_id"] for e in members):
                rejected.append(dict(observation=oi, event_indices=p["event_indices"], reason="different-explicit-game"))
                continue
            best = p
            break
        if best is None:
            continue
        if best["score"] < threshold:
            rejected.append(dict(observation=oi, event_indices=best["event_indices"], reason="insufficient-structural-support", score=best["score"]))
            continue
        shape = structure(text, best["event_indices"], o, events)
        claim = attribution(text, best["event_indices"], o, events, shape)
        states = {events[i]["status"] for i in best["event_indices"]}
        accepted.append(dict(
            o, event_indices=best["event_indices"], joint=len(best["event_indices"]) > 1, structure=shape, attribution=claim,
            relation="v7-structural-decision", verification="supported-as-source-reported-association", score=float(best["score"]),
            score_is_calibrated_probability=False, execution_supported=states <= ASSERTED,
            owner_modalities=sorted(states), individual_cause="unknown", causality_confirmed=False,
            current_environment_verified=False, execution_authorized=False, automatic_approval=False,
        ))
    return accepted, rejected, pool


def analyze(row, parser, catalog, threshold=2.6):
    """Same output schema as v6, so the v6 evaluator applies unchanged."""
    text = row["text"]
    bundle = deconstruct(row, parser, catalog, True)
    events = mentions(text, bundle, row)
    observed = outcomes(text, bundle["parsed"], row, bundle["game_mentions"])
    bundle = dict(bundle, events=events, observations=observed)
    accepted, rejected, pool = decide(text, bundle, row, threshold)
    return dict(
        events=events, observations=accepted, detected_observations=observed, proposals=pool, temporal=bundle["temporal"],
        verification_rejections=rejected, game_mentions=bundle["game_mentions"],
        unattributed_observations=[o for o in observed if not any(x["node"] == o["node"] for x in accepted)],
        automatic_approval=False, execution_authorized=False, parsed=bundle["parsed"],
    )
