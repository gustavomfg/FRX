"""Small learned relation model. No execution, retrieval or application imports."""
import os
os.environ.setdefault("OMP_NUM_THREADS", "4")
os.environ.setdefault("TOKENIZERS_PARALLELISM", "false")
import re
import numpy as np
import torch
from torch import nn
from transformers import AutoModel, AutoTokenizer

from representation import OUTCOMES, DIRECTIONS, MODALITIES, ENV, representation


def tokenize(rows, tokenizer, mode="marked", length=192):
    pairs = [representation(r, mode) for r in rows]
    return tokenizer([x[0] for x in pairs], [x[1] for x in pairs],
                     truncation="only_second", max_length=length,
                     padding="max_length", return_tensors="pt")


class RelationModel(nn.Module):
    def __init__(self, pretrained, eager=False):
        super().__init__()
        self.encoder = AutoModel.from_pretrained(pretrained, local_files_only=True,
            **({'attn_implementation':'eager'} if eager else {}))
        h = self.encoder.config.hidden_size
        self.project = nn.Sequential(nn.Linear(h * 3, 192), nn.Tanh(), nn.Dropout(.1))
        self.heads = nn.ModuleList([nn.Linear(192, n) for n in [5, 10, 3, 2]])

    def forward(self, **batch):
        token_types = batch.get("token_type_ids", torch.zeros_like(batch["input_ids"]))
        h = self.encoder(**batch).last_hidden_state
        mask = batch["attention_mask"].float()
        # First sequence is the supplied action proposal, second is context.
        action = (token_types == 0).float() * mask
        a = (h * action[..., None]).sum(1) / action.sum(1).clamp(min=1)[:, None]
        c = (h * mask[..., None]).sum(1) / mask.sum(1).clamp(min=1)[:, None]
        z = self.project(torch.cat([h[:, 0], a, c], 1))
        return [head(z) for head in self.heads], nn.functional.normalize(z, dim=1)


def labels(rows):
    return torch.tensor([[OUTCOMES.index(r["outcome"]), DIRECTIONS.index(r["direction"]),
                          MODALITIES.index(r["modality"]), int(r["joint"])] for r in rows])


def supervised_contrast(z, y, temperature=.12):
    similarity = (z @ z.T) / temperature
    eye = torch.eye(len(z), dtype=torch.bool)
    positive = (y[:, None] == y[None, :]) & ~eye
    logits = similarity.masked_fill(eye, -1e4)
    logp = logits - torch.logsumexp(logits, dim=1, keepdim=True)
    valid = positive.sum(1) > 0
    if not valid.any():
        return z.sum() * 0
    return -(logp * positive).sum(1)[valid].div(positive.sum(1)[valid]).mean()


def predict(model, tokens, batch_size=16):
    model.eval()
    outputs = [[] for _ in range(4)]
    with torch.inference_mode():
        for i in range(0, len(tokens["input_ids"]), batch_size):
            logits, _ = model(**{k: v[i:i+batch_size] for k, v in tokens.items()})
            for j in range(4):
                outputs[j].append(logits[j].numpy())
    return [np.concatenate(x) for x in outputs]
