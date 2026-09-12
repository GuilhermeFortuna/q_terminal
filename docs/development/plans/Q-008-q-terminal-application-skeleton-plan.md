# Q-008 implementation plan: `q_terminal` application skeleton

**Status:** authoritative in [`../STATUS.md`](../STATUS.md)  
**Specification:** [`../specs/Q-008-q-terminal-application-skeleton-spec.md`](../specs/Q-008-q-terminal-application-skeleton-spec.md)  
**Depends on:** Q-007

## Current-system context

The live-trading surface today is `q_frontend`'s execution workspace: React and
`visx` inside Tauri 2 on WebKitGTK, kept current by roughly eleven React Query
polling timers at one to two seconds. The same Tauri shell also spawns
`docker compose` and `uv` from its Rust side, reaching into `../../q_backend` —
the coupling §7.1 removes and the reason §8 forbids a UI process from owning a
backend process. None of that is touched here.

After Q-007, `q_core` is a six-crate workspace at a `vYYYY.MM.DD` tag with
`q-qt` exposing a `CoreInfo` QObject carrying `version` and `contracts_rev` as
properties, built and linked by `cxx-qt` without any Python present, and proven by
a C++ harness that instantiates `CoreInfo` and asserts both properties. `q_core`
carries `contracts/` and `CONTRACTS_REV` under the vendoring protocol Q-006
defined, and `COMPAT.md` lists three repositories plus `q_core`. The gap this task
closes is that `CoreInfo` has been instantiated only by a test harness: no Qt
application has been built, no QML scene has loaded, and nothing has been rendered
on this machine's Wayland/NVIDIA stack. Every later terminal task assumes that
chain works.

## Interfaces produced

```
// q_terminal/  (new repository)
Cargo.toml              binary crate; cxx-qt-build in build.rs
rust-toolchain.toml     matching q_core's pin
src/
  main.rs               entry point; --headless-report path and the windowed path
  bridge.rs             cxx-qt bridge: AppInfo QObject, projection only
qml/
  Main.qml              the single scene
  qmldir
cpp/
  README.md             permitted role: scene-graph render nodes only, no logic
contracts/              vendored generated Rust
CONTRACTS_REV
Makefile                `make check`, `make run`, `make contracts`, `make contracts-check`
README.md               structure, ownership boundary, prerequisites
BOUNDARY.md             what this repository is for and what it never grows
```

```rust
// src/bridge.rs — projection only; no computation
#[cxx_qt::bridge]
mod ffi {
    extern "RustQt" {
        #[qobject]
        #[qproperty(QString, app_version)]      // this repository's version
        #[qproperty(QString, core_version)]     // from q_core::CoreInfo
        #[qproperty(QString, contracts_rev)]    // from CONTRACTS_REV
        #[qproperty(QString, render_backend)]   // QSGRendererInterface graphics API in use
        type AppInfo = super::AppInfoRust;
    }
}
```

```rust
// src/main.rs
/// Prints app version, core version, contracts rev and render backend, then exits 0.
/// Opens no window; the path CI and an agent session take.
fn headless_report() -> i32;

/// Loads qml/Main.qml into a QQmlApplicationEngine and runs the event loop.
fn run_windowed() -> i32;
```

```
// Makefile targets
check           fmt, clippy, qmllint over qml/, cargo test, cargo build, headless-report
run             cargo run                       (requires a display; not part of check)
contracts       regenerate contracts/ from CONTRACTS_REV
contracts-check regenerate into a temp dir and diff
```

## Implementation decisions

- **`q_core` is a Cargo git dependency pinned by tag, never a path dependency.**
  A path dependency would make the terminal build whatever is checked out next
  door, including uncommitted work, and would put a sibling path into a committed
  manifest — the exact coupling that already exists between the Tauri shell and
  `q_backend` and that §7.1 exists to remove. The cost is that a `q_core` change
  requires a tag before the terminal can use it, which is the intended friction.

- **The application has a headless report path in the binary itself, not a
  separate test binary.** The value being proven is that the real application
  reaches the real bridge; a separate harness proves that a harness can. Making it
  a flag on the shipped binary means acceptance criterion 5 exercises the same
  code path that criterion 1's window will.

