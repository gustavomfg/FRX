# Selected literature and experimental decisions

| Work | Relevant idea | Use in this laboratory |
| --- | --- | --- |
| [DocRED](https://aclanthology.org/P19-1074/) | Relations and supporting evidence can cross sentence boundaries. | Preserve full source documents and literal evidence offsets; bounded windows remain a measured limitation. No DocRED labels are reused as gaming labels. |
| [PURE](https://aclanthology.org/2021.naacl-main.5/) / [Matching the Blanks](https://aclanthology.org/P19-1279/) | Distinguish entity proposals from relation classification; expose argument positions. | Action proposals plus learned relation heads, then action/result edge proposals. Proposal extraction is deterministic. This is not a reproduction of PURE's two trained encoders. |
| [ERICA](https://aclanthology.org/2021.acl-long.260/) / [Supervised Contrastive Learning](https://papers.nips.cc/paper/2020/hash/d89a66c7c80a29b1bdbab0f2a1a94af8-Abstract.html) | Contrast entity/relation representations, including negatives. | Class-based supervised contrastive loss and counterfactual action/result training examples. No ERICA entity-discrimination pretraining is claimed. |
| [SetFit](https://arxiv.org/abs/2209.11055) | Adapt compact sentence representations before classification. | Compare frozen features with actual encoder fine-tuning. Joint multitask training differs from SetFit's two-stage procedure. |
| [Aspect sentiment triplets](https://aclanthology.org/2020.emnlp-main.183/) | Target, opinion and polarity require a correct association. | Score the complete relation/direction/modality/grouping tuple, rather than positive words alone. Technical outcomes can be mixed and are not consumer sentiment. |
| [Temporal event relations](https://aclanthology.org/D19-1642/) | Event order and global consistency differ from token identity. | Preserve successive attempts and revisions. A temporal observation is not a causal effect; no temporal neural model or ILP solver is implemented. |
| [Snorkel](https://link.springer.com/article/10.1007/s00778-019-00552-1) | Imperfect labeling functions can be correlated. | Explicit abstaining weak-label votes, separately weighted. This small ablation does not implement Snorkel's generative denoising model. |
| [BADGE](https://arxiv.org/abs/1906.03671) | Acquisition should consider uncertainty and diversity. | Controlled pool simulation with entropy/diversity/rare-class weights, two seeds and random control. The greedy weighted selector is not BADGE's gradient embedding algorithm. |
| [Confident Learning](https://research.google/pubs/confident-learning-estimating-uncertainty-in-dataset-labels/) | Model disagreement can prioritize label audits. | A queue for independent annotation; labels are never silently replaced by predictions. No independent human review is fabricated. |
| [Calibration](https://proceedings.mlr.press/v70/guo17a.html) / [selective classification](https://arxiv.org/abs/1705.08500) | Confidence and correctness differ; rejection must report coverage. | Held-out temperature scaling, risk/coverage curves and false-success counts. A high marginal score is not a joint reliability guarantee. |
| [Conformal risk control](https://arxiv.org/abs/2208.02814) | Finite-sample control depends on the specified risk and assumptions. | A split-conformal set diagnostic, not a certificate of conditional precision. One calibration game and game drift preclude a broad gaming guarantee. |

## Objectives and statistics

The relation encoder learns four categorical heads: outcome, operation direction, speech act and grouping. The default objective is

`L = CE(outcome) + 0.4 CE(direction) + 0.35 CE(modality) + 0.2 CE(grouping) + λ Lcontrast`.

`Lcontrast` averages the negative log probability of same-outcome examples under cosine similarity at temperature 0.12, excluding self-pairs. Direction and grouping still need their own losses; same outcome does not imply the same operation. Synthetic direction reversals, successive attempts and joint outcomes are training data, never real measurements.

The separate evidence retriever uses normalized mean-pooled query/document embeddings with a symmetric multi-positive contrastive objective. Training query wording differs from evaluation wording. Topic labels establish relevance to a symptom, not that an action works. Retrieval is filtered by AppID before ranking.

Temperature scaling fits `softmax(logits / T)` on calibration labels. Split-conformal nonconformity uses `1 - p(true class)` and the finite-sample order statistic. Those sets address marginal label coverage under exchangeability, **not** the conditional correctness of accepted technical claims. The report includes Wilson intervals and a bootstrap over game clusters; with few games, neither should be read as a population guarantee.

Bayesian optimization, contextual bandits and multi-objective/safe optimization are deferred. An unreliable interpretation or a confounded community observation cannot serve as a trustworthy reward or justify an experiment on a user's game.
