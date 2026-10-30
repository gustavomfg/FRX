"""Replay one neural experiment in a NEW hydrated research directory."""
import argparse
import json
import pathlib
import subprocess
import sys
from data import model_input_digest


def main():
    p=argparse.ArgumentParser();p.add_argument('root',type=pathlib.Path);p.add_argument('experiment')
    a=p.parse_args()
    if (a.root/'frozen.json').exists():raise ValueError('do not train on the sealed original run')
    configs=json.loads((pathlib.Path(__file__).parent/'snapshot/experiment-configs.json').read_text())
    config=configs[a.experiment];file=a.root/(a.experiment+'.config.json');file.write_text(json.dumps(config))
    manifests=json.loads((pathlib.Path(__file__).parent/'snapshot/model-input-manifest.json').read_text())
    train_file=config.get('train_file','train-events.jsonl')
    if train_file in manifests:
        rows=[json.loads(x) for x in (a.root/train_file).read_text().splitlines()]
        if model_input_digest(rows)!=manifests[train_file]:raise ValueError('reconstructed model inputs differ')
    script='pair_train.py' if config.get('pair') else 'train.py'
    subprocess.run([sys.executable,str(pathlib.Path(__file__).parent/script),str(a.root),str(file)],check=True)


if __name__=='__main__':main()