- **`render_backend` is read from Qt's scene-graph renderer interface at runtime
  and reported, rather than inferred from environment variables.** On Wayland with
  the proprietary NVIDIA driver, a failed hardware path falls back to software
  rendering silently and the window looks correct; every frame-time number
  measured afterwards would then be measured on the wrong stack. Reporting what Qt
  actually chose is the only way a later 16 ms budget means anything, and it costs
  one call.

- **QML is checked by `qmllint` inside `make check`, not only compiled.** A QML
  binding error is not a build failure — the application starts and the affected
  item is simply absent. Without the lint, criterion 4's failure mode reaches a
  human as "the number is missing" rather than as a build error naming the line.

- **The scene is a single `Main.qml` with no component library, no theme file, and
  no layout system.** The spec rules out a design system, and the way that rule
  gets broken is by adding "just a colors file" that the next surface then builds
  on. One file with default styling cannot become a design system by accident.

- **`AppInfo` projects `q_core`'s `CoreInfo` rather than re-reading the same
  sources.** Reading `CONTRACTS_REV` and a version from this repository's own
  files would produce plausible values without the bridge working at all, which
  would make criterion 6 pass on a broken build. Routing `core_version` through
  `CoreInfo` means the value can only exist if the link, the bridge, and the
  property system all work.

- **`cpp/` is created with a README and nothing else.** The directory must exist so
  that the first render node has an obvious home and so that the permitted role is
  written down before anyone needs it; it must be empty so that nobody reads a
  sample file as precedent for putting logic there.

- **`BOUNDARY.md` is a separate document from `README.md`.** The rule it carries —
  live trading and operations only, never a strategy editor, an optimizer, or a
  result browser — is the rule most likely to be violated by a well-intentioned
  change, and a rule buried in a README's fourth section is a rule nobody is
  pointed at in review. A standalone file can be linked from a pull request
  template later.

- **`make run` is deliberately not part of `make check`.** `check` must pass with
  no display, in CI and in an agent session; a target that opens a window would
  make the common case fail for an environmental reason and the check would be
  routed around.

- **Prerequisites are verified in a bare container by a human, not by CI.** A CI
  image is chosen to contain the prerequisites, so it can never falsify a claim
  that the list is complete. The container run is the only check that actually
  tests the documentation.

- **The relaunch loop in human criterion 3 is five iterations, not one.** The
  failure this is looking for — a graphics context not released, a compositor
  resource leaked, a zombie process — does not appear on the first exit. Five is
  cheap and is enough to expose the common form; a soak test belongs with the
  first surface that actually renders continuously.

## Ordered implementation

1. Create the branch `Q-008-q-terminal-application-skeleton-spec` in a newly
   initialized `q_terminal` repository at `/home/gui/projects/q/q_terminal`.
2. Add `Cargo.toml` declaring a binary crate with `cxx-qt`, `cxx-qt-lib`, and a git
   dependency on `q_core` pinned to the tag recorded in `COMPAT.md`; add
   `rust-toolchain.toml` matching `q_core`'s pin, `build.rs` invoking
   `cxx-qt-build`, and `.gitignore`. Confirm `cargo metadata` resolves the pinned
   tag. Commit.
3. Add `CONTRACTS_REV` with the hash from `COMPAT.md` and the `contracts` and
   `contracts-check` Makefile targets, identical in behavior to the other
   consumers'. Run `make contracts`. Confirm `make contracts-check` passes. Commit.
4. Write a failing test asserting that `AppInfoRust`'s `core_version` equals the
   version `q_core`'s `CoreInfo` reports and that it is non-empty. Confirm it
   fails to compile before `bridge.rs` exists. Implement `src/bridge.rs` with the
   four properties, `core_version` and `contracts_rev` read through `CoreInfo`.
   Confirm the test passes. Commit.
5. Implement `headless_report` in `src/main.rs` behind a `--headless-report` flag,
   printing the four values one per line and returning 0. Write a failing
   integration test that runs the built binary with the flag and asserts the
   core-version line equals the tag's workspace version and that the exit status is
   0. Confirm it fails, implement, confirm it passes. Commit.
