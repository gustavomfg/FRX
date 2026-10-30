"""Explicit pinned research-only model setup; never called by inference."""
import argparse
import hashlib
import json
import pathlib
import urllib.request


def main(destination):
    manifest=json.loads((pathlib.Path(__file__).parent/'snapshot/pretrained-manifest.json').read_text())
    destination.mkdir(parents=True,exist_ok=True)
    if sum(x['size'] for x in manifest['files'])>100_000_000:raise ValueError('model budget exceeded')
    for item in manifest['files']:
        file=destination/item['file']
        if file.parent!=destination:raise ValueError('unexpected model path')
        if file.exists() and hashlib.sha256(file.read_bytes()).hexdigest()==item['sha256']:continue
        url=f"https://huggingface.co/{manifest['repository']}/resolve/{manifest['revision']}/{item['file']}"
        with urllib.request.urlopen(url,timeout=90) as response:raw=response.read(item['size']+1)
        if len(raw)!=item['size'] or hashlib.sha256(raw).hexdigest()!=item['sha256']:raise ValueError('model integrity mismatch')
        file.write_bytes(raw)
    print('Pinned Apache-2.0 pretrained research assets verified. No fine-tuned model was downloaded.')


if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('destination',type=pathlib.Path);main(p.parse_args().destination)
