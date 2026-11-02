"""Bounded public snapshot of ValveSoftware/Proton issues and comments. Data only."""
import argparse
import json
import subprocess
from pathlib import Path

REPO = "repos/ValveSoftware/Proton"
KEEP_ISSUE = ["number", "title", "body", "state", "created_at", "updated_at", "html_url", "comments", "author_association", "labels", "user", "pull_request"]
KEEP_COMMENT = ["id", "body", "created_at", "updated_at", "html_url", "issue_url", "author_association", "user"]


def page(url):
    return json.loads(subprocess.check_output(["gh", "api", url], timeout=90))


def slim(item, keep):
    out = {k: item.get(k) for k in keep}
    if out.get("user"):
        out["user"] = dict(login=out["user"].get("login"), id=out["user"].get("id"), type=out["user"].get("type"))
    if "labels" in out:
        out["labels"] = [x["name"] for x in out["labels"] or []]
    return out


def fetch(root, kind, url, keep, pages):
    dest = root / f"{kind}.jsonl"
    done = sum(1 for _ in dest.open()) // 100 if dest.exists() else 0
    with dest.open("a") as f:
        for n in range(done + 1, pages + 1):
            try:
                rows = page(f"{url}&per_page=100&page={n}")
            except subprocess.CalledProcessError as e:
                # GitHub stops deep pagination; keep what was already stored.
                print(kind, n, "stopped", e.returncode, flush=True)
                break
            for row in rows:
                f.write(json.dumps(slim(row, keep), ensure_ascii=False) + "\n")
            print(kind, n, len(rows), flush=True)
            if len(rows) < 100:
                break


if __name__ == "__main__":
    p = argparse.ArgumentParser()
    p.add_argument("root", type=Path)
    p.add_argument("--issue-pages", type=int, default=100)
    p.add_argument("--comment-pages", type=int, default=400)
    a = p.parse_args()
    a.root.mkdir(parents=True, exist_ok=True)
    fetch(a.root, "issues", f"{REPO}/issues?state=all&sort=created&direction=desc", KEEP_ISSUE, a.issue_pages)
    fetch(a.root, "comments", f"{REPO}/issues/comments?sort=created&direction=desc", KEEP_COMMENT, a.comment_pages)
