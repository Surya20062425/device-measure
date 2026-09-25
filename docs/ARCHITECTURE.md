# Architecture

## System overview

```
                        core-math (Rust)
              distance / diameter / uncertainty / tracking
                       |                      |
                compiled to WASM       compiled to JNI .so
                       |                      |
                   web-app                android-app
              React + TypeScript      Kotlin + Compose
              getUserMedia camera     Camera2 API (real intrinsics)
              ONNX Runtime Web        TFLite
              manual calibration      auto calibration from hardware
                       \                      /
                        \                    /
                     shared detection models
                    (.onnx for web, .tflite for Android,
                     exported from the same training pipeline)
```

## Why the math core is separate from everything else

`core-math` contains no camera access, no model inference, and no UI. It is
pure numeric/geometric functions: given pixel measurements and camera
intrinsics, compute distance; given distance and pixel measurements, compute
diameter; given a stream of noisy per-frame measurements, compute a smoothed
estimate. This is deliberate:

- It's the one piece of logic that must behave *identically* on Android and
  web — duplicating it in Kotlin and TypeScript risks the two silently
  drifting apart as the project evolves.
- It's the easiest part of the system to test exhaustively (see
  `core-math/src/*.rs` test modules) since it has no I/O or platform
  dependencies.
- It compiles cleanly to both WASM (web) and a shared object consumable via
  JNI (Android) from one source.

## The distance/size ambiguity, and how each backend resolves it

A single 2D image cannot recover both an object's real-world size and its
distance from the camera — a large object far away and a small object close
up produce identical pixels. Every distance method in `core-math::distance`
resolves this ambiguity by injecting a different piece of outside
information:

| Method | Resolves ambiguity via | Accuracy ceiling | Requires |
|---|---|---|---|
| `distance_from_known_width` | Assumed known real-world width of the object class | ~3-8% error, dominated by pixel-edge localization | The object's true width already being known |
| `distance_from_stereo_disparity` | Triangulation between two cameras at a known baseline | ~1-3% at close range, degrades ~quadratically with distance | Calibrated stereo pair |
| `distance_from_depth_sensor` | Direct hardware measurement (ToF/structured light/LiDAR) | Sensor-datasheet-dependent, typically best available | Depth-capable hardware |

Diameter (`core-math::diameter::diameter_from_distance`) is derived from
whichever distance estimate was used, which means **diameter accuracy is
capped by distance accuracy** — this is enforced in the API itself: the
function requires a `Measurement` (value + std_dev), not a bare distance
number, specifically so error can't be silently dropped on the way in.

## Uncertainty propagation

Every `core-math` function that estimates a physical quantity returns a
`Measurement { value, std_dev }`, not a bare `f64`. Uncertainty is
propagated via first-order Taylor expansion (standard error propagation) from
the uncertainty of the inputs — see the doc comments in `distance.rs` and
`diameter.rs` for the derivative-based derivation of each. This is what lets
the UI honestly render `7.4cm ± 0.3cm` instead of a falsely precise number.

## Why bounding boxes are not the same as true object width

Object detectors return axis-aligned bounding boxes, which include
background pixels and are systematically wrong for rotated or
off-axis-photographed objects (foreshortening). `diameter_from_distance`
documents this as a caller responsibility: for accuracy-sensitive use, pixel
width should come from a segmentation mask or fitted contour/ellipse, not a
raw bbox. This is a **systematic bias**, not random noise, so it is not
captured by the `std_dev` uncertainty propagation — it must be corrected
upstream before calling into `core-math`.

## Detection: tiered strategy, not a single "detect everything" model

Standard object detectors are closed-set (fixed class list at training
time), so "detect every device" is not literally achievable with one model.
The plan is a two-tier detector:

- **Fast path**: a YOLO variant fine-tuned on a curated, growing "devices"
  dataset — runs every frame, real-time.
- **Open-vocabulary fallback** (YOLO-World / Grounding DINO): text-prompted,
  broader coverage, but slower and less precise — runs on-demand, not every
  frame.

See [ADDING_A_DEVICE_CLASS.md](ADDING_A_DEVICE_CLASS.md) for how the fast
path's coverage grows over time via community contributions.

## Platform asymmetry: why Android and web have different accuracy ceilings

Android's Camera2 API can report real focal length and sensor size from
hardware (`CameraCharacteristics`), so the Android app can auto-calibrate.
Browsers' `getUserMedia` API exposes none of this, so the web app requires a
manual checkerboard calibration flow (see `calibration/`) or explicit
known-reference-object input. This is a structural, not incidental,
difference — the project reports accuracy separately per platform in
`benchmarks/` rather than implying parity.

## Tracking and smoothing

`core-math::tracking::RollingHistory` takes a stream of single-frame
measurements for **one already-identified** tracked object and produces a
smoothed estimate (rolling median, robust to single-frame outliers like a
partial occlusion glitch). It does **not** perform frame-to-frame identity
assignment — that's the job of a tracker (SORT/ByteTrack) living in the
platform-native detection layer, which assigns the stable object IDs that
`RollingHistory` instances are keyed by.

## Where an LLM/RAG layer fits (and where it doesn't)

RAG cannot estimate distance or diameter — there's no retrievable document
containing "this specific object, at this angle, in this frame, is 87cm
away." That number must be computed geometrically. RAG *does* have a
legitimate role for a different problem: once an object is identified as a
**specific known product** (e.g. via fine-grained classification beyond
generic "phone" detection), its exact manufacturer-published dimensions can
be retrieved instead of estimated — strictly more accurate than any
CV-based estimate for that subset of objects. This is planned as a
post-core-pipeline addition; see [ROADMAP.md](../ROADMAP.md) Phase 4.5.
