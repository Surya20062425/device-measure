# Contributing

## Where to start

- **New to the project?** Read [../README.md](../README.md) and
  [ARCHITECTURE.md](ARCHITECTURE.md) first — the architecture doc explains
  *why* the system is built this way, which will save you from re-proposing
  things already deliberately ruled out (e.g. "why not just use an LLM for
  distance" — answered there).
- **Check [../ROADMAP.md](../ROADMAP.md)** for the current phase. Contributions
  matching the current or next phase are easiest to get merged.

## Areas that don't require deep Rust/CV knowledge

- Adding training images for a device class → [ADDING_A_DEVICE_CLASS.md](ADDING_A_DEVICE_CLASS.md)
- Improving docs, calibration guides
- Testing the app on hardware we don't have access to and reporting accuracy

## Working on `core-math` (Rust)

```
cd core-math
cargo test          # must pass before opening a PR
cargo build --release
```

Every function that estimates a physical quantity (distance, diameter) must
return a `Measurement { value, std_dev }`, not a bare number — see
`src/uncertainty.rs`. If you add a new distance or diameter method, add:
1. A hand-calculated unit test (verify the formula against a value you
   computed by hand, not just "it runs").
2. An uncertainty-propagation test (verify std_dev grows with input noise).
3. A doc comment stating the method's accuracy ceiling and what it assumes,
   matching the style in `distance.rs`.

## Adding a new distance/detection backend

See [ADDING_A_BACKEND.md](ADDING_A_BACKEND.md) (coming in Phase 6 — for now,
open an issue describing the backend and hardware it targets).

## Code of conduct

Be respectful, assume good faith, keep technical disagreements technical.
