"""Public GitHub collection. Final source bodies stay outside the repository.

Only metadata is printed. A query match is not game attribution: require an
explicit AppID in the chosen issue title, and keep the title as provenance.
"""
import argparse
import hashlib
import json
import pathlib
import re
import subprocess
import time
from urllib.parse import quote

FINAL_GAMES = {
    "782330": "DOOM Eternal", "814380": "Sekiro",
    "1716740": "Starfield", "1172380": "Jedi Fallen Order",
    "1252330": "DEATHLOOP", "275850": "No Man's Sky",
    "2515020": "FINAL FANTASY XVI", "1888930": "The Last of Us",
}
TRAIN_GAMES = {"582160":"Assassin's Creed Origins", "374320":"DARK SOULS III",
               "397540":"Borderlands 3", "1446780":"MONSTER HUNTER RISE"}


def get(path, cache):
    f = cache / (hashlib.sha256(path.encode()).hexdigest() + ".json")
    if not f.exists():
        raw = subprocess.check_output(["gh", "api", path], timeout=60)
        if len(raw) > 6_000_000:
            raise ValueError("response size limit")
        f.write_bytes(raw)
        time.sleep(0.4)
    return json.loads(f.read_text())


def main(root, training=False):
    cache = root / "public"
    cache.mkdir(parents=True, exist_ok=True)
    corpus = []
    for app, game in (TRAIN_GAMES if training else FINAL_GAMES).items():
        search = get("search/issues?q=" + quote(
            f'repo:ValveSoftware/Proton in:title "{app}"') +
            "&sort=comments&order=desc&per_page=5", cache)
        items = [x for x in search.get("items", []) if
                 re.search(r"\(" + app + r"\)", x["title"])]
        if not items:
            print("no verified thread", app, flush=True)
            continue
        issue = items[0]
        pages = sorted({1, max(1, (issue["comments"] + 99) // 100)})
        comments = []
        for page in pages:
            comments.extend(get(f'repos/ValveSoftware/Proton/issues/{issue["number"]}/comments?per_page=100&page={page}', cache))
        corpus.append(dict(app_id=app, game=game, issue=issue,
                           comments=comments, role="additional-training" if training else "reserved-final"))
        print(app, issue["number"], len(comments), flush=True)
    (root / ("additional-training-sources.json" if training else "reserved-final-sources.json")).write_text(json.dumps(corpus))


if __name__ == "__main__":
    p = argparse.ArgumentParser()
    p.add_argument("root", type=pathlib.Path)
    p.add_argument("--training",action="store_true")
    args=p.parse_args();main(args.root,args.training)
