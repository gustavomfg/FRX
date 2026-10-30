"""Bounded ONNX inference: no torch, network, subprocess or application imports."""
import hashlib
import json
import pathlib
import numpy as np
from representation import representation,OUTCOMES,DIRECTIONS,MODALITIES
from contracts import check_evidence,checked_span,graph
from proposals import propose
from pairs import inference_pairs,reduce_predictions,result_proposals


def softmax(x):
    x=x-x.max(axis=1,keepdims=True);p=np.exp(x)
    return p/p.sum(axis=1,keepdims=True)


class Runtime:
    def __init__(self,directory):
        import onnxruntime as ort
        from tokenizers import Tokenizer
        directory=pathlib.Path(directory)
        self.meta=json.loads((directory/'runtime.json').read_text())
        if self.meta['mode'] not in ['marked','clauses','masked','pair']:
            raise ValueError('non-deployable representation')
        model=directory/self.meta['model_file']
        if model.parent.resolve()!=directory.resolve():raise ValueError('model outside research directory')
        if model.stat().st_size>140_000_000:raise ValueError('model size bound')
        if hashlib.sha256(model.read_bytes()).hexdigest()!=self.meta['sha256']:
            raise ValueError('model integrity mismatch')
        tokenizer_path=directory/'tokenizer.json'
        if hashlib.sha256(tokenizer_path.read_bytes()).hexdigest()!=self.meta['tokenizer_sha256']:
            raise ValueError('tokenizer integrity mismatch')
        options=ort.SessionOptions();options.intra_op_num_threads=4;options.inter_op_num_threads=1
        self.session=ort.InferenceSession(str(model),sess_options=options,providers=['CPUExecutionProvider'])
        self.tokenizer=Tokenizer.from_file(str(tokenizer_path))
        self.tokenizer.enable_truncation(max_length=192,strategy='only_second')
        self.tokenizer.enable_padding(length=192,pad_id=0,pad_token='[PAD]')

    def tokens(self,rows):
        batch=self.tokenizer.encode_batch([representation(r,self.meta['mode']) for r in rows])
        return dict(input_ids=np.array([x.ids for x in batch],dtype=np.int64),
            attention_mask=np.array([x.attention_mask for x in batch],dtype=np.int64),
            token_type_ids=np.array([x.type_ids for x in batch],dtype=np.int64))

    def raw(self,rows):
        outputs=[[] for _ in range(4)]
        for start in range(0,len(rows),16):
            result=self.session.run(None,self.tokens(rows[start:start+16]))
            for j in range(4):outputs[j].append(result[j])
        return [np.concatenate(x) for x in outputs]

    def classify(self,rows):
        if self.meta['mode']=='pair' and all('candidate_result_span' in r for r in rows):
            raw=self.raw(rows);witnesses=[r['candidate_result_span'] for r in rows]
        elif self.meta['mode']=='pair':
            candidates,groups=inference_pairs(rows)
            raw,selected=reduce_predictions(self.raw(candidates),groups)
            witnesses=[candidates[i]['candidate_result_span'] for i in selected]
        else:
            raw=self.raw(rows)
            witnesses=[next(iter(result_proposals(r['text'],r['action_span'],1)),None) for r in rows]
        return raw,witnesses

    def analyze(self,request):
        if not isinstance(request,dict):raise ValueError('JSON object required')
        context=request.get('context',{});evidence=request.get('evidence',[])
        if not isinstance(evidence,list) or len(evidence)>8:raise ValueError('at most eight evidence sources')
        rows=[];rejected=[];limited=[]
        for item in evidence:
            reason=check_evidence(context,item)
            if reason:
                rejected.append(dict(source=item.get('url') if isinstance(item,dict) else None,reason=reason));continue
            spans=[item['action_span']] if 'action_span' in item else propose(item['text'],7)
            if len(spans)>6:
                limited.append(item['url']);spans=spans[:6]
            for span in spans:
                checked_span(item['text'],span)
                rows.append(dict(item,action_span=span,proposal_origin='caller' if 'action_span' in item else 'deterministic'))
        results=[]
        if rows:
            if self.meta.get('task')=='goal-pair':
                from goals import goal_proposals
                expanded=[]
                for row in rows:
                    for span in goal_proposals(row['text'],row['action_span'],3) or [None]:
                        expanded.append(dict(row,candidate_result_span=span))
                rows=expanded
            logits,witnesses=self.classify(rows)
            temperatures=self.meta.get('temperatures',[1.]*4)
            probs=[softmax(x/t) for x,t in zip(logits,temperatures)]
            preds=np.stack([p.argmax(1) for p in probs],1)
            confidence=np.stack([p.max(1) for p in probs[:3]],1).min(1)
            for i,(row,p) in enumerate(zip(rows,preds)):
                g=graph(row,OUTCOMES[p[0]],DIRECTIONS[p[1]],MODALITIES[p[2]],bool(p[3]),confidence[i],witnesses[i])
                g['proposal_origin']=row['proposal_origin']
                g['result_quote_status']='learned-edge-candidate' if self.meta['mode']=='pair' else 'heuristic-support-candidate'
                g['selective_candidate']=bool(p[0]!=0 and p[1]!=0 and p[2]==2 and confidence[i]>=self.meta.get('threshold',.7))
                g['reported_result']='unknown'  # research approval is separate from a model score
                results.append(g)
        return dict(status='experimental-local-analysis',model=self.meta['model_name'],
            symptom=request.get('symptom'),context=context,candidates=results,rejected=rejected,
            sources_with_truncated_action_proposals=limited,
            causes_confirmed=[],execution_authorized=False,
            uncertainties=['Hardware used by the game is not inferred from a reported GPU.',
                           'Actions and results need independent review before integration.'])
