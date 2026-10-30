"""FRX AI research JSONL protocol. Local inference only. NEVER executes fixes.

Input: {context: {app_id, game?, gpu?, driver?, proton?}, symptom, evidence: [...]}.
Each evidence must carry its own app_id, URL, text and target (or <procedure>).
Output predictions are review-only because the research quality gate failed.
"""
import argparse,hashlib,json,pathlib,sys,re,time
import numpy as np
from core import clean,window,state_of,LABELS
from nli import NLI
from encoder import Encoder

MAX_TEXT=2600;MAX_EVIDENCE=32;MAX_INPUT=128*1024

def verify_model(directory):
    manifest=json.loads((directory/'manifest.json').read_text())
    for item in manifest['files']:
        path=directory/pathlib.PurePosixPath(item['file']).name
        if hashlib.sha256(path.read_bytes()).hexdigest()!=item['sha256']:
            raise ValueError('model integrity mismatch: '+path.name)
    return manifest

def validate(request):
    if not isinstance(request,dict):raise ValueError('request must be an object')
    context=request.get('context',{})
    if not isinstance(context,dict):raise ValueError('invalid context')
    for key in ['gpu','driver','proton']:
        if context.get(key) is not None and not isinstance(context[key],str):raise ValueError('invalid context field: '+key)
    app=str(context.get('app_id',''))
    if not re.fullmatch(r'[0-9]{1,12}',app):raise ValueError('explicit numeric AppID required')
    evidence=request.get('evidence',[])
    if not isinstance(evidence,list) or len(evidence)>MAX_EVIDENCE:raise ValueError('at most 32 evidence items')
    symptom=request.get('symptom','')
    if not isinstance(symptom,str) or len(symptom)>500:raise ValueError('invalid symptom')
    usable=[];excluded=[]
    for i,e in enumerate(evidence):
        reason=None
        if not isinstance(e,dict) or not isinstance(e.get('context',{}),dict):raise ValueError('invalid evidence object')
        if str(e.get('app_id',''))!=app:reason='different-or-unknown-game'
        elif not isinstance(e.get('text'),str) or len(e['text'])>MAX_TEXT:reason='text-too-large-or-invalid'
        elif not re.fullmatch(r'https://github\.com/[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+/(?:issues|discussions)/[0-9]+(?:#[A-Za-z0-9_-]+)?',str(e.get('url',''))):reason='source-url-unverified'
        elif not isinstance(e.get('target'),str) or not e['target'] or len(e['target'])>100:reason='action-target-unidentified'
        elif e.get('synthetic',False) or e.get('origin','').startswith('synthetic'):reason='synthetic-evidence-excluded'
        if reason:excluded.append({'index':i,'reason':reason});continue
        if any(e.get('context',{}).get(k) is not None and not isinstance(e['context'][k],str) for k in ['gpu','driver','proton']):raise ValueError('invalid source context field')
        # Context supplied by sources is retained; never silently generalize GPU/Proton evidence.
        mismatch=[k for k in ['gpu','driver','proton'] if context.get(k) and e.get('context',{}).get(k) and context[k].casefold()!=e['context'][k].casefold()]
        if mismatch:excluded.append({'index':i,'reason':'context-mismatch','fields':mismatch});continue
        usable.append(e)
    return context,symptom,usable,excluded

class ResearchAI:
    def __init__(self,root):
        self.root=pathlib.Path(root);self.nli_meta=verify_model(self.root/'nli');verify_model(self.root/'encoder')
        self.nli=NLI(self.root/'nli');self.encoder=Encoder(self.root/'encoder')
        self.head=json.loads((self.root/'experiments/report-head.json').read_text())
        if self.head.get('automatic_acceptance') is not False:raise ValueError('research model cannot enable automatic acceptance')
    def analyze(self,request):
        context,symptom,rows,excluded=validate(request);start=time.perf_counter();items=[]
        if rows:
            embeddings=self.encoder.encode([symptom]+[clean(e['text']) for e in rows]);scores=embeddings[1:]@embeddings[0]
            order=np.argsort(-scores)[:8];chosen=[rows[i] for i in order];features=self.nli.features(chosen)
            w=np.array(self.head['coefficient']);b=float(self.head['intercept']);report_prob=1/(1+np.exp(-(features@w+b)))
            raw=features.reshape(len(chosen),5,3)[:,:,1]
            for j,(row,i) in enumerate(zip(chosen,order)):
                suggested=int(raw[j].argmax());text=row['text'];state=state_of(row)
                items.append({'source':{'url':row['url'],'app_id':str(row['app_id']),'game':row.get('game'),'published_at':row.get('published_at')},'source_context':row.get('context',{}),'evidence_kind':'community-source','quote':{'text':text,'start':0,'end':len(text),'sha256':hashlib.sha256(text.encode()).hexdigest()},'action_candidate':{'target':None if row['target']=='<procedure>' else row['target'],'state_candidate':state,'mentioned_in_authored_text':row['target']!='<procedure>' and bool(re.search(r'(?<![A-Za-z0-9_])'+re.escape(row['target'])+r'(?![A-Za-z0-9_])',clean(text))),'combined_with':sorted(set(re.findall(r'\b(?:PROTON|VKD3D|DXVK)_[A-Z0-9_]+\b',clean(text)))-{row['target']})},'reported_result':'unknown','experimental_prediction':{'report_score':float(report_prob[j]),'outcome_candidate':LABELS[suggested],'nli_entailment_scores':raw[j].tolist()},'retrieval_score':float(scores[i]),'accepted_automatically':False,'requirements_confirmed':False,'uncertainties':['Learned result extraction did not meet the held-out engineering target.','Retrieval similarity and classifier scores are not safety or causal evidence.','Source hardware and Proton may be missing or differ; no local performance claim.']})
        return {'schema_version':1,'mode':'research-review-only','context':context,'symptom':symptom,'evidence':items,'excluded':excluded,'hypotheses':[],'solution_plans':[],'execution_authorized':False,'elapsed_ms':(time.perf_counter()-start)*1000,'limitations':['No automatic success, regression or cause assertion. Evidence supports review only.','Static retrieval uses an English pretrained encoder; multilingual quality was not evaluated.']}

def main():
    parser=argparse.ArgumentParser();parser.add_argument('--models',type=pathlib.Path,required=True);args=parser.parse_args();ai=ResearchAI(args.models)
    while True:
        line=sys.stdin.buffer.readline(MAX_INPUT+1)
        if not line:break
        if len(line)>MAX_INPUT:
            print(json.dumps({'error':'input limit exceeded','execution_authorized':False}),flush=True)
            break
        try:
            if len(line)>MAX_INPUT:raise ValueError('input limit exceeded')
            result=ai.analyze(json.loads(line))
        except (ValueError,KeyError,TypeError) as error:result={'error':str(error),'execution_authorized':False}
        print(json.dumps(result,ensure_ascii=False),flush=True)
if __name__=='__main__':main()
