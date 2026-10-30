"""Data-only classifier inference; no sklearn, Torch, downloads or execution."""
import json,math
from pathlib import Path
import numpy as np
class Classifier:
 def __init__(self,path,encoder=None):
  self.model=json.loads(Path(path).read_text());self.encoder=encoder;self.weights=np.array(self.model['coefficients'],dtype=float);self.bias=np.array(self.model['intercept'],dtype=float);self.index=self.model['classes'].index(1)
  if self.weights.shape!=(3,19+(384 if self.model.get('semantic') else 0)) or not np.isfinite(self.weights).all():raise ValueError('bounded verifier weights')
  if self.model.get('semantic') and encoder is None:raise ValueError('semantic control requires preserved encoder')
 def probabilities(self,row,b,pool):
  if not pool:return np.zeros((0,3))
  x=np.array([p['features'] for p in pool],dtype=float)
  if self.model.get('semantic'):
   from learning import pair_text
   x=np.column_stack([x,self.encoder.encode([pair_text(row,b,p) for p in pool])])
  z=x@self.weights.T+self.bias;z-=z.max(1,keepdims=True);p=np.exp(z);return p/p.sum(1,keepdims=True)
 def __call__(self,row,b,pool):return self.probabilities(row,b,pool)[:,self.index]
