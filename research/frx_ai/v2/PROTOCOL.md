# FRX AI relation research — protocol v2

Research only; no application integration, game launches, Steam writes, commits or model redistribution. Existing v1 results and all application work are preserved.

## Budget and hypotheses (before training)

Maximum 90 minutes of aggregate timed experiment execution, four CPU threads, target 6 GiB process RAM, 2 GiB new downloads and 4 GiB new cache/checkpoints. Stop a family when measured progress plateaus. Comparisons, seeds and failed runs are recorded; 100 is an upper allowance, not a quota. CPU PyTorch avoids installing multi-gigabyte CUDA dependencies.

1. **Representation:** explicit action spans, authored context, result spans and joint-action groups should outperform a comment-level sentiment label. A joint observation does not establish an individual cause.
2. **Learning:** a small encoder with supervised relation fine-tuning and hard contrastive examples should distinguish suggestions, failed attempts and regressions better than a frozen embedding head. Entity proposals and safety gates remain deterministic and are identified as such.
3. **Data:** keep ambiguous labels unknown; compare real-only, synthetic augmentation and weak labels separately. All annotations by this research agent are provisional, with no independent human agreement estimate.
4. **Selection:** temperature scaling and reject thresholds may improve selective precision; they cannot repair wrong associations. Conformal sets are experimental, with exchangeability explicitly doubtful across games.
5. **Retrieval:** compare lexical, semantic and hybrid retrieval with fixed queries and per-game filtering. Similarity is not evidence of a cause.

## Data separation and acceptance

All v1 final examples are now known and cannot serve as a final test again. New games/threads are reserved before annotation. Split by game/thread; deduplicate source URLs and normalized-text hashes across splits. No synthetic examples derived from new final texts. No access to final labels for model, threshold or prompt selection. Freeze model, source manifests and configuration hashes before final annotation/evaluation. A subsequent adjustment invalidates that final test for the adjusted model.

Development selection reports macro F1, action direction, association errors, false successes, risk/coverage and per-game metrics. A promising automatic extractor needs >=90% accepted exact relation/direction precision, >=20% coverage, >=30 accepted claims across >=4 final games, zero observed severe direction/sign inversions, and uncertainty intervals. These sample floors do not establish a population guarantee; report Wilson and game-cluster bootstrap intervals. Partial progress, including retrieval alone, is reported as such. No experimental result authorizes execution.

Annotations preserve original source, AppID verified from the issue title, original text, timestamps, action and result spans, modality, target, goal, hardware/software context and bundle membership. Missing identities remain unknown. Observation after a bundle is not causal attribution to each member. Git stores code, manifests and aggregate metrics, not raw public comments, model binaries or checkpoints; comment redistribution rights are not assumed.

## Scientific basis

- [DocRED](https://aclanthology.org/P19-1074/): document context and evidence across sentences; FRX does not reuse Wikipedia relations as technical labels.
- [PURE](https://aclanthology.org/2021.naacl-main.5/), [Matching the Blanks](https://aclanthology.org/P19-1279/): proposal/relation separation and explicit entity boundaries. This experiment is inspired by these ideas, not a reproduction of PURE.
- [ERICA](https://aclanthology.org/2021.acl-long.260/), [SetFit](https://arxiv.org/abs/2209.11055): contrastive representation objectives. Class-based supervised contrast is an ablation, not ERICA pretraining.
- [Snorkel](https://link.springer.com/article/10.1007/s00778-019-00552-1): weak signals must remain uncertain; a heuristic vote is not gold.
- [Calibration](https://proceedings.mlr.press/v70/guo17a.html): fit temperature on held-out calibration data, not final labels.
- [Conformal risk control](https://arxiv.org/abs/2208.02814): risk claims require assumptions; small clustered technical threads violate naive IID interpretations.
- [Confident Learning](https://research.google/pubs/confident-learning-estimating-uncertainty-in-dataset-labels/): disagreement can prioritize label review, but cannot automatically correct gold.

Independent annotation, broader source coverage, temporal drift and causal identifiability remain limitations even if classification scores improve.

## Amendment before final access: goal-conditioned edges

Development errors show that `mixed` still collapses different objectives. An
additional experiment will condition on an explicit action/result span pair,
with separate outcomes for each reported goal. For example, a joint procedure
can improve launching and leave a graphics problem unresolved. Its individual
causes remain unknown. This is a **different, narrower task** from classifying
a complete comment; its scores must be reported separately. Final pair spans
are chosen deterministically before annotation, from the same reserved games.
No final labels or bodies have been inspected for this amendment.
