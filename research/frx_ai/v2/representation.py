"""Pure text representation shared by training and native local inference."""
import re

OUTCOMES = ["unknown", "positive", "not-resolved", "worsened", "mixed"]
DIRECTIONS = ["unknown", "enable", "disable", "install", "remove", "update",
              "replace", "switch", "configure", "multiple"]
MODALITIES = ["unknown", "suggested", "tested"]
ENV = re.compile(r"\b(?:PROTON_[A-Z0-9_]+|VKD3D_[A-Z0-9_]+|DXVK_[A-Z0-9_]+|WINE[A-Z0-9_]*|RADV_[A-Z0-9_]+)\b")


def representation(row, mode="marked"):
    text = row["text"]
    a, b = row["action_span"]
    action = text[a:b]
    if mode == "pair":
        result=row.get("candidate_result_span")
        quote=text[result[0]:result[1]] if result else "No result candidate."
        context=f"Action: {action}. Candidate observation: {quote}. Surrounding context: " + text[max(0,a-140):min(len(text),b+380)]
    elif mode == "oracle":
        # DIAGNOSTIC UPPER BOUND ONLY: requires annotated result spans.
        # Never select this mode for the final prototype.
        result = row.get("result_span")
        quote = text[result[0]:result[1]] if result else "No attributed result."
        context = f"Action: {action}. Observation: {quote}. Context: " + text[max(0,a-100):min(len(text),b+240)]
    elif mode == "clauses":
        context = text[max(0,a-140):a] + " [unused1] " + action + " [unused2] " + text[b:min(len(text),b+440)]
        # Preserve the final authored edit; do not silently drop a retraction.
        edits = list(re.finditer(r"(?im)^\s*(?:\*\*)?edit\b",text))
        if edits and edits[-1].start() > b+440:
            context += " Final revision: " + text[edits[-1].start():][:240]
    elif mode == "whole":
        context = text[:1800]
    else:
        # Include following revisions within the bounded window. Neither gold
        # result spans nor outcome labels are used as model input.
        left, right = max(0, a - 400), min(len(text), b + 900)
        context = text[left:a] + " [unused1] " + action + " [unused2] " + text[b:right]
    context = re.sub(r"https?://\S+", "URL", context)
    context = re.sub(r"@[\w-]+", "USER", context)
    if mode == "masked":
        context = ENV.sub("OPTION", context)
        action = ENV.sub("OPTION", action)
    return action, context

