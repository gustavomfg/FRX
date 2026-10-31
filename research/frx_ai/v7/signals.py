"""Typed technical signals with literal evidence. Deterministic; supplied text only.

Portuguese symptom terms exist for the lab CLI and were never evaluated: C7 is English.
"""
import re

SYMPTOMS = [
    ("startup-failure", r"(?i)\bn[ãa]o (?:inicia|abre|roda|funciona|carrega)\b|\bfecha sozinho\b|\b(?:won'?t|doesn'?t|does not|fails? to|not|unable to|can'?t|cannot)\s+(?:launch|start|boot|open|run)\b|\bno(?:thing)?\b[^.\n]{0,20}\bhappens\b|\bfails? at startup\b|\bnever launches\b|\bcrash\w* (?:on|at|during) (?:launch|startup|start)\b|\bimmediately (?:closes|crashes)\b|\bclos\w+ (?:itself|immediately)\b"),
    ("crash", r"(?i)\bfecha sozinho\b|\bcrasha\w*\b|\bcrash\w*\b|\bsegfault\w*\b|\bCTD\b|\bcloses? itself\b"),
    ("freeze", r"(?i)\btrav\w+\b|\bcongel\w+\b|\bfreez\w*\b|\bfroze\b|\bhangs?\b|\bhung\b|\block(?:s|ed)? up\b|\bdeadlock\w*\b"),
    ("black-screen", r"(?i)\btela (?:preta|branca)\b|\bblack[- ]screen\b|\bwhite screen\b|\bblank screen\b"),
    ("graphics", r"(?i)\bgr[áa]ficos? (?:quebrad|distorcid)\w*\b|\bgeometria distorcida\b|\b(?:flicker\w*|artifact\w*|corrupt\w*|glitch\w*|distort\w*|missing textures?|rendering (?:issue|bug|problem)|transparen\w*|graphic\w* (?:issue|bug|problem))\b"),
    ("performance", r"(?i)\bpoucos fps\b|\blento\b|\b(?:low (?:fps|performance)|bad performance|poor performance|fps drop\w*|slow\w*|lag\w*|desempenho|performance (?:issue|problem|regression)|ruim)\b|\bFPS\b"),
    ("stutter", r"(?i)\bengasg\w+\b|\bstutter\w*\b|\bmicro-?stutter\w*\b|\bhitch\w*\b|\bframe[- ]?(?:time|pacing)\b"),
    ("audio", r"(?i)\bsem som\b|\b[áa]udio\b|\b(?:no sound|no audio|sound|audio|crackl\w*|music)\b"),
    ("video", r"(?i)\b(?:cutscenes?|video playback|videos?(?!\s+(?:driver|card|memory|settings?|options?))|movies?|mfplat|media foundation)\b"),
    ("input", r"(?i)\bcontrole\b|\b(?:controller|gamepad|joystick|keyboard|mouse|dualsense|steam input)\b"),
    ("online", r"(?i)\b(?:multiplayer|online|server|connection|anti-?cheat|EAC|easy ?anti ?cheat|battl?eye|matchmaking)\b"),
    ("physx", r"(?i)\bphysx\b"),
]

