# FRX AI — local research prototype

The subsequent [v2 scientific laboratory](v2/README.md) preserves this experiment and adds actual encoder fine-tuning, relation and retrieval experiments, a new held-out game test, and [a separate scientific report](../../docs/FRX-AI-SCIENTIFIC-2026-10-29.md). Its promising result concerns evidence ranking; automatic action/result interpretation still fails the acceptance target. The results below remain the historical v1 record.

**Research only. The automatic interpretation quality target was NOT met.** This module is not connected to the application, Fix Engine or Complete Experience Packs. It never creates executable plans. A high model score cannot authorize an action or confirm a result.

The prototype combines local ONNX MiniLM evidence retrieval, DeBERTa-xsmall natural-language inference, and a specialized logistic head trained on 100 manually labeled public-comment/action pairs. The upstream encoder/NLI weights are frozen. Action-state parsing is deterministic and remains unreliable on complex procedures. This is a hybrid learned research model, not a newly trained foundation model.

## What is usable

- JSONL interface accepting explicitly attributed GitHub evidence and game context.
- Bounded local semantic retrieval, original quotes, hashes and source metadata.
- Experimental action/result candidates, explicitly unverified.
- Exclusion of another game's evidence, known hardware/driver/Proton mismatches and explicitly marked synthetic input.
- No inference API, Ollama, Steam writes, game launches or shell execution.

It does **not** reliably decide which fix worked, generate causes, infer missing library identities or establish that a configuration is safe. `reported_result` stays `unknown`, `accepted_automatically` and `execution_authorized` stay false. The actual erroneous candidate scores remain visible for research, not as recommendations.

## Local environment already prepared

The datasets, public source cache, research-only checkpoints and model binaries are outside Git:

```bash
AI_WORK="${XDG_CACHE_HOME:-$HOME/.cache}/frx-ai-rnd-20261029"
"$AI_WORK/venv/bin/python" research/frx_ai/prototype.py --models "$AI_WORK" < "$AI_WORK/demo-input.jsonl"
"$AI_WORK/venv/bin/python" -m unittest discover -s research/frx_ai -p 'test_*.py' -v
```

`demo-input.jsonl` uses a real Crimson Desert report of distorted geometry after `descriptor_heap`. It is a diagnostic example, not part of the final benchmark. The candidate outcome is incorrectly classified as `not-resolved` rather than `worsened`; the source quote is preserved and no automatic result is accepted. This failure is deliberately documented.

## Research setup on another machine

```bash
python3 -m venv /path/to/frx-ai-venv
/path/to/frx-ai-venv/bin/pip install -r research/frx_ai/requirements.txt
/path/to/frx-ai-venv/bin/python research/frx_ai/setup_models.py --destination /path/to/frx-ai-models
/path/to/frx-ai-venv/bin/python research/frx_ai/prototype.py --models /path/to/frx-ai-models
```

Setup is explicit and downloads about 114 MiB, checking pinned revisions, sizes and SHA-256. Inference itself never downloads. No model is included in FRX installers. The runtime uses four CPU threads and 256-token model inputs; long comments can lose relevant context. Model licenses/origins are recorded in `snapshot/model-manifests.json`. Both model cards declare Apache-2.0. Production distribution would also need the proper notices and native ONNX/tokenizer packaging.

Example request (illustrative schema; use actual source text for investigation):

```json
{"context":{"app_id":"3321460","game":"Crimson Desert"},"symptom":"crashes after changing launch options","evidence":[{"app_id":"3321460","url":"https://github.com/ValveSoftware/Proton/issues/9595#issuecomment-6001922940","target":"VKD3D_CONFIG","text":"<paste original evidence here>","context":{"gpu":"NVIDIA RTX 3060"}}]}
```

The target `<procedure>` denotes a procedural action whose exact identity is not established. The prototype returns a null target and a tentative state instead of inventing a DLL. Missing source context remains missing. Known context matching is conservative string matching, not a full hardware compatibility engine.

## Experiments and reproducibility

`collect.py` fetches at most 100 comments from each of 12 public Proton issue threads, caching responses and respecting a bounded request count. The original FRX provider cache supplies the previously investigated Crimson Desert/Batman development material. No user Steam state is used. Do not distribute the comment corpus under the model's license: public comments do not automatically inherit the repository's software license.

The annotated corpus is local: `train.jsonl` (100), `dev.jsonl` (50), `final.jsonl` (80). Manual annotations were made by this research agent, from complete comments, with no second annotator. `0` means no attributable outcome, not a failed fix. The generic procedure target and complex bundles make some labels debatable. The files contain the original text, URL, game/AppID, target, timestamp, label and annotation provenance. Hardware/Proton are retained in source text, but not exhaustively annotated.

Thread/game splits and file digests are in `snapshot/dataset-manifest.json`. Raw datasets and checkpoints remain outside Git as requested. Reproducing the measurements on this machine uses those preserved files; a new clone alone cannot reconstruct the manual labels. `data.py` regenerates the sampling, not the labels. Public sources can change, so verify digests before reusing a split.

Run research scripts with the research venv, setting `OPENBLAS_NUM_THREADS=4`:

1. `experiment.py AI_WORK`: whole-comment TF-IDF, local encoder, local word/character model and consensus.
2. `iterate.py AI_WORK`: training-only synthetic augmentation and separate state head.
3. `factor_experiment.py AI_WORK`: sentence-focused representations.
4. `nli_experiment.py AI_WORK`, then `reportability.py AI_WORK`: NLI and the narrower learned report detector.
5. Preserve a model/threshold freeze before reading final labels. `final_evaluation.py AI_WORK` checks recorded hashes and refuses to overwrite its first final result.

The initial NLI learned-head *development comparator* had an in-place normalization bug; its development metrics are invalidated. The selected binary head used separately saved raw features and was unaffected. The code now copies the normalization view. No weights or thresholds were changed after the final test. Final metrics are preserved in `snapshot/final-results.json`.

The retrieval audit was labeled after the outcome model freeze and is **exploratory**, not another independent final test or evidence of meeting the 90% goal. It compares plain lexical overlap with frozen MiniLM on 23 game/symptom queries over 75 unique comments. Neither method is BM25 or a trained reranker.

## Results

- Learned report detector: dev 15/18 (83.3%); unseen final 14/29 (48.3%), coverage 36.3%.
- NLI outcome + action state: final 3/15 (20.0%), coverage 18.8%, four false success predictions. Zero positive↔negative sign inversions does not make those false successes acceptable.
- Existing Rust rules: final 3/4 correct action/outcome assertions; tiny coverage (5%), insufficient evidence of 90% precision.
- Exploratory retrieval precision@3: lexical 40.6%, MiniLM 44.9% — a small signal, not a robust gain.
- Local CPU warm latency: median 112 ms, p95 139 ms (20 repeated simple requests), peak RSS about 830 MiB, cold startup 1.05 s. Model/tokenizer files about 114 MiB.
- Eight protocol/security contract tests pass; the real local ONNX demo runs. These tests do not establish model accuracy.

The research budget was four bounded cycles/families of methods, not an unbounded search. The useful result is an executable, inspectable experimental pipeline and evidence about why it fails. No trained component from this round is approved for automatic recommendations. The next substantive need is independently reviewed, span-level action/goal/outcome labels across more games, especially regressions and multi-step procedures; bigger language models alone did not address that bottleneck here.
