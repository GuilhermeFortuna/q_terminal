//! Display identity, placement intent and compositor rule export (Q-053).

pub mod compositor;
pub mod identity;
pub mod placement;

pub use compositor::{export_hyprland_rules, CompositorWindow};
pub use identity::{
    DisplayDescription, ResolutionReport, ResolutionRule, ScreenFingerprint, ScreenGeometry,
};
pub use placement::{GeometryIntent, PlacementMode, PlacementReport, PlacementStrategy};
