# FRX AI — relation and retrieval laboratory v2

Local research, separate from FRX, Steam and the Fix Engine. The learned components are a fine-tuned MiniLM relation encoder and a contrastively trained evidence retriever. Action/result proposals, source gates and graph serialization are deterministic. This is not a foundation model, causal inference system or approved fix recommender.

The final measurements and limitations are in [the scientific report](../../../docs/FRX-AI-SCIENTIFIC-2026-10-29.md). [Protocol](PROTOCOL.md), [literature and mathematics](LITERATURE.md), and [small reproducibility snapshots](snapshot/) describe the experiment. Model binaries, original comments and checkpoints remain outside Git.

## Run the prepared prototype

On the research machine:

```bash
AI_WORK="${XDG_CACHE_HOME:-$HOME/.cache}/frx-ai-rnd-v2-20261029"
"$AI_WORK/venv/bin/python" research/frx_ai/v2/prototype.py \
  --model "$AI_WORK/runtime-goal" --retrieval "$AI_WORK/runtime-retrieval" \
  < "$AI_WORK/demo-input-v2.jsonl"
```

Input is JSONL with `context.app_id`, a symptom and up to eight attributed GitHub evidence objects containing `app_id`, `url` and original `text`. Optional `action_span` offsets ask about a specific passage. Offsets count Unicode code points, not UTF-8 bytes. The original text hash accompanies each quoted span. Missing identities, hardware used by the game and individual causes stay unknown.

The goal model can return different candidate effects for different observation spans. It does not assign the effect of a joint procedure to each member. Scores and quotes remain experimental: `accepted_automatically` and `execution_authorized` are false. No output is an executable recipe. A reported result is not a local benchmark. Explicitly marked synthetic evidence is rejected by the real-evidence interface.

Native inference uses ONNX Runtime and the pinned tokenizer, without PyTorch, Ollama, a remote inference API or implicit downloads. Optional semantic ranking means topical relevance, not confirmation that a solution works. The runtime validates caller attribution and known context mismatches; it cannot authenticate a remote comment without a verified upstream source object.

## Reproduce on another machine

Tested with Python 3.14 on Linux. Use an isolated environment:

```bash
python3 -m venv /path/to/frx-ai-venv
/path/to/frx-ai-venv/bin/pip install -r research/frx_ai/v2/requirements.txt
/path/to/frx-ai-venv/bin/python research/frx_ai/v2/setup.py /path/to/new-run/pretrained
/path/to/frx-ai-venv/bin/python research/frx_ai/v2/data.py \
  research/frx_ai/v2/snapshot/annotations.json /path/to/new-run --fetch
/path/to/frx-ai-venv/bin/python research/frx_ai/v2/data.py \
  research/frx_ai/v2/snapshot/goal-annotations.json /path/to/new-run --fetch --goals
/path/to/frx-ai-venv/bin/python research/frx_ai/v2/replay.py /path/to/new-run E07-full-contrast005
```

The explicit fetch uses `gh api`, with a request bound, original body hashes and preserved CRLF offsets. Edited/deleted public comments require the original research cache; changed text must not silently reuse old annotations. Public availability does not imply permission to redistribute comments under the model's license. All evaluation labels are provisional annotations by the research agent, with no independent human review. The blind review queue is in `snapshot/human-review-queue.json`.

The pretrained weights are Apache-2.0, pinned to an upstream revision and checked against SHA-256/LFS metadata. Training modifies weights; the source encoder is not merely executed frozen. Fine-tuned models are not published in FRX installers. Redistribution still requires appropriate notices and dataset/license review.

## Experimental controls

- E01–E04: 87 real span annotations; E05–E10: 113; E11–E17: 167. Synthetic and weak labels are explicit training-only additions.
- E18 is a different task: action/observation relations by goal, with separate effects rather than an overall mixed label. Compare its scores separately.
- `baselines.py` evaluates lexical representations; `active.py` simulates acquisition with two seeds; `retrieval_train.py` trains query/document representations.
- `export.py` and `retrieval_export.py` compare float/int8 on development before selecting native artifacts. `freeze.py` fits calibration on a separate game and seals model/configuration/input hashes.
- The new final games are never loaded by training. `final_evaluate.py` refuses changed frozen inputs and preserves the first result. Further training requires a new protocol/run and a new independent final test.

The old final test is known development material in this round. Reproducing its scores does not create a new independent test. The final action sample is conditional on deterministic proposals; global entity extraction recall was not measured. English technical comments are the tested language. Free-form Portuguese, thread-level coreference, hardware compatibility learning and cause identification remain separate work.

Run research contracts with the research environment:

```bash
/path/to/frx-ai-venv/bin/python -m unittest discover -s research/frx_ai/v2 -p 'test_*.py' -v
```

No application tests, UI changes, game launches, Steam writes, commits or pushes are part of this laboratory.
