"""Read-only adapters to prior laboratories; own caches never write into them."""
import sys,json,hashlib,importlib.util
from pathlib import Path
HERE=Path(__file__).resolve().parent
V5=HERE.parent/'v5'
if str(V5) not in sys.path:sys.path.append(str(V5))
from common import read,write,jsonl,digest,evidence,prior4
from compact import CompactSyntax
from syntax import Syntax
from events import extract as prior_events,children,family,governing,canonical
from relations import observations as prior_observations,reconstruct as prior_reconstruct
from lab import retriever
from incidents import Incidents,action_key,context_match
def sha(p):return hashlib.sha256(Path(p).read_bytes()).hexdigest()
def immutable(root):
 if (root/'frozen.json').exists():raise ValueError('frozen v6; no further fitting/selection')
