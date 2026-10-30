"""Training-only counterfactuals. No templates derived from final comments."""
import random


def generate(n=320, seed=17):
    rng = random.Random(seed)
    rows = []
    targets = ["OPTION_X", "PROTON_NO_FSYNC", "DXVK_ASYNC", "a graphics option",
               "a legacy library", "the driver", "a configuration file", "Proton"]
    directions = {
        "enable": ["enabled", "turned on", "activated"],
        "disable": ["disabled", "turned off", "deactivated"],
        "install": ["installed", "added"], "remove": ["removed", "uninstalled"],
        "update": ["updated", "upgraded", "downgraded"],
        "replace": ["replaced", "copied"], "switch": ["switched to", "changed to"],
        "configure": ["configured", "adjusted"],
    }
    results = {
        "positive": ["The crashes stopped.", "That fixed the missing audio.",
                     "It now launches correctly.", "Frame pacing improved.", "This resolved the artifacts."],
        "not-resolved": ["The same crash still happens.", "It did not help at all.",
                         "There was no improvement.", "The issue persists.", "That didn't fix the problem."],
        "worsened": ["That caused distorted geometry and crashes.", "Performance got worse.",
                     "It introduced flickering.", "Now it freezes immediately instead.", "The frame times became worse."],
        "mixed": ["The crashes stopped, but performance got worse.",
                  "It launches now but the audio is still broken.",
                  "Stuttering improved but new artifacts appeared."],
    }
    for i in range(n):
        direction = list(directions)[i % len(directions)]
        verb = rng.choice(directions[direction]); target = rng.choice(targets)
        action = f"{verb} {target}"
        outcome = ["unknown", "positive", "not-resolved", "worsened", "mixed"][i % 5]
        modality = "tested"
        if outcome == "unknown":
            prefix = rng.choice(["Try to ", "You could ", "Could you "])
            # Present-tense variants avoid synthetic grammar as a label shortcut.
            base = {"enable":"enable", "disable":"disable", "install":"install",
                    "remove":"remove", "update":"update", "replace":"replace",
                    "switch":"switch to", "configure":"configure"}[direction]
            action = f"{base} {target}"
            text = prefix + action + ". Let us know if it helps."
            modality = "suggested"; result_span = None
        else:
            result = rng.choice(results[outcome])
            text = f"I {action}. {result}"
            result_span = [text.index(result), len(text)]
        joint = i % 9 == 0
        if joint:
            old = action
            action += " and installed a runtime"
            text = text.replace(old, action)
            if result_span:
                result_span = [result_span[0] + len(action) - len(old), len(text)]
            direction = "multiple"
        start = text.index(action)
        rows.append(dict(id=f"synthetic-{seed}-{i}", text=text,
                         action_span=[start, start + len(action)], result_span=result_span,
                         outcome=outcome, direction=direction, modality=modality,
                         joint=joint, origin="synthetic-training-only", split="train",
                         app_id=None, game=None, url=None, thread=f"synthetic-{seed}"))
    return rows


def hard_generate(n=320, seed=41):
    """Relational counterfactuals: same words, different edge/modality.

    Includes individual attempts followed by another successful attempt, and
    joint interventions where only the bundle has a reported outcome.
    """
    rng=random.Random(seed);rows=[]
    for i in range(n):
        x=rng.choice(["setting X","a launch option","an optional feature"])
        lib=rng.choice(["a runtime","a graphics library","a dependency"])
        verb,direction=rng.choice([("enabled","enable"),("disabled","disable")])
        typ=i%8
        if typ in [0,1]:
            action=f"{verb} {x}"
            result="did not help" if typ==0 else "fixed the crash"
            text=f"I {action}, which {result}. Then I installed {lib}. No result from that installation yet."
            y="not-resolved" if typ==0 else "positive";m="tested";joint=False
        elif typ==2:
            action=f"installed {lib}"
            text=f"I {verb} {x} and it fixed the crash. Later I {action}, but have not tested the game again."
            y="unknown";m="tested";direction="install";result=None;joint=False
        elif typ in [3,4]:
            both=f"{verb} {x} and installed {lib}"
            text=f"I {both}. The crashes stopped, but performance got worse. I do not know which change caused either effect."
            action=both if typ==3 else f"{verb} {x}"
            y="mixed" if typ==3 else "unknown";m="tested";joint=True
            direction="multiple" if typ==3 else direction
            result="The crashes stopped, but performance got worse." if typ==3 else None
        elif typ==5:
            action=f"enable {x}"
            text=f"You could {action} to see whether it fixes the crash. Nobody has tested this suggestion."
            y="unknown";m="suggested";direction="enable";result=None;joint=False
        elif typ==6:
            action=f"{verb} {x}"
            text=f"I {action}. That caused distorted geometry and crashes. Removing it restored the previous behavior."
            y="worsened";m="tested";result="That caused distorted geometry and crashes.";joint=False
        else:
            action=f"updated {lib}"
            text=f"The release notes say the maintainer {action}. I have not tried this version."
            y="unknown";m="unknown";direction="update";result=None;joint=False
        start=text.index(action)
        rows.append(dict(id=f"hard-{seed}-{i}",text=text,action_span=[start,start+len(action)],
            result_span=[text.index(result),text.index(result)+len(result)] if result else None,
            outcome=y,direction=direction,modality=m,joint=joint,origin="synthetic-training-only",
            split="train",app_id=None,game=None,url=None,thread=f"hard-{seed}"))
    return rows
