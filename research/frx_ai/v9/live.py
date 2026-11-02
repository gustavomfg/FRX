"""Live complement: real requests to the public GitHub API, without credentials, for known and test cases.

Waits out the unauthenticated limits instead of insisting. Results are not reproducible and are kept apart.
"""
import json
import random
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import engine  # noqa: E402
import net  # noqa: E402
import sources  # noqa: E402

CACHE = net.CACHE
BATMAN = dict(id="batman-known-demo", input=dict(game="Batman: Arkham City GOTY", app_id="200260", gpu="NVIDIA", symptom="Poor performance with PhysX"), cutoff=None, exclude=[], gold=dict(concepts=["nvcuda", "libcuda"], documented=True))
CONDITIONS = dict(B="single", F="fixed", C="iterative")


def cases():
    rows = [json.loads(x) for x in (CACHE / "test-reserved.jsonl").read_text().splitlines()]
    gold = {g["id"]: g for g in map(json.loads, (CACHE / "test-gold.jsonl").read_text().splitlines())}
    for r in rows:
        r["gold"] = gold[r["id"]]
    documented = [r for r in rows if r["gold"]["documented"]]
    rng = random.Random("frx-v9-live")
    return [BATMAN] + rng.sample(documented, 5) + rng.sample([r for r in rows if not r["gold"]["documented"]], 1)


def wait_for_quota(need):
    while True:
        import http.client
        import ssl
        c = http.client.HTTPSConnection("api.github.com", 443, timeout=15, context=ssl.create_default_context())
        c.request("GET", "/rate_limit", headers={"User-Agent": net.USER_AGENT})
        limits = json.loads(c.getresponse().read())["resources"]
        c.close()
        if limits["core"]["remaining"] >= need and limits["search"]["remaining"] >= 3:
            return limits
        pause = max(10, (limits["core"]["reset"] if limits["core"]["remaining"] < need else limits["search"]["reset"]) - time.time() + 5)
        print(f"waiting {int(pause)}s for quota: core {limits['core']['remaining']}, search {limits['search']['remaining']}", flush=True)
        time.sleep(min(pause, 900))


def text_of(result):
    return "\n".join(f["term"] + " " + " ".join(e["quote"] for e in f["evidence"]) for f in result["findings"]).lower()


def main():
    dest = CACHE / "live-results.json"
    rows = json.loads(dest.read_text()) if dest.exists() else []
    file = CACHE / "live-incidents.json"
    incidents = json.loads(file.read_text()) if file.exists() else []
    for attempt in range(4):
        done = {(r["id"], r["condition"]) for r in rows}
        if all((c["id"], n) in done for c in cases() for n in CONDITIONS):
            break
        run_round(rows, done, incidents, dest)


def run_round(rows, done, incidents, dest):
    for case in cases():
        for name, strategy in CONDITIONS.items():
            if (case["id"], name) in done:
                continue
            limits = wait_for_quota(10)
            client = net.Client(max_requests=25)
            start = time.perf_counter()
            state = engine.investigate(case["input"], sources.GitHub(client), strategy, until=case.get("cutoff"), exclude=set(case.get("exclude", [])))
            limited = [f for f in state.failures if f.get("reason") == "rate-limited"]
            if limited:
                # An invalid run is not a result: wait the limit out and do this condition again.
                pause = max(90, limited[0].get("reset", 0) - time.time() + 30)
                print(f"secondary or primary limit; waiting {int(pause)}s", flush=True)
                incidents.append(dict(id=case["id"], condition=name, at=sources.now(), reset=limited[0].get("reset")))
                (CACHE / "live-incidents.json").write_text(json.dumps(incidents, indent=1))
                time.sleep(min(pause, 3700))
                continue
            result = engine.report(state, strategy)
            read = "\n".join(p["full_text"] for p in state.passages).lower()
            concepts = case["gold"].get("concepts", [])
            row = dict(id=case["id"], condition=name, documented=case["gold"]["documented"], at=sources.now(), seconds=round(time.perf_counter() - start, 1), network_requests=client.network_requests, cached=sum(c["outcome"] == "cache" for c in client.calls), rejections=client.rejections, statuses=sorted({c["outcome"] for c in client.calls}), quota_before=dict(core=limits["core"]["remaining"], search=limits["search"]["remaining"]),
                       queries=result["queries"], repos=sorted({q["repo"] for q in result["queries"]}), sources=result["sources_examined"], status=result["status"], findings=[dict(term=f["term"], status=f["status"], threads=f["threads"], url=f["evidence"][0]["url"] if f["evidence"] else None) for f in result["findings"]], contradictions=[c["term"] for c in result["contradictions"]], next=[c["id"] for c in result["next_checks"]], failures=result["failures"],
                       report_concepts=[c for c in concepts if c in text_of(result)], read_concepts=[c for c in concepts if c in read])
            rows.append(row)
            dest.write_text(json.dumps(rows, indent=1))
            print(case["id"], name, row["network_requests"], row["status"], row["report_concepts"], row["read_concepts"], flush=True)


if __name__ == "__main__":
    main()
