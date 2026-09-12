# Q-008: `q_terminal` application skeleton

**Status:** authoritative in [`../STATUS.md`](../STATUS.md)  
**Project direction:** [`../../system-architecture.md`](../../system-architecture.md)  
**Depends on:** Q-007  
**Implementation plan:** [`../plans/Q-008-q-terminal-application-skeleton-plan.md`](../plans/Q-008-q-terminal-application-skeleton-plan.md)

## Purpose

The live trading and operations surface is moving out of the web stack because
WebKitGTK cannot give it a GPU path or a dense real-time tape. That move rests on
a chain nobody has yet run on this machine: a Qt application, linking a Rust
workspace through a C++ bridge, on Wayland with a proprietary driver. Every part
of that chain is individually plausible and the combination is where projects
discover a two-week problem. This task builds the smallest application that
exercises the whole chain — window, QML scene, bridge, core — and establishes the
repository's structure, validation, and the boundary rule that keeps it from
becoming a second research UI. It carries no product behavior on purpose: the
first vertical slice with real data is the next task, and it should not also be
the task that discovers the toolchain.

## Requirements

### The chain works

- A launched application shows a window with a QML scene, on the target desktop
  platform, driven by the target driver stack.
- The scene displays a value that could only have come from the Rust workspace
  through the bridge, so the integration is demonstrated rather than assumed.
- The application exits cleanly, releasing the graphics context, so that a
  restart during development is not a reboot.
- The rendering backend actually in use at runtime is reported by the
  application, because the difference between hardware and software rendering is
  invisible in a screenshot and decides whether any later performance number
  means anything.

### Repository structure

- The repository separates QML, the Rust bridge and application code, and the
  C++ scene-graph area, and states which kinds of code belong in each.
- The C++ area exists and is empty of logic, with its permitted role stated: it
  moves buffers for custom scene-graph nodes and computes nothing.
- The application owns no backend process: it launches nothing, supervises
  nothing, and stops nothing, and this is stated as a rule in the repository
  rather than being merely true today.
- The repository declares the ownership boundary — live trading and operations
  only — and names what it will never grow.

### Dependencies

- The workspace providing computational semantics is consumed as a pinned
  dependency, by tag, with no path into a sibling checkout.
- Generated contract types are carried under the same vendoring protocol every
  other consumer uses, with the contracts commit recorded and checked.
- The pinned versions are the ones recorded as known-good, and the record is
  updated to include this repository.

### Validation without a display

- The repository has a validation command that runs to completion in an
  environment with no display, so that CI and an agent session can both run it.
- QML is checked for correctness — syntax and binding errors — as part of that
  command, because a QML error appears at runtime as a blank area and not as a
  build failure.
- The parts of the application that can be tested without a window are tested
  without one, and what cannot be is named explicitly as requiring a human.

### Prerequisites are real

- The system packages and toolchain versions required to build and run are
  documented, and the documentation is verified against an environment that does
  not already have them.
- The application's own build does not require Python, and building the wheel
  from the core workspace is not a prerequisite for building the terminal.

## Constraints and non-goals

- **No market data.** No stream client, no WebSocket, no REST call, no quotes, no
  bars. The vertical slice with one live symbol is the next task and is where the
  data path is proven; doing it here would mean proving the toolchain and the
  transport in one change and not knowing which one broke.
- **No custom scene-graph node.** The C++ area is created empty. A render node
  written before there is a buffer to render is a render node written against a
  guess.
- **No controls.** No start, stop, flatten, or kill switch. Those are commands
  with consequences and they arrive with the execution surface.
- **No direct Parquet reads.** Catalog-driven reading needs a catalog, which does
  not exist yet.
- **No design system.** Colors, typography, and layout are not decided here. A
  skeleton that also carries a visual language will have both reworked when the
  first real surface arrives.
- **No removal of the frontend's execution workspace.** It is removed when the
  terminal has a working replacement, which is not this task.
- **No packaging or distribution.** The application runs from a build directory.

## Acceptance criteria

### Agent-verifiable

1. The repository exists, builds from a clean checkout with a single command, and
   contains no path reference to a sibling checkout other than through pinned
   dependencies.
2. The core workspace is consumed by tag, and the tag matches the one recorded as
   known-good.
3. Generated contract types are carried, the contracts commit is recorded, and a
   clean regeneration produces no diff.
4. The QML check passes over every QML file and fails when a deliberate binding
   error is introduced, verified by introducing one, observing the failure, and
   reverting it.
5. The application binary builds and, when run headless, reports its version, the
   core version reached through the bridge, and the contracts commit, then exits
   zero without opening a window.
6. The value reported through the bridge matches the core workspace's own version
   at the pinned tag.
7. The C++ area exists, contains no logic, and its permitted role is stated in the
   repository.
8. The repository states the no-backend-process-ownership rule and the ownership
   boundary.
9. The validation command runs to completion with no display available.
10. The build does not require Python, verified by building in an environment
    without a Python development toolchain.
11. The known-good pin record includes this repository.
12. The full validation suite passes.

### Human-verifiable

1. The application launches on the target desktop and shows a window containing
   the value obtained through the bridge.
   Command: `cd q_terminal && make run`
2. The application reports hardware rendering, and the reported backend is
   confirmed against an independent measure of GPU use.
   Command: `make run` then observe the reported backend and `nvidia-smi` while it runs
3. The application is closed and relaunched five times in succession with no
   failure, no leaked process, and no degraded rendering.
   Command: `for i in $(seq 5); do make run; done`
4. The documented prerequisites are verified against an environment that lacks
   them, confirming no step succeeds because of something already installed.
   Command: `podman run --rm -it -v $PWD:/src <clean image> sh -c 'cd /src && make check'`
