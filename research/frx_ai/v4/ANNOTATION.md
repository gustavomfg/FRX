# Event annotation contract, v4-events-1

Old labels are immutable. New annotations are provisional agent interpretations, not human-reviewed gold. Every categorical claim must point to literal evidence; missing identities remain null. Do not add an event merely to match an old label.

- **Trigger:** the smallest verb/operation token(s) indicating the technical event. Installing a dependency, selecting a rendering setting, removing a driver and trying a Proton version are separate events. A noun such as “any update?” is not a trigger. Metadata/log “enabled (table …)” is not a user configuration attempt.
- **Target:** an explicit literal argument. “The missing library” is a description, not an identified file; its exact identity remains unknown. A DLL filename does not establish game technology support. Variable assignment direction is about the variable, not inferred underlying feature semantics.
- **Status:** `performed` means the source explicitly asserts execution, not that FRX observed it or that it worked. `suggested` includes advice/plans. `negated` means execution was denied or unavailable. `preexisting` is an already-present state, not a new attempt. Otherwise `unknown`.
- **Observation:** a separate literal result span, with zero or more associated event indices. Association with several actions is `joint`; no individual causal credit. A symptom predating an action is not its outcome. Unknown/retracted attribution remains unknown even if the text sounds positive.
- **Temporal edge:** only an explicit cue such as “then”, “after” or “before” supports an edge. Text order is separately available and never proof of causality.
- **Scope:** bounded authored paragraph, with full source retained outside Git and the source offset/hash preserved. Results apply to the paragraph's reported objective. Later edits, other goals, version changes and contradictory full-source context prevent an overall “resolved” assertion.

Evaluation matches triggers by overlap and also reports exact spans. Modality and direction are evaluated on matched events. False performed claims include an unmatched procedural event. A result association must match its owner/group and literal observation, not just polarity. Explicit quotes are verified mechanically; correct quotation alone does not prove correct interpretation.
