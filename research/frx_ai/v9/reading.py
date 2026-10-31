"""Evidence reader and evaluator. Everything external stays data: rules read it, nothing obeys it.

Facts that are known, unknown and never invented; technical terms; the nature of each claim;
and whether a passage applies to the game and the environment.
"""
import re
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent / "v7"))
sys.path.insert(0, str(HERE.parent / "v8"))
import detector  # noqa: E402
import logs  # noqa: E402
from signals import symptoms  # noqa: E402

COMPONENTS = {
    "physx", "dxvk", "vkd3d", "vkd3d-proton", "wined3d", "gamescope", "winetricks", "protontricks", "mfplat", "mf", "winegstreamer", "gstreamer", "xact", "faudio", "pipewire", "pulseaudio", "alsa", "ntsync", "fsync", "esync",
    "nvapi", "dlss", "fsr", "mangohud", "easyanticheat", "eac", "battleye", "h264", "hdr", "wayland", "xwayland", "x11", "d3dcompiler", "dotnet", "vcrun", "directshow", "xaudio2", "steamworks", "pressure-vessel", "sniper", "soldier",
    "steam-runtime", "umu", "protonfixes", "d3d8", "d3d9", "d3d10", "d3d11", "d3d12", "dxgi", "opengl", "vulkan", "mesa", "radv", "nvk", "anv", "amdgpu", "nouveau", "prime", "shader", "lutris", "wmp", "quartz", "cinepak", "xinput", "sdl", "hidraw", "dualsense", "steam-input", "large-address-aware", "laa", "cpu-topology", "split-lock", "seccomp", "ubisoft", "ea-app", "uplay", "denuvo", "crackproof", "vsync", "reshade",
}
GENERIC = {"steam", "proton", "wine", "linux", "game", "windows", "kernel", "log", "exe", "nvidia", "amd", "intel"}
STOPWORDS = {"the", "and", "with", "for", "game", "games", "goty", "edition", "remastered", "definitive", "deluxe", "enhanced", "complete", "ultimate", "director", "directors", "cut", "of", "a", "an", "to", "in"}
TERM_PATTERNS = [
    ("library", re.compile(r"(?<![\w/\\.-])[A-Za-z][\w.+-]{1,40}\.(?:dll|so(?:\.\d+)*|drv|sys|ocx)\b", re.I)),
    ("option", re.compile(r"\b(?:PROTON_[A-Z0-9_]+|WINE[A-Z0-9_]*|DXVK_[A-Z0-9_]+|VKD3D_[A-Z0-9_]+|RADV_[A-Z0-9_]+|MESA_[A-Z0-9_]+|__GL_[A-Z0-9_]+|SteamDeck)\b")),
    ("error", re.compile(r"\b(?:VK_ERROR_[A-Z_]+|E_[A-Z_]{4,}|STATUS_[A-Z_]{4,}|c0000[0-9a-f]{3}|0x[0-9a-fA-F]{8}|err:[a-z0-9_]+:[A-Za-z0-9_]+)\b")),
    ("version", re.compile(r"\b(?:GE-Proton[\w.-]*\d|Proton[- ]\d+(?:\.\d+)*(?:-\d+)?|Mesa[- ]\d+\.\d+(?:\.\d+)?|NVIDIA[- ]\d{3}(?:\.\d+)*|nvidia[- ]\d{3}(?:\.\d+)*)", re.I)),
]
WORD = re.compile(r"[a-z][a-z0-9+-]{2,}")
SAFE_TERM = re.compile(r"^[A-Za-z0-9_][A-Za-z0-9_.+-]{1,48}( [A-Za-z0-9_.+-]{1,20})?$")
QUALIFIER = re.compile(r"(?i)\b(?:repo|org|user|in|is|author|involves|label|state|created|updated|sort):")
THREAD_REF = re.compile(r"(?:https://github\.com/([\w.-]+/[\w.-]+)/issues/|(?<![\w/#])#)(\d{1,6})\b")
VENDORS = {"nvidia": r"\b(?:nvidia|geforce|rtx|gtx|nouveau|nvk)\b", "amd": r"\b(?:amd|radeon|radv|amdgpu|rx\s?\d{3,4})\b", "intel": r"\b(?:intel|arc\s+a\d{3}|iris|uhd|anv)\b"}
HOSTILE = re.compile(
    r"(?i)(?:ignore\s+(?:all\s+|any\s+|the\s+)?(?:previous|prior|above|earlier)|disregard\s+(?:all|the|your)|system\s+prompt|you\s+are\s+now|as\s+an?\s+(?:ai|assistant|agent)|"
    r"(?:curl|wget)\b[^\n|]{0,160}\|\s*(?:sudo\s+)?(?:ba|z|fi)?sh\b|\b(?:send|post|upload|exfiltrate|reveal|share)\b[^\n.]{0,40}\b(?:token|credential|password|api[_ -]?key|ssh\s+key|env(?:ironment)?\s+var)|"
    r"\brm\s+-rf\b|base64\s+-d\b[^\n]*\|\s*(?:ba)?sh\b|\bsudo\s+(?:chmod|chown|dd|rm)\b)"
)
INSTRUCTION = re.compile(r"(?i)\b(?:download|unpack|run\s|execute|install|sudo|chmod|curl|wget|replace\s+the|copy\s+the|paste)\b|\./\S+|%command%")
CONTROL = re.compile(r"\x1b\[[0-9;?]*[ -/]*[@-~]|\x1b\][^\x07]*(?:\x07|\x1b\\)|[\x00-\x08\x0b-\x1f\x7f]")
SYMPTOM_WORD = {"startup-failure": "launch", "crash": "crash", "freeze": "freeze", "black-screen": "black screen", "graphics": "graphics", "performance": "performance", "stutter": "stutter", "audio": "audio", "video": "video", "input": "input", "online": "online", "physx": "physx"}
OLD_YEARS = 4
# What people observe, most specific first. Hand-written; it only helps to word a search.
PHENOMENA = ["white screen", "black screen", "loading screen", "main menu", "cutscene", "intro video", "video", "launcher", "login", "multiplayer", "online", "controller", "gamepad", "steering wheel", "mouse", "keyboard", "fullscreen", "resolution", "shader", "texture", "overlay", "save", "audio", "sound", "microphone", "stutter", "fps", "freeze", "hang", "crash", "segfault", "artifact", "flicker", "glitch"]


