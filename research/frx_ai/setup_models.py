"""Explicit RESEARCH setup, bounded and verified downloads. Never invoked by inference."""
import argparse,json,pathlib,hashlib,urllib.request,shutil
MAX_TOTAL=140_000_000

def main():
 p=argparse.ArgumentParser();p.add_argument('--destination',type=pathlib.Path,required=True);args=p.parse_args();here=pathlib.Path(__file__).parent;manifests=json.loads((here/'snapshot/model-manifests.json').read_text());total=0
 for name,manifest in manifests.items():
  directory=args.destination/name;directory.mkdir(parents=True,exist_ok=True)
  for item in manifest['files']:
   total+=item['bytes']
   if total>MAX_TOTAL:raise ValueError('download budget exceeded')
   file=directory/pathlib.PurePosixPath(item['file']).name
   if file.exists() and hashlib.sha256(file.read_bytes()).hexdigest()==item['sha256']:continue
   url=manifest['repository']+'/resolve/'+manifest['revision']+'/'+item['file']
   with urllib.request.urlopen(url,timeout=60) as response:data=response.read(item['bytes']+1)
   if len(data)!=item['bytes'] or hashlib.sha256(data).hexdigest()!=item['sha256']:raise ValueError('artifact integrity mismatch')
   # Data only; model graphs are loaded later by the bounded local runtime.
   file.write_bytes(data)
  (directory/'manifest.json').write_text(json.dumps(manifest,indent=2))
 (args.destination/'experiments').mkdir(exist_ok=True);shutil.copyfile(here/'snapshot/report-head.json',args.destination/'experiments/report-head.json')
 print('Research models verified:',total,'bytes')
if __name__=='__main__':main()
