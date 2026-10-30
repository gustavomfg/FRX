# FRX AI v4 — evidence-driven events and next checks

Research only; preserve the application and v1/v2/v3. No games, Steam writes, fixes, commits, tags or push. Four CPU threads, <=6 GiB process RAM, <=60 minutes aggregate timed model work, <=600 MiB new model downloads and <=2 GiB new cache. Reuse the v2 retrieval encoder and existing CPU training environment. Stop a family after informative negative results rather than grid-searching indefinitely.

## New questions, registered before new model fitting

1. Audit the annotation unit before fitting: separate action trigger, literal argument, evidential status cue, observation, outcome association and joint procedure. Old labels remain unchanged. A new audit schema records ambiguities and ontology changes, without treating model disagreement as proof of label error.
2. Train an event-span proposal model rather than another comment-outcome classifier. Compare old broad action spans, new audited event-trigger spans, and synthetic contrast augmentation. Separate detection recall from status/direction/association precision. Preserve lexical and v3 learned baselines.
3. Verify candidate assertions against literal bounded evidence. Compare ungated proposals with a constrained verifier: suggestion, performed, negated attempt and pre-existing state are distinct. An argument or outcome not literally supported stays unknown. A result after several actions belongs to the combination, not each action individually.
4. Evaluate next-check selection on real historical report/request pairs when a faithful preceding report can be recovered. Compare missing-field heuristics, lexical/semantic case retrieval and a small trained ranking policy. Do not fabricate real cases from synthetic diagnostic assumptions.
5. Reuse contextual memory, adding game build, technology, configuration and symptom to the research context where available. Preserve old memory format through an adapter; never import a community success as a local result. Deduplicate both comments and threads when aggregating evidence.

## Evaluation rules

All old test sets are now known audit/development material. Reserve new games/discussions before inspecting their final text. No training or synthetic variants derived from final texts. Freeze code, model/configuration hashes and criteria before final annotation and prediction. Agent labels remain provisional, without independent human review.

Primary event metrics: exact/overlap trigger precision/recall; status and direction on matched events; supported action/result relation precision; false performed actions; false successes; joint-procedure attribution; hallucinated identities; coverage; literal quote integrity. A component is promising only with a material improvement over an appropriate baseline and nonzero useful coverage. A >=90% claim needs >=30 accepted claims in >=4 final games, confidence intervals and no observed severe inversions; these floors are not a population guarantee.

Next-check metrics: pertinent check@1/@3, missing-evidence recognition, unsupported diagnoses and deduplication/context consistency. Separate real historical requests from synthetic stress fixtures. No numeric real-game cause probabilities without a justified fitted model. A check is a request for evidence, never an executable installation command.

Counterfactuals are training or regression controls, clearly synthetic and separate from independent real evaluation. Audit findings and labels are versioned. Any method changed after final inspection requires a new final test to support independent claims. No output authorizes the Fix Engine.

Before any final-body inspection: reserve one paragraph per comment from new AppIDs 1627720, 2358720, 1182900, 1693980 and 1282100, with ten eventful and four hash-control comments where available. The first game has only one eligible comment; Remnant II was added for adequate game coverage before reading any final text. This is an event-enriched stress test, not representative prevalence. Full source context stays available; bounded paragraph assertions do not claim whole-thread resolution.

Development amendment before final access: legacy broad targets are a diagnostic data-unit control. Token-trigger labels are much rarer; test training-only class-frequency weighting, and contrast loss ablation. Compare raw subword decoding with word-boundary-constrained decoding. Training history uses exploratory any-overlap for early checkpoint selection; all definitive model comparisons are rescored with character IoU >=0.5 so a large predicted phrase cannot earn a correct event by touching one word. Do not report loose-overlap scores as final extraction quality.

Further development-only decoding control: project a proposed coarse span to literal operation heads inside it, comparing against the same detector without projection. This may improve boundaries without changing model weights. Supplied-gold-trigger status controls compare v3 and the verifier independently of detection; they must never be reported as end-to-end extraction accuracy. The resulting literal verifier is deterministic, not a newly trained status model.