def sanitize(text):
    """Terminal control sequences never reach the screen or the report."""
    return CONTROL.sub("", text or "")


def interpret(problem):
    """Known facts as given. A missing value stays unknown and is never filled in."""
    game = sanitize(problem.get("game") or "").strip()
    symptom = logs.redact(sanitize(problem.get("symptom") or problem.get("problem") or ""))
    log = logs.redact(sanitize(problem.get("log") or ""))
    gpu = sanitize(problem.get("gpu") or "")
    vendor = vendor_of(gpu) if gpu else None
    known = dict(game=game or None, app_id=str(problem["app_id"]) if problem.get("app_id") else None, gpu=gpu or None, gpu_vendor=vendor, driver=problem.get("driver"), proton=problem.get("proton"), game_version=problem.get("game_version"), symptom=symptom or None, log_supplied=bool(log))
    unknown = [k for k in ("game", "app_id", "gpu", "driver", "proton", "game_version") if not known.get(k)]
    if not log:
        unknown.append("log")
    text = symptom + "\n" + log
    low = text.lower()
    phenomena = [p for p in PHENOMENA if re.search(rf"\b{re.escape(p)}", low)]
    return dict(known=known, unknown=unknown, symptoms=sorted({s["kind"] for s in symptoms(text)}), phenomena=phenomena, input_terms=terms_of(text + " " + game), game_tokens=game_tokens(game), text=text)


def game_tokens(game):
    words = re.findall(r"[a-z0-9]+", re.sub(r"[™®©]", "", game.lower()))
    return [w for w in words if w not in STOPWORDS and len(w) > 1]


def vendor_of(text):
    found = [k for k, rx in VENDORS.items() if re.search(rx, text, re.I)]
    return found[0] if len(found) == 1 else None


def terms_of(text):
    """Technical terms by explicit pattern and a small hand-written domain vocabulary."""
    out = {}
    clean = re.sub(r"https?://\S+", " ", text or "")
    for kind, pattern in TERM_PATTERNS:
        for m in pattern.finditer(clean):
            out.setdefault(_normal(m[0]), kind)
    for w in WORD.findall(clean.lower()):
        if w in COMPONENTS and w not in GENERIC:
            out.setdefault(w, "component")
    return out


def _normal(term):
    return term.strip().lower().replace(" ", "-")


def safe_term(term):
    """A term from a document enters a query only in this form: no qualifiers, quotes or operators."""
    return bool(SAFE_TERM.match(term)) and not QUALIFIER.search(term)


def references(text, repo):
    """Issue references, returned as repository and number, never as a URL to open."""
    out = []
    for m in THREAD_REF.finditer(text or ""):
        out.append((m.group(1) or repo, int(m.group(2))))
    return out


