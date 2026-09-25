# device-measure

An open-source system that detects devices/objects in real time via camera,
estimates their **distance** and **diameter** using computer vision, and
displays full measurement details per object — available as both an
**Android app** and a **web app**, with community-extensible detection
coverage.

## Status

🚧 **Phase 0 — Foundation.** The shared `core-math` library (distance,
diameter, uncertainty propagation, temporal smoothing) is implemented and
tested. No app, no models, no UI yet. See [ROADMAP.md](ROADMAP.md) for the
full build plan and current phase.

## What this is not

Read this before filing an issue expecting these:

- **Not** a system that detects literally any object — it uses a fast
  closed-set detector for known device classes, with a slower open-vocabulary
  fallback for the long tail. Coverage grows via community contributions.
  See [docs/ADDING_A_DEVICE_CLASS.md](docs/ADDING_A_DEVICE_CLASS.md).
- **Not** lab-grade metrology. Every measurement is reported with an honest
  uncertainty range (`7.4cm ± 0.3cm`), not a bare number pretending to be
  more precise than the underlying method allows.
- **Not** equally accurate on both platforms. Android has a structurally
  higher accuracy ceiling because it can read real camera hardware
  intrinsics (focal length, sensor size) via Camera2. The web app relies on
  manual calibration since browsers don't expose this. Both are documented
  and benchmarked separately — see [benchmarks/](benchmarks/).

## Why distance/diameter can't come from an LLM alone

A single 2D image cannot disambiguate object size from distance — a large
object far away and a small object close up can produce identical pixels.
This is resolved geometrically (known reference width, stereo triangulation,
or a depth sensor), not by "asking a model to look harder." See
[docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for the full reasoning and how
each distance backend resolves the ambiguity differently, with its own
accuracy ceiling.

## Repository layout

```
core-math/      Rust: distance, diameter, uncertainty, tracking/smoothing
                (platform-agnostic, compiled to WASM for web + JNI for Android)
models/         Detection model training scripts and exported .tflite/.onnx
android-app/    Kotlin + Jetpack Compose, Camera2, TFLite            [not started]
web-app/        React + TypeScript, ONNX Runtime Web                 [not started]
data/           Device class list + community-contributed training images
calibration/    Camera calibration tooling (auto for Android, manual for web)
benchmarks/     Ground-truth objects + accuracy reporting, per platform/backend
docs/           Architecture, contributing, and extension guides
```

## Building `core-math`

```
cd core-math
cargo test      # run the full unit + integration test suite
cargo build --release
```

## Contributing

See [docs/CONTRIBUTING.md](docs/CONTRIBUTING.md). Good first contributions:
adding a training image for a device class, adding a new distance backend,
improving calibration docs.

## License

MIT — see [LICENSE](LICENSE).
