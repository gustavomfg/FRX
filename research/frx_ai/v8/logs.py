"""Log reading with privacy first: fixtures and public excerpts only, redacted, nothing executed.

Signals come from the explicit rules of v7. A log reading never names a cause; it says what
the lines show and what they do not allow concluding.
"""
import re
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
FIXTURES = HERE / "fixtures"
sys.path.insert(0, str(HERE.parent / "v7"))
from signals import log_signals  # noqa: E402

ORIGIN = "rule"
PRIVATE_ROOTS = ("/.steam", "/.local/share/Steam", "/steamapps", "/compatdata", "/.var/app", "/.config", "/.ssh", "/.codex", "/.claude")
REDACTIONS = [
    (re.compile(r"(?i)((?:/home|/Users|/var/home)/)[^/\s\"']+"), r"\1<user>"),
    (re.compile(r"(?i)(C:\\+users\\+)[^\\\s\"']+"), r"\1<user>"),
    (re.compile(r"(?i)(\\+home\\+)[^\\\s\"']+"), r"\1<user>"),
    (re.compile(r"\b[\w.+-]+@[\w-]+\.[\w.-]+\b"), "<email>"),
    (re.compile(r"\b(?:\d{1,3}\.){3}\d{1,3}\b"), "<ip>"),
    (re.compile(r"\b(?:[0-9a-fA-F]{2}:){5}[0-9a-fA-F]{2}\b"), "<mac>"),
    (re.compile(r"(?i)\b(steamid|steam_id|steamuser|accountname|account_name|hostname|token|password|api[_-]?key)(\s*[=:]\s*)\S+"), r"\1\2<redacted>"),
    (re.compile(r"\b7656119\d{10}\b"), "<steamid>"),
]
NOISE = re.compile(r"(?m)^.*\b(?:fixme|trace|warn):[\w-]*:.*$")
ANY_ERROR = re.compile(r"(?im)^.*(?:\berr:|error|fail|fatal|exception|segfault|cannot|undefined symbol|not found).*$")
MODULE = re.compile(r"(?i)\b(?:in\s+(?:module\s+)?|module\s+)[\"']?(?P<module>[\w.-]+\.(?:dll|so|exe)(?:\.\d+)*)|(?:Library|load library)\s+(?P<library>[\w.-]+\.(?:dll|so)(?:\.\d+)*)|\b(?P<object>lib[\w.+-]+\.so(?:\.\d+)*)")


def redact(text):
    for pattern, replacement in REDACTIONS:
        text = pattern.sub(replacement, text)
    return text


def safe_path(path):
    """Fixtures only. Reading any other file needs a new, specific authorization from the owner."""
    resolved = Path(path).resolve()
    if FIXTURES.resolve() not in resolved.parents:
        raise PermissionError(f"recusado: {resolved.name} está fora de fixtures/; arquivos privados não são lidos sem autorização específica")
    return resolved


def read_fixture(name):
    return safe_path(FIXTURES / name).read_text()


def inline_excerpts(text, limit=60):
    """Lines of a public comment that look like a log, redacted. Attachments are never fetched."""
    lines = [x for x in (text or "").splitlines() if re.search(r"\b(?:err|fixme|warn|trace):\w*:|\bwine: |Unhandled exception|undefined symbol|shared object|^\s*\d+\.\d+:", x)]
    return redact("\n".join(lines[:limit]))


def read_log(text):
    """What a log shows. Redacts before reading, so quotes never carry personal data."""
    clean = redact(text or "")
    signals = log_signals(clean)
    modules = sorted({next(v for v in m.groupdict().values() if v) for s in signals for m in MODULE.finditer(s["quote"])})
    noise = len(NOISE.findall(clean))
    limits = ["O log mostra sintomas e pontos de falha; ele não confirma a causa nem indica correção."]
    if not signals:
        limits.append("Nenhum sinal conhecido: isso não significa que não há falha, só que as regras não reconheceram nenhuma.")
    if noise and all(s["kind"] != "unimplemented-function" for s in signals):
        limits.append(f"{noise} linhas fixme/warn/trace são comuns em execuções normais e não indicam falha por si.")
    return dict(signals=[dict(s, origin=ORIGIN) for s in signals], modules=modules, noise_lines=noise, limits=limits, confirmed_cause=None, origin=ORIGIN)


def literal_search(text):
    """Baseline: every line a plain error search would show."""
    return [m[0].strip() for m in ANY_ERROR.finditer(redact(text or ""))]
