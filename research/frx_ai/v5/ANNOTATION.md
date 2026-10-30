# v5 annotation and replay contract

All labels are provisional agent annotations, without independent human review. Preserve older labels. An audit flag is not a corrected label. Published prior final cohorts are known development material.

An event has a minimal operation trigger, optional literal target spans, status cue, operation direction, and role: corrective, diagnostic, reproduction, or unknown. If `tried` introduces a concrete operation, prefer that operation as the event trigger; do not count the wrapper twice. `performed` means the source asserts an attempted/performed operation, not successful completion or a local FRX observation. `suggested` includes plans. `negated` denies execution; inability to complete is a limitation, not automatic proof no attempt happened. `preexisting` denotes existing configuration. Otherwise abstain.

An observation is separate from an action. Preserve its literal predicate, goal and polarity. An error mentioned before a change is not automatically its outcome. Improvements and regressions can coexist for different goals. An association's owners are an exact event/group, with the textual connector supporting the edge. Joint outcomes never give individual causal credit. Temporal order and community attribution never establish independently verified causality.

Targets can be descriptive, such as “the missing library”. Exact identity remains null without a literal filename, variable or documented component identity. GPU presence does not prove game technology; a module loader error does not identify an appropriate package to install.

Keep full-source hashes and bounded-unit offsets outside Git. Sources in a game thread establish thread attribution, not every event's exact game/build/hardware. Preserve ambiguity and contradictory follow-ups. No source text, shell snippet, instruction, hypothesis or model output is executable.

Incident snapshots retain available observations separately from withheld future evidence, expected checks and outcomes. Expected checks are evaluation labels, never inference input. A replay can reveal only a cached, authorized evidence item, cannot run a system tool, launch a game or fabricate its contents. Matching prior attempts requires AppID/build/GPU/driver/Proton/technology/configuration/symptom. Unknown context cannot silently inherit measured outcomes. Duplicate comments from a discussion are one dependent source group.
