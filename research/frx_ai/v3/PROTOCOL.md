# FRX AI v3 — pre-experiment protocol, 2026-10-30

Scope: local research only. Preserve v1/v2 and the entire application. No Steam access, game launches, fixes, commits or pushes.

Budget: four CPU threads; <=45 minutes aggregate timed fitting/evaluation, <=6 GiB process RAM, <=1 GiB new downloads and <=2 GiB new cache. Reuse the v2 encoders and environment. Stop a family when it provides no useful new information. Target 12–24 meaningful configurations across distinct questions, not a quota.

## Registered hypotheses

1. **Procedure units:** comment-level outcome is the wrong unit. Classify bounded authored clauses as procedure, suggestion, observation or other; assess this independently of outcome attribution. Compare TF-IDF, frozen original MiniLM, frozen FRX retrieval representations, and context-conditioned representations. These are trained heads; segmentation remains deterministic. No classification establishes a causal relation.
2. **Retrieval:** a small trained query/document ranker combining lexical, semantic and bounded document signals can beat either signal alone. Compare TF-IDF/BM25, frozen MiniLM, preserved FRX retriever, reciprocal rank fusion and learned pairwise ranking. Topic relevance is not a successful fix. Report P@3, recall@5, NDCG@5 and MRR; include unanswered queries separately.
3. **Investigation:** typed evidence and three-valued prerequisite matching can prevent context leakage and unsupported diagnoses. Build read-only investigation graphs from literal logs, documented requirements and caller-attributed attempts. Compare simple retrieval with evidence-constrained candidate selection on historical cases and explicit stress fixtures.
4. **Learning:** estimate observation likelihoods from simulated incidents, then compare static, random and expected-information-gain inspection policies. Evaluate unseen simulated contexts and shifted sensor noise. Numerical probabilities belong only to this learned simulation, never uncalibrated real-game hypotheses.

## Splits and criteria

All v1/v2 final material is now known development material. Training and selection may use it only with that label. A new final reservation uses six AppIDs absent from v2: 1196590, 553850, 1462040, 1222140, 2246340 and 1771300. Reserve comment identities algorithmically before inspecting their text. Fetching/caching is not annotation. No new final labels or bodies are used for tuning; freeze code, configurations and artifacts before final annotation and the first evaluation.

Real labels remain provisional agent annotations, with no independent human reviewer. Synthetic cases, injected context conflicts, and simulator results are separate from public reports. Public text remains outside Git. Preserve URLs, title/AppID attribution, original hashes and Unicode offsets; changed sources cannot silently reuse annotations.

Procedure classification: macro F1, per-class precision/recall, confusion, coverage/precision of accepted spans, false tested claims, and exact event association only if actually annotated. Retrieval: paired comparisons and game-cluster bootstrap; useful improvement requires a gain without disguising lost metrics. Diagnostic constraints: unsupported assertions, wrong-context evidence, source coverage and next-check relevance, with real and synthetic scores separate. Simulations: diagnostic accuracy, mean inspections/cost, Brier score and failure under shift.

An accepted extraction claim would require >=90% precision, >=20% coverage, >=30 accepted examples across >=4 final games, and no observed severe suggestion/test or polarity inversions. Otherwise expose unverified candidates only. No prototype output authorizes execution regardless of metrics. Do not call simulated diagnostic accuracy real-game validation.

## Reproducibility

Implementation clarification before any new final text/label access: the first procedure experiment uses the existing annotated action/result spans as supplied units and trains separate speech-act and observation heads. It does not measure complete clause extraction or temporal relation recall. New final modality units use one deterministic action proposal per comment; new final observation units are selected without gold. Report supplied-span versus automatic-proposal results separately. Primary retrieval selection uses v2 train+dev games for fitting and the now-known v2 final games for development; these are not a fresh final evaluation.

Store initial file hashes, source/cache manifests, labels, model artifacts and timing logs in an external v3 research directory. Git contains only code, small label offsets/manifests, aggregate results and documentation. A final set used to revise a method is no longer an independent final test.
