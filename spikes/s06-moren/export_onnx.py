"""Exports an own kana CTC model (finetune.py) to ONNX for kaiwa-speech, and checks that ONNX Runtime
gives the probabilities PyTorch gives.

Input `samples` (1 × n, float32, 16 kHz, normalised to zero mean and unit variance as the feature
extractor does), output `logits` (1 × frames × vocab). Next to `model.onnx` go `vocab.json` and
`model.int8.onnx`: int8 for the matrix multiplications only, per channel. Quantising every op
(the convolutions too) moved the variant scores by 7 in the median and is useless here; this way
they move by about 0.04, at a third of the size.

The check runs on speech (`--clips`: wav files): random input exaggerates the differences.

  python -I export_onnx.py --model ../../target/kana-ctc-v1/model --out ../../target/kana-ctc-v1/onnx \
      --clips ../../target/s06-real/clips/*.wav
"""

import argparse
import shutil
from pathlib import Path

import numpy as np
import torch


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--model", required=True)
    ap.add_argument("--out", required=True)
    ap.add_argument("--clips", nargs="+", required=True)
    args = ap.parse_args()
    from transformers import Wav2Vec2ForCTC
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    model = Wav2Vec2ForCTC.from_pretrained(args.model).eval()

    class Logits(torch.nn.Module):
        def __init__(self, m):
            super().__init__()
            self.m = m

        def forward(self, samples):
            return self.m(samples).logits

    x = torch.randn(1, 16000 * 3)
    torch.onnx.export(Logits(model), (x,), out / "model.onnx", input_names=["samples"], output_names=["logits"],
                      dynamic_axes={"samples": {1: "n"}, "logits": {1: "frames"}}, opset_version=17,
                      dynamo=False)
    shutil.copy(Path(args.model) / "vocab.json", out / "vocab.json")

    from onnxruntime.quantization import QuantType, quantize_dynamic
    quantize_dynamic(str(out / "model.onnx"), str(out / "model.int8.onnx"), weight_type=QuantType.QInt8,
                     op_types_to_quantize=["MatMul"], per_channel=True)

    import librosa
    import onnxruntime as ort
    from transformers import Wav2Vec2FeatureExtractor
    fe = Wav2Vec2FeatureExtractor.from_pretrained(args.model)
    # The export changes the traced model in place; compare against a fresh copy.
    model = Wav2Vec2ForCTC.from_pretrained(args.model).eval()
    for f in ("model.onnx", "model.int8.onnx"):
        sess = ort.InferenceSession(str(out / f))
        worst = []
        for clip in args.clips[:20]:
            a, _ = librosa.load(clip, sr=16000)
            x = fe(np.pad(a, 8000), sampling_rate=16000, return_tensors="np").input_values.astype(np.float32)
            with torch.no_grad():
                want = torch.log_softmax(model(torch.from_numpy(x)).logits, -1).numpy()
            got = torch.log_softmax(torch.from_numpy(sess.run(None, {"samples": x})[0]), -1).numpy()
            worst.append(np.abs(np.exp(got) - np.exp(want)).max())
        print(f, "largest probability difference %.4f (median over clips %.4f)" % (max(worst), np.median(worst)))
    for f in ("model.onnx", "model.int8.onnx"):
        print(f, "%.0f MB" % ((out / f).stat().st_size / 1e6))


if __name__ == "__main__":
    main()