LOG_SIGNALS = [
    ("undefined-symbol", r"(?im)^[^\n]*undefined symbol:\s*(?P<symbol>[A-Za-z_][\w@.]*)[^\n]*"),
    ("missing-library", r"(?im)^[^\n]*(?:Library\s+(?P<library>[\w.+-]+\.(?:dll|so[\w.]*)|[\w.+-]+)\s*\(which is needed by[^\n]*not found|(?P<library2>[\w.+-]+\.so[\w.]*):\s*cannot open shared object file)[^\n]*"),
    ("unimplemented-function", r"(?im)^[^\n]*unimplemented function\s+(?P<function>[\w.]+)[^\n]*"),
    ("unhandled-exception", r"(?im)^[^\n]*(?:Unhandled (?:page fault|exception)|Exception 0xc0000\w+|EXCEPTION_ACCESS_VIOLATION|page fault on (?:read|write|execute))[^\n]*"),
    ("native-segfault", r"(?im)^[^\n]*(?:Segmentation fault|SIGSEGV|core dumped|Assertion [`'\"]?.{0,60}failed)[^\n]*"),
    ("device-lost", r"(?im)^[^\n]*(?:VK_ERROR_DEVICE_LOST|DEVICE_LOST|DXGI_ERROR_DEVICE_(?:REMOVED|HUNG)|GPU hang|ring gfx timeout|NVRM: Xid)[^\n]*"),
    ("graphics-api-error", r"(?im)^[^\n]*(?:err:(?:d3d|vkd3d|dxgi|vulkan|d3d12|dxvk)[\w:]*|VK_ERROR_[A-Z_]+|vkCreate\w+ failed|DxvkError|No Vulkan|failed to create (?:device|swapchain))[^\n]*"),
    ("prefix-error", r"(?im)^[^\n]*(?:OSError: \[Errno|FileExistsError|PermissionError|Traceback \(most recent call last\)|pfx[^\n]{0,60}(?:corrupt|not empty))[^\n]*"),
    ("anti-cheat", r"(?im)^[^\n]*(?:EasyAntiCheat|EAC\b|BattlEye|BEService|Launcher error\s+\S*EAC)[^\n]*"),
    ("audio-error", r"(?im)^[^\n]*err:(?:pulse|alsa|mmdevapi|dsound|xaudio|winepulse|winealsa)[^\n]*"),
    ("media-error", r"(?im)^[^\n]*(?:err:(?:mfplat|quartz|gstreamer|winegstreamer|wmvcore)[^\n]*|gstreamer[^\n]*(?:error|fail)[^\n]*|Missing decoder[^\n]*)"),
    ("runtime-container", r"(?im)^[^\n]*(?:pressure-vessel[^\n]*(?:error|fail|warning)|steam-runtime[^\n]*(?:error|fail)|sniper[^\n]*(?:error|fail))[^\n]*"),
    ("sync-timeout", r"(?im)^[^\n]*(?:err:sync:|wait timed out in thread)[^\n]*"),
]
CONTEXT = dict(
    gpu_vendor=[("nvidia", r"(?i)\b(?:nvidia|geforce|gtx|rtx)\b"), ("amd", r"(?i)\b(?:amd|radeon|radv|rx ?\d{3,4})\b"), ("intel", r"(?i)\b(?:intel arc|intel hd|intel uhd|anv\b|iris)\b")],
)
PROTON = re.compile(r"(?i)\b(?:GE-Proton[\w.-]*|proton[- ]?(?:experimental|hotfix|cachyos[\w.-]*|ge[\w.-]*|\d+(?:\.\d+)?(?:-\d+)?))\b")
DRIVER = re.compile(r"(?i)\b(?:nvidia\s+\d{3}(?:\.\d+)*|\d{3}\.\d{2,3}(?:\.\d+)?|mesa\s+\d+(?:\.\d+)*)\b")
OPTION = re.compile(r"\b(?:PROTON_[A-Z0-9_]+|WINEDLLOVERRIDES|DXVK_[A-Z_]+|VKD3D_[A-Z_]+|RADV_[A-Z_]+|LD_[A-Z_]+|SDL_[A-Z_]+)(?:=\S+)?|%command%")


def symptoms(text):
    out = []
    for name, pattern in SYMPTOMS:
        m = re.search(pattern, text or "")
        if m:
            out.append(dict(kind=name, quote=m[0], span=list(m.span()), source="supplied-problem-text"))
    return out


def log_signals(log, source="supplied-log"):
    found = []
    for name, pattern in LOG_SIGNALS:
        seen = set()
        for m in re.finditer(pattern, log or ""):
            line = m[0].strip()
            key = re.sub(r"[\da-fx]{4,}", "#", line)[:80]
            if key in seen or len(seen) >= 3:
                continue
            seen.add(key)
            groups = {k: v for k, v in m.groupdict().items() if v}
            if "library2" in groups:
                groups["library"] = groups.pop("library2")
            a = m.start() + m[0].index(line)
            found.append(dict(kind=name, quote=line[:400], span=[a, a + len(line[:400])], details=groups, source=source))
    return found


def environment(text):
    """Only what the text states; a mention is not proof of what ran."""
    out = {}
    for vendor, pattern in CONTEXT["gpu_vendor"]:
        if re.search(pattern, text or ""):
            out.setdefault("gpu_vendor_mentions", []).append(vendor)
    if PROTON.search(text or ""):
        out["proton_mentions"] = sorted(set(x[0] if isinstance(x, tuple) else x for x in PROTON.findall(text)))[:6]
    if DRIVER.search(text or ""):
        out["driver_mentions"] = sorted(set(DRIVER.findall(text)))[:4]
    options = sorted(set(OPTION.findall(text or "")) - {"%command%"})
    if options:
        out["options_mentioned"] = options[:12]
    return out
