# OCR models

`text-detection.onnx` and `text-recognition.onnx` are the [Ocrs](https://github.com/robertknight/ocrs) models
from [robertknight/ocrs-models](https://github.com/robertknight/ocrs-models), bundled because upstream no longer
guarantees its download URLs ([robertknight/ocrs#282](https://github.com/robertknight/ocrs/issues/282)).
`engine/build.rs` embeds them in the binary when the `ocr` feature is enabled.
