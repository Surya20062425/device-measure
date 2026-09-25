# Roadmap

## Phase 0 — Foundation ✅ (current)
- [x] Repo structure, license, root docs
- [x] `core-math` Rust crate: distance, diameter, uncertainty, tracking/smoothing
- [x] Unit + integration test suite (26 tests, all passing)
- [ ] Compile `core-math` to WASM, verify in a bare HTML test page
- [ ] Compile `core-math` to Android via JNI, verify in a bare Android test app
- [ ] CI (GitHub Actions): run `cargo test` on every PR

## Phase 1 — Android MVP
- [ ] Camera2 integration + auto-calibration from hardware intrinsics
- [ ] TFLite fast-path detector (start with stock YOLOv8n / COCO device-relevant classes)
- [ ] Wire in `core-math` (via JNI) for distance/diameter
- [ ] Basic bbox overlay, single object, no tracking yet
- **Milestone**: distance+diameter shown on screen for one object, validated by hand with a tape measure/ruler

## Phase 2 — Tracking + Details Panel (Android)
- [ ] SORT/ByteTrack for stable per-object IDs
- [ ] `RollingHistory` wired per tracked ID for smoothing
- [ ] Tab strip + details side panel UI (class, confidence, distance, diameter, uncertainty, history)
- **Milestone**: multiple objects tracked simultaneously, selectable, live-updating detail panels

## Phase 3 — Web MVP
- [ ] getUserMedia camera access
- [ ] Manual checkerboard calibration flow
- [ ] ONNX Runtime Web + same detection model (exported to ONNX)
- [ ] `core-math` via WASM
- [ ] Port UI patterns from Android (React tab strip + panel)
- **Milestone**: web app functionally matches Android MVP; lower accuracy ceiling clearly documented in-UI

## Phase 4 — Device coverage expansion
- [ ] Curate device-class training dataset (Open Images V7 + LVIS + community contributions)
- [ ] Fine-tune custom device detector, export to `.tflite` + `.onnx`
- [ ] Add YOLO-World/Grounding DINO open-vocab fallback (opt-in, on-demand)
- **Milestone**: v0.1 public release — N known device classes, fast path + fallback mode

## Phase 4.5 — Product Spec RAG Layer (stretch, post-core-pipeline)
- [ ] Fine-grained product classifier (e.g. "iPhone 15 Pro", not just "phone")
- [ ] Vector DB of product specs (Chroma/FAISS), community-contributed entries
- [ ] Retrieval: detected product → exact manufacturer dimensions, bypassing CV estimation entirely
- [ ] Fallback to CV pipeline for unrecognized/generic objects
- See `docs/ARCHITECTURE.md` for why this is additive, not a replacement for the CV pipeline

## Phase 5 — Precision hardening
- [ ] Sub-pixel edge refinement (Canny/contour fitting inside detected boxes)
- [ ] Segmentation-based diameter (SAM/YOLO-seg) instead of raw bbox width
- [ ] Full uncertainty surfaced in UI (`± Y cm`) — `core-math` already supports this; wire it through
- [ ] Benchmark suite with real ground-truth objects, accuracy numbers published per platform/backend

## Phase 6 — Community infrastructure
- [ ] `docs/ADDING_A_BACKEND.md`, `docs/ADDING_A_DEVICE_CLASS.md` with real walkthroughs
- [ ] CI runs benchmark suite on PRs, flags accuracy regressions
- [ ] Good-first-issue templates
- [ ] Public discussion board for backend/hardware requests

## Phase 7 — Ongoing / v1.0+
- [ ] Depth-camera backend (RealSense) as optional high-precision mode
- [ ] PWA + offline model caching for web
- [ ] Wider docs/contributor localization
