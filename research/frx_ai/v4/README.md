# FRX AI v4 — evidence-driven event research

Local laboratory only. The new trained component proposes **event triggers**, followed by a deterministic literal-evidence verifier. The preserved v2 retriever and v3 investigation/memory are reused. No UI, Fix Engine, Steam, game, installer, commit or push is changed.

**Result:** fewer false performed-action claims, with severe loss of coverage. On 57 reserved paragraphs from five unseen games, the selected pipeline accepted 5/33 performed events, with no false performed claims. Five accepted examples do **not** establish 90% precision. Full accepted status/direction tuples were only 8/16 correct. Action/result interpretation remains unvalidated and disabled by default. Do not connect this prototype to execution.

[Scientific report](REPORT.md), [protocol](PROTOCOL.md), [annotation contract](ANNOTATION.md), [literature](LITERATURE.md), [source-free snapshots](snapshot/).

## Run the prepared prototype

From the repository root on the research machine:

```bash
FRX_AI_V4="${XDG_CACHE_HOME:-$HOME/.cache}/frx-ai-rnd-v4-20261030"
FRX_AI_V2="${XDG_CACHE_HOME:-$HOME/.cache}/frx-ai-rnd-v2-20261029"
FRX_AI_PY="$FRX_AI_V2/venv/bin/python"

"$FRX_AI_PY" research/frx_ai/v4/run.py "$FRX_AI_V4" "$FRX_AI_V2" \
  < "$FRX_AI_V4/demos.jsonl"
```

Input/output are JSONL. Input includes `context.app_id`, optional `gpu`, `driver`, `proton`, `game_version`, `technology`, `configuration`, a bounded `symptom`, optional `log`, attributed `evidence`, documented `facts`, `attempts` and `candidate_actions`. See the prepared demos for six real-source/controlled-view cases. Public reports are never local telemetry. Hardware support or a DLL name does not prove game technology support.

Output preserves source URLs, hashes, Unicode-code-point offsets, proposed triggers, literal status cues, unknown identities, investigable log hypotheses and next checks. `automatic_interpretation_approved`, `execution_authorized` and confirmed causality remain false. Model scores mean proposal/topical relevance, not calibrated confidence or safe installation.

Add `--memory /path/to/external/research-memory` for caller-reported attempts. Fully known AppID/GPU/driver/Proton plus matching optional build, technology, configuration and symptom namespace prevent automatic reuse across contexts. Failed actions are marked previously failed in the same context. Community model predictions never enter local memory. Matching is conservative exact metadata, not a learned hardware-compatibility model.

`run.py` adds a **post-evaluation engineering guard** to the unchanged frozen `prototype.py`: explicitly supplied attempts/candidates with missing or different environment metadata are excluded as well. This fixes a gap in the old adapter, whose namespace protected stored history but not directly supplied build-mismatched attempts. The guard was tested as an isolation contract, not as a new independent accuracy experiment.

`--experimental-relations` exposes tentative local result links. Keep it off for ordinary use: even supplying correct event states yielded no recovered result links on the final cohort. Temporal/coreference handling is limited. Sources are untrusted data; no shell, package installation or binary execution occurs.

## Reproduce and audit

Native inference requires `requirements.txt` only: ONNX Runtime, tokenizers, NumPy. No PyTorch, Ollama, inference API or implicit download is required. Training additionally uses `requirements-train.txt` and the [prepared v2/v3 artifacts](../v2/README.md). Model weights and raw comments remain outside Git.

```bash
# Research contracts; no application tests or Steam writes.
"$FRX_AI_PY" -m unittest discover -s research/frx_ai/v4 -p 'test_*.py' -v
"$FRX_AI_PY" research/frx_ai/v4/seal.py "$FRX_AI_V4" --verify

# Rebuild exactly hashed annotations and source units from the external cache.
"$FRX_AI_PY" research/frx_ai/v4/hydrate.py /path/to/new-run \
  --body-cache "$FRX_AI_V4/bodies"
"$FRX_AI_PY" research/frx_ai/v4/make_demo.py /path/to/new-run \
  --body-cache "$FRX_AI_V4/bodies"

# Replay six fits in a NEW external directory; never use the sealed original.
"$FRX_AI_PY" research/frx_ai/v4/reproduce.py /path/to/new-training-run \
  "$FRX_AI_V2" "${XDG_CACHE_HOME:-$HOME/.cache}/frx-ai-rnd-v3-20261030" \
  --body-cache "$FRX_AI_V4/bodies"
```

Without the original body cache, explicitly add `--fetch` for bounded public GitHub API requests via `gh`. Edited/deleted comments fail hash checks; annotations are never silently reapplied to changed text. Public availability does not license corpus redistribution. Pretrained MiniLM provenance, Apache-2.0 license, revision and checksums remain in [v2's manifest](../v2/snapshot/pretrained-manifest.json).

The replay writes a clearly marked replication seal and can run the CLI against replicated weights. Published final labels are now known: rerunning them is replication, never a new independent test. Model/ONNX checksums can differ on another training stack; do not replace artifacts in the original sealed directory.

`benchmark.py`, `parity.py`, `retrieval_replay.py` and `review.py` reproduce resource, native parity, known retrieval and descriptive error checks. `snapshot/human-review-queue.json` deliberately omits predictions and provisional labels. All labels still need independent human review.
