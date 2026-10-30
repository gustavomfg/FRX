"""Goal-conditioned edge representation; retains different effects separately."""
import re
from pairs import result_proposals,overlaps,RESULT


def goal_proposals(text,action_span,limit=6):
    spans=[]
    for a,b in result_proposals(text,action_span,12):
        last=a
        for m in re.finditer(r'\b(?:but|however|although|whereas)\b',text[a:b],re.I):
            end=a+m.start()
            if end-last>5 and RESULT.search(text[last:end]):spans.append([last,end])
            last=a+m.end()
        if b-last>5 and (RESULT.search(text[last:b]) or re.search(r'\b(?:not|never|still)\b',text[last:b],re.I)):
            spans.append([last,b])
    a,b=action_span
    spans.sort(key=lambda s:(min(abs(s[0]-b),abs(s[1]-a)),s[0]))
    return spans[:limit]


def synthetic_goals(rows):
    output=[]
    for row in rows:
        if row['outcome']=='mixed':
            text=row['text'];r=row['result_span']
            if not r:continue
            quote=text[r[0]:r[1]];m=re.search(r'\bbut\b',quote)
            if not m:continue
            first=[r[0],r[0]+m.start()];second=[r[0]+m.end(),r[1]]
            y='worsened' if re.search('worse|new artifacts',text[second[0]:second[1]],re.I) else 'not-resolved'
            for span,outcome in [(first,'positive'),(second,y)]:
                output.append(dict(row,candidate_result_span=span,result_span=span,outcome=outcome))
        else:
            spans=[row['result_span']] if row.get('result_span') else goal_proposals(row['text'],row['action_span'],1) or [None]
            for span in spans:output.append(dict(row,candidate_result_span=span))
    return output
