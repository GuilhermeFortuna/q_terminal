# q_terminal — agent instructions

Read `README.md` and `BOUNDARY.md` before changing anything. `BOUNDARY.md` is binding:
this repository is the live trading and operations surface, it owns no backend process,
and it never grows a strategy editor, an optimizer or a result browser.

## Build and check

- `env -u WAYLAND_DISPLAY -u DISPLAY make check` is the gate. It runs formatting, clippy,
  the build, `qmllint`, the tests and `make contracts-check`. Unset both display variables:
  the suites run offscreen, and a Wayland session otherwise changes what they do.
- `make run` launches the terminal against an already-running stack. Start the stack from
  the workspace root with `./dev up execution`, never from here.
- `make bench-frames` measures frame time; `BENCH_EXECUTION_ROWS=N` sets the store size.
  The budget is p95 under 16 ms.
- Never hand-edit `contracts/`. It is vendored from the commit in `CONTRACTS_REV`.

## Visual work

Visual and design work is reference-driven: adapt a named entry from
`docs/design/references.md`, never design from a blank canvas. `docs/design/ui-direction.md`
is the direction of record; `docs/design/planning-answers.md` records the decisions taken.

**Look at what you build.** From Q-051 onward the component gallery renders itself to
image files:

```bash
env -u WAYLAND_DISPLAY -u DISPLAY make gallery-shot   # writes PNGs, no display needed
```

Then open the images with your own image tool — `Read` in Claude Code, `view_image` in
codex — and judge the result before committing. Do this per component as you build it, not
once at the end. A screen you have not looked at is a screen you have not finished.

`docs/design/components.md` records, per component, the reference adapted, what was taken
and what was deliberately changed. Fill it in as you go.

## Design rules

- `qml/theme/` is the only place a colour, a type size or a pixel dimension may be
  written as a literal. `tools/token_gate.py` enforces this and runs in `make check`.
- QML decides where things are, how they are arranged and how they look. It never decides
  what a value means. Semantic roles come from Rust; `qml/theme/Semantic.qml` is the only
  file that maps a role to a token.
- Controls are Qt Quick Controls restyled, so keyboard navigation, focus and interaction
  states come from the substrate. Build fully custom only for the trading-specific
  widgets — ladder, tape, footprint — where there is nothing to restyle.
