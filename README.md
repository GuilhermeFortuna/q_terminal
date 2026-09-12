# `q_terminal`

High-performance desktop trading and operations terminal for the Q platform, built on Qt 6 / QML and backed by Rust (`q_core`) via CXX-Qt.

---

## Repository Structure

```
q_terminal/
├── Cargo.toml          # Package manifest; links cxx-qt and git-pinned q_core tag
├── rust-toolchain.toml # Exact Rust compiler pin (1.98.0)
├── build.rs            # CXX-Qt build script compiling bridge and registering QML module
├── Makefile            # Standing validation targets: check, fmt, lint, test, build, run
├── CONTRACTS_REV       # Pinned q_contracts commit hash
├── BOUNDARY.md         # Explicit ownership scope and prohibited surfaces
├── contracts/          # Vendored generated Rust wire types from q_contracts
├── cpp/                # C++ scene-graph render nodes (buffer movement only; empty of logic)
│   └── README.md
├── qml/                # Declarative QML scenes
│   ├── Main.qml        # Single application window scene
│   └── qmldir         # QML module definition
└── src/                # Rust application and bridge code
    ├── main.rs         # Entry point: --headless-report and windowed event loop
    ├── bridge.rs       # CXX-Qt AppInfo projection over q_core::CoreInfo
    ├── render_backend.h
    └── render_backend.cpp
```

### Separation of Concerns

- **`qml/`**: User interface scenes and declarative presentation layout.
- **`src/`**: Application entry point, CLI arguments, and the CXX-Qt projection bridge (`AppInfo`). No business or simulation logic is implemented here; values project directly from `q_core`.
- **`cpp/`**: Reserved exclusively for custom scene-graph render nodes (`QSGRenderNode`) to move buffers into GPU memory. Computes nothing.
- **`contracts/`**: Vendored contract types generated from `q_contracts`.

---

## Architectural Boundary

`q_terminal` is dedicated strictly to **live trading and operations**. It never grows research surfaces, strategy editors, backtesters, or parameter optimizers.

Furthermore, `q_terminal` **owns no backend process**: it launches nothing, supervises nothing, and stops nothing. See [`BOUNDARY.md`](BOUNDARY.md) for details.

---

## Prerequisites

Building and running `q_terminal` requires:

1. **Rust Toolchain**:
   - `rustc 1.98.0` (pinned in `rust-toolchain.toml`)
   - Components: `rustfmt`, `clippy`

2. **C++ Compiler & Build Tools**:
   - `g++` with C++17 support
   - `make`, `git`, `curl`
   - Debian/Ubuntu packages: `build-essential`, `lld`

3. **Qt 6 Libraries & Tooling**:
   - On Debian/Ubuntu:
     - `qt6-base-dev`
     - `qt6-declarative-dev`
     - `qt6-declarative-dev-tools` (provides `qmllint`)
     - `libgl1-mesa-dev` (or proprietary NVIDIA OpenGL driver)
   - Runtime dependencies: `libglib2.0-0t64` (or `libglib2.0-0`), `libdouble-conversion3`, `libpcre2-16-0`.
   - If a system Qt 6 SDK is not present, `qt-build-utils` / `cxx-qt-build` automatically downloads a minimal Qt 6 runtime (`qt_minimal`).

4. **Python**:
   - **Not required.** Building and running `q_terminal` does not require Python or Python development headers. `make contracts-check` uses `python3` or `uv` if present, but the core terminal build and runtime are strictly Rust and C++/Qt.

---

## Running the Application

### 1. Headless Report Mode (CI & Headless Verification)

Prints application version, core version, vendored contracts revision, and render backend to stdout, then exits 0 without creating or opening any window:

```bash
cargo run -- --headless-report
```

### 2. Windowed Desktop Mode

Launches the application window displaying the live QML scene:

```bash
make run
# or: cargo run
```

---

## Validation Suite

The standing validation command runs all checks headless without requiring a display:

```bash
env -u WAYLAND_DISPLAY -u DISPLAY make check
```

This runs:
1. `fmt-check`: Rust formatting verification (`cargo fmt --all --check`).
2. `lint`: Clippy lints denying warnings (`cargo clippy --all-targets -- -D warnings`).
3. `build`: Compiles the binary and generated Qt artifacts.
4. `qml-lint`: Validates QML syntax and property bindings (`qmllint -W 0`).
5. `test`: Unit and integration test suite (`cargo test`).
6. `contracts-check`: Verifies vendored `contracts/` match clean regeneration against `CONTRACTS_REV`.
7. `headless-report`: Executes `cargo run -- --headless-report`.
