# Q-054: Component gallery and visual baselines

**Status:** authoritative in the [Q project board](https://github.com/users/GuilhermeFortuna/projects/2)  
**Project direction:** [`q_contracts/docs/system-architecture.md` §5.1, §10 Phase 5](https://github.com/GuilhermeFortuna/q_contracts/blob/03214a0760945120c56c2af97a542fcb26da2f1e/docs/system-architecture.md#51-ui-ownership-boundary)  
**UI direction:** [`../../design/ui-direction.md`](../../design/ui-direction.md) §2, §4, §15  
**Reference register:** [`../../design/references.md`](../../design/references.md)  
**Depends on:** Q-053  
**Implementation plan:** [`../plans/Q-054-visual-baselines-plan.md`](../plans/Q-054-visual-baselines-plan.md)

## Purpose

Q-051 makes design a mechanical rule — no literal escapes the token layer. That catches
the rules and nothing else. A panel edited in Q-052 can silently shift a table's column
alignment, a Q-053 layout change can break a header at one resolution, and the token gate
will pass every time. Appearance is currently checked by a human looking at screenshots
attached to a handoff, which does not scale past the handful of screens the terminal has
today and will not survive the tape, DOM and footprint surfaces phase 5 adds.

This task adds the two things that make design durable: a gallery that renders every
component in every state as the living counterpart to the reference register, and
baselines that render the gallery and the real screens offscreen and fail when their
appearance changes without anyone intending it.

It comes after the shell and the workspaces because a baseline captured before those
layouts settle is a baseline that gets thrown away.

## Requirements

### Gallery

- A gallery surface renders every component of the design system in every declared
  interaction state, every semantic role in every treatment, the full type scale, the
  colour tokens and the icon set.
- The gallery is reachable from the terminal in a development build and absent from a
  normal run. It is not an operations surface and carries no live data.
- Each entry names the register entry it adapts, so the gallery and
  `docs/design/components.md` can be read side by side against the reference.
- A component that exists without a gallery entry is a failure, not an omission.

### Baselines

- The gallery and the real screens — each composition, each detail table, the header in a
  healthy state and in each §8.1 degraded state, each dialog, the merged single-window
  arrangement — render offscreen to images in a fixed, reproducible environment.
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

- **No new component, panel, screen or behaviour.** This task renders what exists.
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

1. The gallery renders every component in every declared state, and a test fails if a
   component in the design system has no gallery entry, or a declared state has no entry.
2. Running the baseline capture twice on the same commit produces byte-identical images.
3. A deliberate one-token change — a spacing value, then a colour token — fails the
   comparison, and the written diff image marks the changed regions. Reverting it passes.
4. Baselines exist for every composition, every detail table, the header in a healthy
   state and in each §8.1 degraded state, every dialog and the merged arrangement, at both
   workstation resolutions.
5. The acceptance command rewrites baselines, reports every file it rewrote, and a normal
   test run never modifies a baseline file, proven by asserting the working tree is clean
   after a failing run.
6. The gallery is absent from a normal run and present in a development build, asserted
   both ways.
7. Fonts resolve from the vendored resources with system font directories excluded, and
   the rendering environment is pinned and recorded.
8. Every Q-035 to Q-053 test passes; `make check` passes and includes the comparison.

### Human-verifiable

1. The gallery is reviewed against the reference register entry by entry, and any
   divergence from the adapted reference is either fixed or recorded in
   `docs/design/components.md` as deliberate.
2. A baseline failure is triggered by a real change made during review, and the diff image
   is confirmed to make the cause obvious without reading the code.
