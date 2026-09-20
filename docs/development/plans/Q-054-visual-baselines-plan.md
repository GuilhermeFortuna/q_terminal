# Q-054 implementation plan: Visual baselines

**Status:** authoritative in the [Q project board](https://github.com/users/GuilhermeFortuna/projects/2)  
**Specification:** [`../specs/Q-054-visual-baselines-spec.md`](../specs/Q-054-visual-baselines-spec.md)  
**Depends on:** Q-053

## Current-system context

After Q-051 the design system exists, the gallery renders every component in every state
behind the `gallery` cargo feature, the fonts are vendored in the `qrc`, and
`tools/token_gate.py` enforces the rules statically. After Q-052 the shell composes
panels into windows; after Q-053 workspaces restore them. Appearance itself is checked only by a
human opening the gallery or comparing screenshots attached to a handoff; nothing notices
a change.

The existing headless harnesses (`tests/degraded_states.rs`, `tests/slice_end_to_end.rs`,
`tests/ops_views.rs`, and the Q-052 and Q-053 suites) already drive the application
offscreen against a fake server with fixed data, which is most of what a baseline run
needs. `make check` runs with `env -u WAYLAND_DISPLAY -u DISPLAY`. `make bench-frames`
shows the project's precedent for a measured, recorded artefact in CI.

## Interfaces produced

```
src/visual/capture.rs            new: offscreen render to PNG at fixed size and DPR
src/visual/compare.rs            new: tolerance comparison + diff image writer
tests/visual_baselines.rs        new: criteria 1–5
tests/baselines/<name>@<w>x<h>.png   committed baselines
tools/visual.py                  new: `capture`, `check`, `accept` entry points
docs/design/visual-testing.md    new: the pinned environment, how to accept a change
Makefile                         new targets: visual-check (in check), visual-accept
```

## Implementation decisions

- **Qt's own approach, scoped down.** Qt tests QtQuick rendering with baseline images, and
  the failure mode is well known: GPU drivers, font rendering and scaling shift pixels
  between machines, which is why Qt's system keeps per-platform baselines behind a server.
  The terminal does not need that. It needs one pinned environment — software
  rasterisation (`QT_QUICK_BACKEND=software`), the offscreen platform, the vendored fonts
  with system font paths excluded, fixed window sizes and a fixed device pixel ratio — in
  which two runs are byte-identical. The tolerance then exists for genuinely
  imperceptible differences, not to paper over a wrong environment. If byte-identity does
  not hold in step 2, the environment is wrong and gets fixed there, rather than the
  tolerance being widened.

- **The gallery is captured, not built.** Q-051 owns it, along with the test that fails
  when a component has no entry in it. Here it is simply the most valuable single image to
  baseline, because it puts every component and state in one frame: one diff covers the
  whole system.

- **Baselines are captured from the existing fakes.** The screens already render
  headlessly against the fake server with fixed data in the Q-047, Q-052 and Q-053 suites.
  The capture reuses those harnesses, so a baseline and a behavioural test describe the
  same state, and there is no second source of fixture data to drift.

- **Both resolutions, because the workstation has both.** 1920×1080 and 2560×1440 are the
  real monitors. A header that only breaks at the wider width is exactly the regression
  this task exists to catch.

- **Accepting is explicit and shows its work.** `make visual-accept` rewrites baselines and
  prints every file it rewrote; a normal run never writes one, which criterion 5 asserts
  by checking the working tree is clean after a failure. A test run that quietly updates
  its own expectations is worse than no test.

- **The chart's bars are out of scope.** Q-037 and Q-038 already cover the render node,
  and the frame benchmark covers its cost. Pixel-diffing live candles would produce noise
  and teach the team to ignore the check.

## Ordered implementation

- [ ] 1. Work on the branch `Q-054-visual-baselines` in `q_terminal`, created from
   `development` by `./work start`. Confirm Q-053 is merged. Commit.
- [ ] 2. Pin the rendering environment and prove byte-identity across two runs of one
   screen before anything else is built (criterion 1, and criterion 5's font check). If it
   does not hold, fix the environment here. Record it in `docs/design/visual-testing.md`.
   Commit.
- [ ] 3. Implement `src/visual/capture.rs` and `compare.rs` with the diff image writer, and
   `tools/visual.py` with `capture`, `check` and `accept`. Commit.
- [ ] 4. Capture baselines for the Q-051 gallery and every screen in criterion 3 at both
   resolutions, from the existing fake harnesses. Commit the baselines.
- [ ] 5. Wire `visual-check` into `make check` and CI, publishing diff images on failure.
   Prove criteria 2 and 4 with the deliberate token change and the clean-tree assertion.
   Commit.
- [ ] 6. Write `docs/design/visual-testing.md` and update `README.md` and `BOUNDARY.md`.
   Commit.
- [ ] 7. Run `env -u WAYLAND_DISPLAY -u DISPLAY make check`. Fix, re-run, commit.
- [ ] 8. **Human:** the human-verifiable criterion.

## Validation

- **Determinism:** two capture runs on one commit, byte-compared.
- **Detection:** a deliberate spacing change and a deliberate colour-token change, each
  failing with a legible diff, each passing after revert.
- **Coverage:** the gallery and every screen in criterion 3 have a baseline at both
  resolutions.
- **Hygiene:** the working tree is clean after a failing run; `visual-accept` reports every
  file it rewrote.
- **Regression:** every Q-035 to Q-053 test, the token gate, `qmllint`,
  `make contracts-check`.

```bash
cd /home/gui/projects/q/q_terminal
env -u WAYLAND_DISPLAY -u DISPLAY make check
make visual-check
make visual-accept        # explicit, never automatic
```

## Handoff

Give the pinned environment exactly as recorded, and the byte-identity result. Give the
number of baselines by screen and resolution, and the total size added to the repository.
Give the two deliberate-change results with their diff images.
