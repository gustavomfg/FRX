"""Local data-only semantic retrieval; no learned score establishes a cause."""
import hashlib
import json
import pathlib
import numpy as np


class Retriever:
    def __init__(self,directory):
        import onnxruntime as ort
        from tokenizers import Tokenizer
        directory=pathlib.Path(directory);self.meta=json.loads((directory/'runtime.json').read_text())
        file=directory/self.meta['model_file']
        if file.parent.resolve()!=directory.resolve() or file.stat().st_size>140_000_000:raise ValueError('retrieval model bound')
        if hashlib.sha256(file.read_bytes()).hexdigest()!=self.meta['sha256']:raise ValueError('retrieval model integrity')
        tok=directory/'tokenizer.json'
        if hashlib.sha256(tok.read_bytes()).hexdigest()!=self.meta['tokenizer_sha256']:raise ValueError('retrieval tokenizer integrity')
        options=ort.SessionOptions();options.intra_op_num_threads=4;options.inter_op_num_threads=1
        self.session=ort.InferenceSession(str(file),sess_options=options,providers=['CPUExecutionProvider'])
        self.tokenizer=Tokenizer.from_file(str(tok));self.tokenizer.enable_truncation(max_length=192);self.tokenizer.enable_padding(pad_id=0,pad_token='[PAD]')

    def encode(self,texts):
        out=[]
        for start in range(0,len(texts),16):
            batch=self.tokenizer.encode_batch(texts[start:start+16])
            feed=dict(input_ids=np.array([x.ids for x in batch],dtype=np.int64),
                attention_mask=np.array([x.attention_mask for x in batch],dtype=np.int64),
                token_type_ids=np.array([x.type_ids for x in batch],dtype=np.int64))
            out.append(self.session.run(None,feed)[0])
        return np.concatenate(out)
