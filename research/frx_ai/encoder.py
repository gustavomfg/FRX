"""Local quantized MiniLM, explicit masked mean pooling, four CPU threads."""
import pathlib,numpy as np
class Encoder:
    def __init__(self,path):
        import onnxruntime as ort
        from tokenizers import Tokenizer
        path=pathlib.Path(path); options=ort.SessionOptions();options.intra_op_num_threads=4;options.inter_op_num_threads=1
        self.session=ort.InferenceSession(str(path/'model_quint8_avx2.onnx'),sess_options=options,providers=['CPUExecutionProvider'])
        self.tokenizer=Tokenizer.from_file(str(path/'tokenizer.json'));self.tokenizer.enable_truncation(max_length=256);self.tokenizer.enable_padding(pad_id=0,pad_token='[PAD]')
    def encode(self,texts):
        all=[]
        for start in range(0,len(texts),16):
            batch=self.tokenizer.encode_batch(texts[start:start+16]);ids=np.array([x.ids for x in batch],dtype=np.int64);mask=np.array([x.attention_mask for x in batch],dtype=np.int64)
            feed={'input_ids':ids,'attention_mask':mask,'token_type_ids':np.zeros_like(ids)}
            output=self.session.run(None,feed)[0]; pooled=(output*mask[...,None]).sum(1)/mask.sum(1).clip(1)[:,None];pooled/=np.linalg.norm(pooled,axis=1,keepdims=True).clip(1e-9);all.append(pooled.astype(np.float32))
        return np.concatenate(all)
