# Q-054: Visual baselines

**Status:** authoritative in the [Q project board](https://github.com/users/GuilhermeFortuna/projects/2)  
**Project direction:** [`q_contracts/docs/system-architecture.md` §5.1, §10 Phase 5](https://github.com/GuilhermeFortuna/q_contracts/blob/03214a0760945120c56c2af97a542fcb26da2f1e/docs/system-architecture.md#51-ui-ownership-boundary)  
**UI direction:** [`../../design/ui-direction.md`](../../design/ui-direction.md) §2, §4, §15  
**Reference register:** [`../../design/references.md`](../../design/references.md)  
**Batch:** 09 — terminal UX and visual validation
**Depends on:** Q-053, Q-056, Q-058
**Implementation plan:** [`../plans/Q-054-visual-baselines-plan.md`](../plans/Q-054-visual-baselines-plan.md)

## Purpose

Q-051 makes design a mechanical rule — no literal escapes the token layer — and gives the
terminal a gallery that shows the whole system on one page. Between them they catch a
broken rule and support a human looking at a screen. Neither notices a regression. A panel
edited in Q-052 can silently shift a table's column alignment, a Q-053 layout change can
break a header at one resolution, and the token gate passes every time.

This task closes that gap: the gallery and the real screens render offscreen in a fixed
environment, and their images are compared against committed baselines, so an unintended
change in appearance fails like any other test.

It comes after Q-055 to Q-058 because those tasks change chart identity, chart controls,
the palette, and the merged workspace. Capturing the current screens first would commit
baselines that the redesign immediately replaces. The component gallery and manual
screenshots remain the visual review tools while those tasks are built.

## Requirements

### Baselines

- The Q-051 gallery and the real screens — each composition, each detail table, the header
  in a healthy state and in each §8.1 degraded state, each dialog, the merged single-window
  arrangement, Q-055 chart identity/freshness states, and Q-056 chart picker/mode/error
  states — render offscreen to images in a fixed, reproducible environment.
- Rendering is deterministic: software rasterisation, vendored fonts with system font
  paths excluded, fixed sizes, fixed device pixel ratio, and fixed data from the existing
  fakes rather than anything live. Two runs on the same commit produce identical images.
- Images are compared against committed baselines with a stated tolerance, and the
  comparison fails with a written diff image naming the regions that changed.
- Baselines are captured at both workstation resolutions, so a layout that only breaks on
  the wider monitor is caught.
- Accepting a change is one explicit command that rewrites the baselines and shows what it
  rewrote. Baselines are never regenerated as a side effect of a test run.
- The comparison runs in `make check` and in CI, and CI publishes the diff images of a
  failure.

## Constraints and non-goals

- **No new component, panel, screen or behaviour.** This task renders what exists. The
  gallery itself is Q-051's; here it is only a thing to photograph.
- **No live data in a baseline.** Everything comes from the existing fake server and fake
  stores.
- **No cross-machine pixel equality claim.** The baselines are valid for the pinned
  rendering environment. A developer on a different setup runs the same environment, not a
  different renderer with a looser tolerance.
- **No chart-content baselines.** The scene-graph bar path is covered by its own Q-037 and
  Q-038 tests and by the frame benchmark; a pixel baseline of live candles would be noise.
  The chart panel's chrome, axes and empty states are in scope; its rendered bars are not.
- **No frame-budget impact.** Nothing here runs in a normal session.

## Acceptance criteria

### Agent-verifiable

1. Running the baseline capture twice on the same commit produces byte-identical images.
2. A deliberate one-token change — a spacing value, then a colour token — fails the
   comparison, and the written diff image marks the changed regions. Reverting it passes.
3. Baselines exist for the gallery, every composition, every detail table, the header in a healthy
   state and in each §8.1 degraded state, every dialog, the merged arrangement, chart
   identity/freshness states, and picker/mode/error states at both workstation resolutions.
4. The acceptance command rewrites baselines, reports every file it rewrote, and a normal
   test run never modifies a baseline file, proven by asserting the working tree is clean
   after a failing run.
5. Fonts resolve from the vendored resources with system font directories excluded, and
   the rendering environment is pinned and recorded.
6. Every Q-035 to Q-058 test passes; `make check` passes and includes the comparison.

### Human-verifiable

1. A baseline failure is triggered by a real change made during review, and the diff image
   is confirmed to make the cause obvious without reading the code.
