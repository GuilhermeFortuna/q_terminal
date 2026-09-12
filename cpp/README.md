# C++ Scene-Graph Area

This directory is reserved exclusively for custom Qt Quick scene-graph render nodes (`QSGNode`, `QSGGeometryNode`, `QSGRenderNode`) that require direct graphics context access or buffer movement.

## Permitted Role

- Custom scene-graph nodes and buffer movement only.
- Direct interaction with Qt Quick rendering hardware interface (RHI) / graphics context when QML or standard Qt Quick elements cannot achieve required tape or chart density.

## Explicit Restrictions

- **Zero computation and zero business logic.** Everything computational or deterministic lives in the Rust core (`q_core`) or the Rust terminal application/bridge layer.
- Code in this directory moves buffers into GPU structures; it computes nothing, evaluates nothing, and contains no trading or simulation state machines.
