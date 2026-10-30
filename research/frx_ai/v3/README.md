# FRX AI v3 — evidence-based investigation laboratory

This round compares procedure understanding, retrieval, constrained investigation and learned probabilistic diagnosis. It preserves v1/v2 and the application. **The new language heads failed; no automated outcome interpretation or real-game root-cause model is approved.**

The useful deliverable combines the previously trained local FRX retriever with literal log observations, explicit unknowns, context constraints, a small evidence graph and optional persistent attempt history. These latter components are deterministic engineering, not a trained causal model. New learned diagnostic structures are evaluated only in declared simulations.

[Scientific report](../../../docs/FRX-AI-MULTIDISCIPLINARY-2026-10-30.md), [initial protocol](PROTOCOL.md), [separate structure protocol](STRUCTURE-PROTOCOL.md), [literature](LITERATURE.md), and [reproducibility snapshots](snapshot/).

## Run the prepared local prototype

From the repository root, on the research machine:

```bash
FRX_AI_RUN="${XDG_CACHE_HOME:-$HOME/.cache}/frx-ai-rnd-v3-20261030"
FRX_AI_PREVIOUS="${XDG_CACHE_HOME:-$HOME/.cache}/frx-ai-rnd-v2-20261029"
FRX_AI_PY="$FRX_AI_PREVIOUS/venv/bin/python"

"$FRX_AI_PY" research/frx_ai/v3/case_review.py \
  "$FRX_AI_RUN" "$FRX_AI_PREVIOUS" < "$FRX_AI_RUN/demo-real.jsonl"
```

The prepared example is a public Last of Us Part I report containing an explicit `AVRT.dll` loader error. It is **not local telemetry**, a validated download recommendation or proof of the incident's cause. The output preserves its source and literal observation and proposes identifying the correct component/version. It does not install anything.

Input: JSONL with `context.app_id`, optional `gpu`/`driver`/`proton`, `symptom`, bounded `log`, attributed `evidence`, explicit `attempts`, documented `facts` and optional structured `candidate_actions`. Input text is data, never executable instructions. Outputs include ranked sources, supplied-log observations, investigable hypotheses, requirements, unknowns and next checks. Hardware support does not imply the game implements a feature. No technology is inferred from a DLL filename alone.

Optional caller-reported attempt memory:

```bash
"$FRX_AI_PY" research/frx_ai/v3/case_review.py \
  "$FRX_AI_RUN" "$FRX_AI_PREVIOUS" \
  --memory "$FRX_AI_RUN/user-attempt-memory.json"
```

Only fully matched AppID/GPU/driver/Proton contexts reuse attempts. Memory survives process restart, deduplicates IDs and refuses conflicting replacements. It stores caller-reported outcomes, **never model predictions or proven causal effects**. The CLI-selected file is local research state; no retrieved content chooses its path. Logs and public source bodies are not stored in this memory.

Add `--experimental-events` only to inspect the failed trained heads' tentative speech-act/outcome candidates. They remain unaccepted, unverified and disconnected from execution. The default omits these unreliable predictions. The frozen original challenger remains reproducible through `prototype.py`; its window ranking did not improve over the preserved v2 baseline on the new final set.

`case_review.py` and memory were assembled **after** evaluating the frozen challenger. They preserve the preregistered v2 baseline; the complete combined workflow has not received an independent diagnostic-accuracy test.

## Experiments and reproduction

24 learned configurations: 12 event-head fits, six pairwise rerankers, three likelihood/context models and three dependency-structure models. Lexical/semantic baselines and engineering contracts are additional comparisons, not additional trained configurations. v3 reuses the v2 neural weights; new learning is in linear heads, rankers, empirical likelihoods and dependency structure. No new LLM or encoder fine-tuning is claimed.

Reproduce fits in a **new external directory**, preserving the sealed original:

```bash
"$FRX_AI_PY" research/frx_ai/v3/reproduce.py \
  /path/to/new-experiment "$FRX_AI_PREVIOUS"

"$FRX_AI_PY" research/frx_ai/v3/seal.py "$FRX_AI_RUN" --verify
"$FRX_AI_PY" -m unittest discover -s research/frx_ai/v3 -p 'test_*.py' -v
```

On another machine, install `requirements.txt` in an isolated Python environment. First reproduce the preserved v2 data and model artifacts using the [v2 laboratory](../v2/README.md). `reproduce.py` expects its `train-events.jsonl`, `dev-events.jsonl`, `final-events.jsonl`, retrieval datasets and the `runtime-retrieval` / `runtime-frozen-control` directories. Original and trained encoders must use the same tokenization/quantization controls. No model is downloaded by inference.

The v3 snapshot publishes the final annotations with URLs, source hashes and offsets, **without original comments**. Rebuild original source units using an existing external `bodies/` cache, or explicit bounded GitHub API requests:

```bash
"$FRX_AI_PY" research/frx_ai/v3/hydrate_final.py /path/to/new-experiment --fetch
"$FRX_AI_PY" research/frx_ai/v3/make_demo.py "$FRX_AI_RUN"
```

Explicit fetching requires `gh` and permitted public API access; changed source bodies fail their hash check. `make_demo.py --fetch` can fetch its one source if missing. All labels are provisional research-agent annotations, with no independent human review. Public availability is not permission to redistribute the corpus under a model license. Pretrained MiniLM provenance/license remains in the v2 manifest. Learned model artifacts and public bodies remain outside Git.

The old final sets are known development data. Do not treat rerunning a published test as new independent validation. New experiments that use these final results need fresh held-out games.

## Code map

| Files | Purpose |
| --- | --- |
| `events.py`, `prepare.py` | Train separate speech-act/observation heads and construct blind source proposals. |
| `ranking.py`, `common.py` | Semantic/lexical/window comparisons, pairwise ranking and context-independent helpers. |
| `investigation.py`, `memory.py` | Three-valued constraints, literal supplied-log signals, typed evidence and contextual attempt retention. |
| `simulation.py`, `structure.py` | Explicit synthetic likelihood learning, information gain, duplicate grouping and tree structure. |
| `prototype.py`, `case_review.py` | Frozen challenger and conservative post-evaluation assembly. |
| `seal.py`, `final_evaluate.py` | First-evaluation protection and held-out measurements. |
| `diagnostic_study.py`, `benchmark_review.py` | Historical log demonstrations and bounded CPU measurement. |
| `hydrate_final.py`, `make_demo.py`, `reproduce.py`, `snapshot.py` | Reproduction and small auditable metadata. |
| `test_research.py`, `test_memory.py` | Research contracts, isolation, persistence and unknowns. |

The tested text is predominantly English technical prose. Environment matching uses conservative string identities, not a learned compatibility model; aliases or differently written names can exclude compatible reports. Free-form Portuguese, broad event coreference, arbitrary log formats, automatic source authentication, validated diagnosis probabilities and reliable procedure/outcome attribution remain open work. The prototype does not retrieve attachments, decode remote payloads, run shell commands, execute fixes or initiate games.