6. Add `qml/Main.qml` displaying the four values in a plain column, and `qml/qmldir`.
   Implement `run_windowed` loading it into a `QQmlApplicationEngine`. Add
   `render_backend` population from the scene-graph renderer interface once the
   window is exposed. Commit.
7. Add `qmllint` to `make check` over `qml/`. Verify criterion 4: introduce a
   binding to an undefined property in `Main.qml`, run `make check`, confirm it
   fails naming the file and line, and revert. Do not commit the broken binding.
8. Write `cpp/README.md` stating the permitted role and that nothing in the
   directory may compute. Write `BOUNDARY.md` stating the live-trading-and-
   operations scope, the list of surfaces this repository never grows, and the
   no-backend-process-ownership rule. Write `README.md` covering structure,
   prerequisites with exact package names and versions, and the two ways to run.
   Commit.
9. Assemble `make check` as fmt, clippy, `qmllint`, `cargo test`, `cargo build`,
   the headless report, and `make contracts-check`. Confirm it passes with
   `WAYLAND_DISPLAY` and `DISPLAY` both unset. Commit.
10. Verify criterion 10: build in a container without a Python development
    toolchain and confirm success. Record the command and outcome.
11. Add `.github/workflows/ci.yml` running `make check`. Commit.
12. Update `q_contracts/COMPAT.md` to add `q_terminal` with its commit, its
    `q_core` tag, and its contracts hash, and extend the "verified by" line.
13. Human step, matching human-verifiable criterion 1: run `make run` on the target
    desktop and confirm a window appears showing the four values, with
    `core_version` matching the pinned tag.
14. Human step, matching human-verifiable criterion 2: read the reported
    `render_backend` and confirm hardware rendering against `nvidia-smi` showing
    the process holding GPU memory while the window is open.
15. Human step, matching human-verifiable criterion 3: launch and close the
    application five times in succession, confirming a clean exit each time, no
    surviving process, and no visual degradation.
16. Human step, matching human-verifiable criterion 4: run `make check` inside a
    bare container and confirm the documented prerequisites are complete.
17. Run the full validation suite and commit. Report the handoff.

## Validation

- **Unit:** `AppInfoRust`'s `core_version` matches `CoreInfo`'s and is non-empty.
- **Integration:** the built binary's `--headless-report` output, compared line by
  line against the pinned `q_core` tag's workspace version and the content of
  `CONTRACTS_REV`. This is the test that proves the whole chain short of the
  window.
- **Regression:** `make contracts-check` and the pinned `q_core` tag are the
  standing checks that this repository does not drift off the known-good set
  recorded in `COMPAT.md`.
- **Manual:** steps 13 through 16, plus the negative verification in step 7.
- **Measurement:** report the cold `cargo build` wall-clock time from a clean
  checkout, because `cxx-qt` code generation plus a Qt link is the slowest step in
  the project and every later terminal task pays it.

```bash
cd /home/gui/projects/q/q_terminal
make contracts-check
env -u WAYLAND_DISPLAY -u DISPLAY make check     # criterion 9, must pass headless
cargo run -- --headless-report                   # criteria 5 and 6

# criterion 4, the deliberate QML error
sed -i 's/app_version/app_versionn/' qml/Main.qml
make check; echo "expected non-zero, got $?"
git checkout -- qml/Main.qml

# human criteria 1-3
make run
```

## Handoff

Report the four lines of `--headless-report` output verbatim, alongside the
`q_core` workspace version at the pinned tag and the content of `CONTRACTS_REV`,
so the bridge proof is shown rather than asserted. Report the `q_core` tag and the
contracts hash this repository pins, and confirm both match `COMPAT.md`. Report
the `render_backend` string the running application reported and the `nvidia-smi`
evidence that accompanied it — if it reports software rendering, say so plainly
and stop, because every later frame-time budget in this repository depends on
that answer and a software fallback is a finding, not a detail. Report the outcome
of five launch-and-close cycles, including whether any process survived. Report
the exact `qmllint` failure produced by the deliberate binding error. Report the
cold build time from a clean checkout. Report the result of the bare-container run
and any prerequisite that was missing from `README.md` and had to be added.