def paragraphs(text):
    pos = 0
    for block in re.split(r"(\n\s*\n)", text or ""):
        if block.strip():
            yield pos, block
        pos += len(block)


def nature_of(part, paragraph, claims):
    """Who says it comes from API metadata, never from what the text says about its author."""
    if part["association"] == "OFFICIAL_DOC":
        return "official-document"
    if claims:
        kind = "reported-result-maintainer" if part["association"] in ("MEMBER", "OWNER", "COLLABORATOR") else "reported-result-community"
        return kind
    if part["association"] in ("MEMBER", "OWNER", "COLLABORATOR"):
        return "maintainer-statement"
    if re.search(r"(?i)\b(?:might|maybe|perhaps|probably|i\s+think|i\s+guess|could\s+be|seems?\s+like|my\s+guess|suspect)\b", paragraph):
        return "user-hypothesis"
    if re.search(r"(?i)\b(?:tried|tested|installed|removed|set|switched|updated|enabled|disabled)\b", paragraph):
        return "attempt"
    return "community-information"


def same_game(title, facts):
    tokens = facts["game_tokens"]
    if not tokens:
        return None
    app = facts["known"].get("app_id")
    if app and re.search(rf"\(\s*{re.escape(app)}\s*\)", title or ""):
        return True
    words = set(re.findall(r"[a-z0-9]+", (title or "").lower()))
    return sum(t in words for t in tokens) / len(tokens) >= .75


def evaluate(paragraph, doc, facts, created_at, until=None, has_claim=False):
    """Applicability of one passage. Similar is not enough: reasons against are kept."""
    notes = []
    game = same_game(doc["title"], facts)
    vendor = facts["known"].get("gpu_vendor")
    said = vendor_of(paragraph)
    conflict = bool(vendor and said and said != vendor)
    if conflict:
        notes.append(f"trata de {said}; o caso é {vendor}")
    reference = until or facts.get("today")
    if created_at and reference and int(reference[:4]) - int(created_at[:4]) >= OLD_YEARS:
        notes.append(f"antigo ({created_at[:4]}): pode estar obsoleto")
    shared = {s["kind"] for s in symptoms(paragraph)} & set(facts["symptoms"])
    tech = set(terms_of(paragraph)) & set(facts["input_terms"])
    if game is False:
        notes.append("thread de outro jogo: analogia técnica, não relato deste jogo")
    if conflict:
        status = "incompatible"
    elif game:
        status = "same-game"
    elif tech or (shared and has_claim):
        # A shared word like "crash" alone is too weak: it needs a shared term or a reported result.
        status = "technical-analogy"
    else:
        status = "unrelated"
    return dict(status=status, notes=notes, shared_symptoms=sorted(shared), shared_terms=sorted(tech))


def read_document(doc, facts, query_terms=(), until=None, max_passages=40):
    """Passages with literal quotes and provenance, the terms they hold and a hostility flag."""
    out = []
    wanted = {t.lower() for t in query_terms} | set(facts["input_terms"])
    for part in doc["parts"]:
        cleaned = detector.clean(part["text"])
        for start, block in paragraphs(cleaned):
            raw = part["text"][start:start + len(block)]
            if not block.strip() or len(block.strip()) < 15:
                continue
            found = detector.detect(raw)
            terms = terms_of(block)
            hostile = bool(HOSTILE.search(part["text"][start:start + len(block)]))
            lowered = block.lower()
            overlap = sum(t in lowered for t in wanted)
            if not (found["claims"] or terms or overlap):
                continue
            verdict = evaluate(block, doc, facts, part["created_at"], until, bool(found["claims"]))
            first = found["claims"][0] if found["claims"] else None
            a, b = (first["sentence_span"] if first else (0, min(len(raw), 240)))
            quote = sanitize(raw[a:b].strip())[:280]
            out.append(dict(repo=doc["repo"], thread=doc["number"], title=sanitize(doc["title"])[:120], url=part["url"], part_id=part["id"], kind=part["kind"], author=part["author"], association=part["association"], created_at=part["created_at"], updated_at=part["updated_at"], fetched_at=doc["fetched_at"], source=doc["source"], sha256=part["sha256"], quote=quote, offset=start,
                            terms=terms, claims=[dict(cls=c["cls"], quote=sanitize(c["quote"])) for c in found["claims"]], nature=nature_of(part, block, found["claims"]), applicability=verdict, hostile=hostile, thread_state_is_not_a_result=True, full_text=block,
                            references=references(block, doc["repo"])))
    out.sort(key=lambda p: (-sum(t in p["full_text"].lower() for t in wanted), p["offset"]))
    return out[:max_passages]
