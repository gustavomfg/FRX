# Separate post-language-evaluation experiment: learned diagnostic dependence

The language/retrieval v3 final has been evaluated and remains frozen. It is not used in this family. Earlier synthetic results suggested that naive conditional independence double-counts duplicated sensor observations. Test a learned dependency structure against the same naive features and a learned exact-duplicate grouping control.

Data: the declared simulator in `simulation.py`, 1,200 training cases (seed 17), 300 known development cases (29). Reserve NEW unseen synthetic final seeds 131 (600 cases) and 137 (600 shifted-noise cases). Models: naive Bayes, exact-duplicate groups learned from train columns, and a tree-augmented Bayes structure learned by conditional mutual information and a maximum spanning tree. No model sees final cases while fitting or selecting.

All models use the same empirical prior and Laplace alpha=1. Infer by enumerating 128 possible seven-sensor states; no handpicked causal diagnosis probabilities. Compare static versus expected-information-gain/cost inspection, max five checks, posterior stopping threshold 0.9 fixed from the earlier protocol. Select on development: an EIG policy must remain within 0.02 accuracy of that model's static policy and have no more than five additional falsely confident cases; otherwise retain static. Among eligible policies use minimum mean cost. Freeze before generating either new final sample.

Metrics: accuracy, inspection count, unitless cost, Brier score and falsely confident diagnoses. Report noise shift and the prior-only chance floor. These are closed-world single-fault SYNTHETIC cases, not Linux gaming accuracy or validated real-world costs. Do not install a learned simulation model in the real-evidence prototype.
