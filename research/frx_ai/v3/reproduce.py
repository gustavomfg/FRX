"""Rebuild the v3 experiment in a NEW external cache, never the sealed run."""
import argparse
from pathlib import Path
import subprocess
import sys

def main():
 parser=argparse.ArgumentParser();parser.add_argument('root',type=Path);parser.add_argument('old',type=Path);a=parser.parse_args()
 if (a.root/'frozen.json').exists():raise ValueError('do not overwrite the sealed original research')
 a.root.mkdir(exist_ok=True,parents=True);here=Path(__file__).resolve().parent
 for script,args in [('events.py',[a.root,a.old]),('ranking.py',[a.root,a.old]),('simulation.py',[a.root]),('structure.py',[a.root])]:
  subprocess.run([sys.executable,str(here/script),*[str(x) for x in args]],check=True)

if __name__=='__main__':main()
