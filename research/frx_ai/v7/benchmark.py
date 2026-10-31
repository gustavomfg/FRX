"""Cost of the local path, measured in fresh processes. Python itself is not counted."""
import json
import resource
import statistics
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path[:0] = [str(HERE), str(HERE.parent / "v6")]
root = Path(sys.argv[1])
t0 = time.perf_counter()
from bridge import CompactSyntax, read  # noqa: E402
import events  # noqa: E402
import procedures  # noqa: E402
inherited = events.evidence
events.evidence = lambda text, a, b: inherited(text, min(a, b), max(a, b))
parser = CompactSyntax(root)
catalog = json.loads((root / "catalog.json").read_text())
rows = read(root / "p7-gold.jsonl") + read(root / "j7-gold.jsonl")
parser.cache = root / "benchmark-parse"
parser.cache.mkdir(exist_ok=True)
for f in parser.cache.glob("*.json"):
    f.unlink()
start = time.perf_counter()
procedures.analyze(rows[0], parser, catalog, 3.4)
first = time.perf_counter() - start
cold = []
for r in rows[1:]:
    start = time.perf_counter()
    procedures.analyze(r, parser, catalog, 3.4)
    cold.append((time.perf_counter() - start) * 1000)
warm = []
for r in rows:
    start = time.perf_counter()
    procedures.analyze(r, parser, catalog, 3.4)
    warm.append((time.perf_counter() - start) * 1000)
out = dict(units=len(rows), startup_s=round(start - t0, 3), first_request_s=round(first, 3), parse_ms_median=round(statistics.median(cold), 1), cached_parse_ms_median=round(statistics.median(warm), 2), cached_parse_ms_p95=round(statistics.quantiles(warm, n=20)[18], 2), rss_mib_language_path=round(resource.getrusage(resource.RUSAGE_SELF).ru_maxrss / 1024, 1))
print(json.dumps(out))
(root / "efficiency-language.json").write_text(json.dumps(out, indent=1))
