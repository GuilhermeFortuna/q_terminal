//! Window and panel arrangement as pure structure (Q-052). No Qt here: the shell controller
//! exposes this to QML, and Q-053 serialises it. A window is a tree of splits whose leaves
//! are tab groups of panel ids; a panel lives in exactly one group of exactly one window.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Orientation {
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Node {
    Split {
        orientation: Orientation,
        children: Vec<Node>,
        /// Relative pane sizes, one per child.
        weights: Vec<f64>,
    },
    Tabs {
        panels: Vec<String>,
        active: usize,
    },
}

impl Node {
    pub fn tabs(panels: &[&str]) -> Self {
        Node::Tabs {
            panels: panels.iter().map(|p| p.to_string()).collect(),
            active: 0,
        }
    }

    pub fn split(orientation: Orientation, weights: &[f64], children: Vec<Node>) -> Self {
        debug_assert_eq!(weights.len(), children.len());
        Node::Split {
            orientation,
            children,
            weights: weights.to_vec(),
        }
    }

    pub fn panels(&self) -> Vec<&str> {
        match self {
            Node::Tabs { panels, .. } => panels.iter().map(String::as_str).collect(),
            Node::Split { children, .. } => children.iter().flat_map(Node::panels).collect(),
        }
    }

    fn contains(&self, panel: &str) -> bool {
        self.panels().contains(&panel)
    }

    /// Removes `panel` from the tree; returns whether it was there.
    fn remove(&mut self, panel: &str) -> bool {
        let found = match self {
            Node::Tabs { panels, active } => match panels.iter().position(|p| p == panel) {
                Some(i) => {
                    panels.remove(i);
                    if *active >= panels.len() {
                        *active = panels.len().saturating_sub(1);
                    } else if i < *active {
                        *active -= 1;
                    }
                    true
                }
                None => false,
            },
            Node::Split { children, .. } => children.iter_mut().any(|c| c.remove(panel)),
        };
        if found {
            self.prune();
        }
        found
    }

    /// Drops empty groups and collapses one-child splits.
    fn prune(&mut self) {
        if let Node::Split {
            children, weights, ..
        } = self
        {
            let mut i = 0;
            while i < children.len() {
                children[i].prune();
                if children[i].is_empty() {
                    children.remove(i);
                    weights.remove(i);
                } else {
                    i += 1;
                }
            }
            if children.len() == 1 {
                *self = children.remove(0);
            }
        }
    }

    pub fn is_empty(&self) -> bool {
        match self {
            Node::Tabs { panels, .. } => panels.is_empty(),
            Node::Split { children, .. } => children.iter().all(Node::is_empty),
        }
    }

    fn first_tabs_mut(&mut self) -> &mut Node {
        match self {
            Node::Tabs { .. } => self,
            Node::Split { children, .. } => children[0].first_tabs_mut(),
        }
    }

    fn set_active(&mut self, panel: &str) -> bool {
        match self {
            Node::Tabs { panels, active } => match panels.iter().position(|p| p == panel) {
                Some(i) => {
                    *active = i;
                    true
                }
                None => false,
            },
            Node::Split { children, .. } => children.iter_mut().any(|c| c.set_active(panel)),
        }
    }

    fn at_path_mut(&mut self, path: &[usize]) -> Option<&mut Node> {
        match path.split_first() {
            None => Some(self),
            Some((&i, rest)) => match self {
                Node::Split { children, .. } => children.get_mut(i)?.at_path_mut(rest),
                Node::Tabs { .. } => None,
            },
        }
    }

    /// Keeps only panels for which `keep` holds, in the same arrangement.
    fn restricted(&self, keep: &dyn Fn(&str) -> bool) -> Node {
        let mut copy = self.clone();
        for p in self.panels() {
            if !keep(p) {
                copy.remove(p);
            }
        }
        if copy.is_empty() {
            copy = Node::Tabs {
                panels: vec![],
                active: 0,
            };
        }
        copy
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Window {
    pub id: String,
    pub composition: String,
    pub root: Node,
}

/// Remembers the windows folded into `target` by a merge, so they can be split out again.
#[derive(Debug, Clone, PartialEq)]
struct Merge {
    target: String,
    sources: Vec<Window>,
    /// Window ids in the order they had before the merge, so a split restores it.
    order: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LayoutError {
    UnknownComposition(String),
    UnknownWindow(String),
    UnknownPanel(String),
}

impl std::fmt::Display for LayoutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownComposition(c) => write!(f, "unknown composition `{c}`"),
            Self::UnknownWindow(w) => write!(f, "unknown window `{w}`"),
            Self::UnknownPanel(p) => write!(f, "unknown panel `{p}`"),
        }
    }
}

impl std::error::Error for LayoutError {}

pub const COMPOSITIONS: [&str; 3] = ["market", "operations", "merged"];

pub fn composition(name: &str) -> Option<Node> {
    use Orientation::{Horizontal as H, Vertical as V};
    Some(match name {
        "market" => Node::split(
            H,
            &[0.75, 0.25],
            vec![
                Node::split(
                    V,
                    &[0.94, 0.06],
                    vec![Node::tabs(&["chart"]), Node::tabs(&["instrument"])],
                ),
                Node::tabs(&["tape", "dom", "footprint"]),
            ],
        ),
        "operations" => Node::split(
            V,
            &[0.16, 0.84],
            vec![
                Node::tabs(&["status"]),
                Node::split(
                    H,
                    &[0.3, 0.7],
                    vec![Node::tabs(&["deployments"]), Node::tabs(&["detail"])],
                ),
            ],
        ),
        "merged" => Node::split(
            V,
            &[0.14, 0.82, 0.04],
            vec![
                Node::tabs(&["status"]),
                Node::split(
                    H,
                    &[0.26, 0.74],
                    vec![
                        Node::tabs(&["deployments"]),
                        Node::split(
                            V,
                            &[0.42, 0.58],
                            vec![Node::tabs(&["chart"]), Node::tabs(&["detail"])],
                        ),
                    ],
                ),
                Node::tabs(&["instrument"]),
            ],
        ),
        _ => return None,
    })
}

#[derive(Debug, Clone, Default)]
pub struct Layout {
    windows: Vec<Window>,
    next_id: u32,
    merges: Vec<Merge>,
    /// Where a panel was before it was moved or floated, for docking back.
    origin: HashMap<String, String>,
}

impl Layout {
    pub fn windows(&self) -> &[Window] {
        &self.windows
    }

    pub fn window(&self, id: &str) -> Option<&Window> {
        self.windows.iter().find(|w| w.id == id)
    }

    fn window_mut(&mut self, id: &str) -> Result<&mut Window, LayoutError> {
        self.windows
            .iter_mut()
            .find(|w| w.id == id)
            .ok_or_else(|| LayoutError::UnknownWindow(id.into()))
    }

    pub fn window_of(&self, panel: &str) -> Option<&str> {
        self.windows
            .iter()
            .find(|w| w.root.contains(panel))
            .map(|w| w.id.as_str())
    }

    pub fn all_panels(&self) -> Vec<&str> {
        self.windows.iter().flat_map(|w| w.root.panels()).collect()
    }

    pub fn is_merged(&self, window: &str) -> bool {
        self.merges.iter().any(|m| m.target == window)
    }

    fn fresh_id(&mut self) -> String {
        self.next_id += 1;
        format!("w{}", self.next_id)
    }

    /// Takes `panel` out of whichever window holds it. Windows left empty are dropped.
    fn detach_panel(&mut self, panel: &str) {
        for w in &mut self.windows {
            if w.root.remove(panel) && w.root.is_empty() {
                w.root = Node::Tabs {
                    panels: vec![],
                    active: 0,
                };
            }
        }
        self.windows.retain(|w| !w.root.is_empty());
        let live: Vec<String> = self.windows.iter().map(|w| w.id.clone()).collect();
        self.merges.retain(|m| live.contains(&m.target));
    }

    /// Opens a named composition. A panel it names that already lives in another window is
    /// moved here with its state rather than duplicated.
    pub fn open(&mut self, composition_name: &str) -> Result<String, LayoutError> {
        let root = composition(composition_name)
            .ok_or_else(|| LayoutError::UnknownComposition(composition_name.into()))?;
        let id = self.fresh_id();
        for p in root.panels() {
            if let Some(from) = self.window_of(p).map(str::to_string) {
                self.origin.insert(p.to_string(), from);
            }
            self.detach_panel(p);
        }
        self.windows.push(Window {
            id: id.clone(),
            composition: composition_name.into(),
            root,
        });
        Ok(id)
    }

    /// Closes a window. Its panels move to the first remaining window, so nothing another
    /// window uses is torn down. Closing the last window leaves an empty layout.
    pub fn close(&mut self, window: &str) -> Result<(), LayoutError> {
        let idx = self
            .windows
            .iter()
            .position(|w| w.id == window)
            .ok_or_else(|| LayoutError::UnknownWindow(window.into()))?;
        let closed = self.windows.remove(idx);
        self.merges.retain(|m| m.target != window);
        for m in &mut self.merges {
            m.sources.retain(|s| s.id != window);
        }
        if let Some(dest) = self.windows.first().map(|w| w.id.clone()) {
            for p in closed.root.panels() {
                self.origin.insert(p.to_string(), closed.id.clone());
                self.add_tab(&dest, p);
            }
        }
        Ok(())
    }

    fn add_tab(&mut self, window: &str, panel: &str) {
        if let Ok(w) = self.window_mut(window) {
            if let Node::Tabs { panels, active } = w.root.first_tabs_mut() {
                panels.push(panel.to_string());
                *active = panels.len() - 1;
            }
        }
    }

    /// Moves a panel into another window as a tab of its first group.
    pub fn move_panel(&mut self, panel: &str, to_window: &str) -> Result<(), LayoutError> {
        let from = self
            .window_of(panel)
            .ok_or_else(|| LayoutError::UnknownPanel(panel.into()))?
            .to_string();
        if self.window(to_window).is_none() {
            return Err(LayoutError::UnknownWindow(to_window.into()));
        }
        if from == to_window {
            return Ok(());
        }
        self.origin.insert(panel.to_string(), from);
        self.detach_panel(panel);
        self.add_tab(to_window, panel);
        Ok(())
    }

    /// Floats a panel into a window of its own.
    pub fn float(&mut self, panel: &str) -> Result<String, LayoutError> {
        let from = self
            .window_of(panel)
            .ok_or_else(|| LayoutError::UnknownPanel(panel.into()))?
            .to_string();
        if self.window(&from).map(|w| w.root.panels().len()) == Some(1) {
            return Ok(from);
        }
        self.origin.insert(panel.to_string(), from);
        self.detach_panel(panel);
        let id = self.fresh_id();
        self.windows.push(Window {
            id: id.clone(),
            composition: "floating".into(),
            root: Node::tabs(&[panel]),
        });
        Ok(id)
    }

    /// Docks a panel back into the window it came from (or the first window).
    pub fn dock_back(&mut self, panel: &str) -> Result<(), LayoutError> {
        let current = self
            .window_of(panel)
            .ok_or_else(|| LayoutError::UnknownPanel(panel.into()))?
            .to_string();
        let target = self
            .origin
            .get(panel)
            .filter(|o| **o != current && self.window(o).is_some())
            .cloned()
            .or_else(|| {
                self.windows
                    .iter()
                    .map(|w| w.id.clone())
                    .find(|id| *id != current)
            });
        match target {
            Some(t) => self.move_panel(panel, &t),
            None => Ok(()),
        }
    }

    /// Folds every other window into `window`: their panels become one tab group beside
    /// the window's own arrangement. Panel identity and state are untouched.
    pub fn merge_into(&mut self, window: &str) -> Result<(), LayoutError> {
        if self.window(window).is_none() {
            return Err(LayoutError::UnknownWindow(window.into()));
        }
        let order: Vec<String> = self.windows.iter().map(|w| w.id.clone()).collect();
        let sources: Vec<Window> = self
            .windows
            .iter()
            .filter(|w| w.id != window)
            .cloned()
            .collect();
        if sources.is_empty() {
            return Ok(());
        }
        let panels: Vec<String> = sources
            .iter()
            .flat_map(|w| w.root.panels().into_iter().map(str::to_string))
            .collect();
        self.windows.retain(|w| w.id == window);
        // Merges recorded earlier for the removed windows are folded into this one.
        let mut folded: Vec<Window> = Vec::new();
        for m in self.merges.drain(..) {
            if m.target == window {
                folded.extend(m.sources);
            }
        }
        let target = self.window_mut(window)?;
        let old = std::mem::replace(
            &mut target.root,
            Node::Tabs {
                panels: vec![],
                active: 0,
            },
        );
        target.root = Node::Split {
            orientation: Orientation::Vertical,
            children: vec![old, Node::Tabs { panels, active: 0 }],
            weights: vec![0.6, 0.4],
        };
        folded.extend(sources);
        self.merges.push(Merge {
            target: window.to_string(),
            sources: folded,
            order,
        });
        Ok(())
    }

    /// Restores one panel's original window after a merge; a panel that was never merged
    /// is floated. Returns the window it now lives in.
    pub fn split_out(&mut self, panel: &str) -> Result<String, LayoutError> {
        let current = self
            .window_of(panel)
            .ok_or_else(|| LayoutError::UnknownPanel(panel.into()))?
            .to_string();
        let Some(mi) = self
            .merges
            .iter()
            .position(|m| m.target == current && m.sources.iter().any(|s| s.root.contains(panel)))
        else {
            return self.float(panel);
        };
        let si = self.merges[mi]
            .sources
            .iter()
            .position(|s| s.root.contains(panel))
            .unwrap_or(0);
        let source = self.merges[mi].sources.remove(si);
        let order = self.merges[mi].order.clone();
        if self.merges[mi].sources.is_empty() {
            self.merges.remove(mi);
        }
        // Everything of that window still living in the merged one goes back with it.
        let here: Vec<String> = self
            .window(&current)
            .map(|w| w.root.panels().iter().map(|p| p.to_string()).collect())
            .unwrap_or_default();
        let root = source.root.restricted(&|p| here.iter().any(|h| h == p));
        for p in root.panels() {
            if let Some(w) = self.windows.iter_mut().find(|w| w.id == current) {
                w.root.remove(p);
            }
        }
        self.windows.push(Window {
            id: source.id.clone(),
            composition: source.composition,
            root,
        });
        // Back where they were; windows the merge never knew keep their relative order last.
        self.windows.sort_by_key(|w| {
            order
                .iter()
                .position(|id| *id == w.id)
                .unwrap_or(usize::MAX)
        });
        Ok(source.id)
    }

    /// Splits every window folded into `window` back out.
    pub fn split_all(&mut self, window: &str) -> Result<(), LayoutError> {
        while let Some(panel) = self
            .merges
            .iter()
            .find(|m| m.target == window)
            .and_then(|m| m.sources.first())
            .and_then(|s| s.root.panels().first().map(|p| p.to_string()))
        {
            if self.window_of(&panel) != Some(window) {
                // The panel already left; drop the stale record rather than looping.
                if let Some(m) = self.merges.iter_mut().find(|m| m.target == window) {
                    m.sources.remove(0);
                }
                continue;
            }
            self.split_out(&panel)?;
        }
        self.merges.retain(|m| !m.sources.is_empty());
        Ok(())
    }

    pub fn set_active(&mut self, window: &str, panel: &str) -> Result<(), LayoutError> {
        let w = self.window_mut(window)?;
        if w.root.set_active(panel) {
            Ok(())
        } else {
            Err(LayoutError::UnknownPanel(panel.into()))
        }
    }

    /// Records the pane sizes the user dragged to, for as long as the window is open.
    pub fn set_weights(
        &mut self,
        window: &str,
        path: &[usize],
        new_weights: &[f64],
    ) -> Result<(), LayoutError> {
        let w = self.window_mut(window)?;
        if let Some(Node::Split { weights, .. }) = w.root.at_path_mut(path) {
            if weights.len() == new_weights.len() && new_weights.iter().all(|x| *x > 0.0) {
                *weights = new_weights.to_vec();
            }
        }
        Ok(())
    }

    pub fn window_json(&self, window: &str) -> String {
        match self.window(window) {
            Some(w) => {
                let mut v = serde_json::to_value(w).unwrap_or_default();
                v["merged"] = self.is_merged(window).into();
                v.to_string()
            }
            None => "null".into(),
        }
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(&self.windows).unwrap_or_else(|_| "[]".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn two_windows() -> (Layout, String, String) {
        let mut l = Layout::default();
        let m = l.open("market").unwrap();
        let o = l.open("operations").unwrap();
        (l, m, o)
    }

    #[test]
    fn compositions_are_valid_and_panels_unique() {
        for name in COMPOSITIONS {
            let node = composition(name).unwrap();
            let panels = node.panels();
            assert!(!panels.is_empty());
            let mut sorted = panels.clone();
            sorted.sort();
            sorted.dedup();
            assert_eq!(sorted.len(), panels.len(), "{name}");
            assert!(panels.iter().all(|p| super::super::panels::is_valid_id(p)));
        }
        assert!(composition("nope").is_none());
    }

    #[test]
    fn each_panel_lives_in_exactly_one_window() {
        let (l, _, _) = two_windows();
        let mut all = l.all_panels();
        let n = all.len();
        all.sort();
        all.dedup();
        assert_eq!(all.len(), n);
    }

    #[test]
    fn opening_a_composition_pulls_existing_panels_instead_of_duplicating() {
        let mut l = Layout::default();
        let merged = l.open("merged").unwrap();
        let market = l.open("market").unwrap();
        assert_eq!(l.window_of("chart"), Some(market.as_str()));
        assert_eq!(l.window_of("detail"), Some(merged.as_str()));
        let n = l.all_panels().len();
        let mut d = l.all_panels();
        d.sort();
        d.dedup();
        assert_eq!(d.len(), n);
    }

    #[test]
    fn moved_panel_keeps_its_identity_and_leaves_no_copy() {
        let (mut l, m, o) = two_windows();
        l.move_panel("chart", &o).unwrap();
        assert_eq!(l.window_of("chart"), Some(o.as_str()));
        assert!(!l.window(&m).unwrap().root.panels().contains(&"chart"));
        assert_eq!(l.all_panels().iter().filter(|p| **p == "chart").count(), 1);
        // moved in as the active tab
        assert!(l.window(&o).unwrap().root.panels().contains(&"chart"));
    }

    #[test]
    fn moving_the_last_panel_out_drops_the_window() {
        let mut l = Layout::default();
        let a = l.open("operations").unwrap();
        let f = l.float("chart").unwrap_err();
        assert_eq!(f, LayoutError::UnknownPanel("chart".into()));
        let b = l.open("market").unwrap();
        for p in ["chart", "instrument", "tape", "dom", "footprint"] {
            l.move_panel(p, &a).unwrap();
        }
        assert!(l.window(&b).is_none());
        assert_eq!(l.windows().len(), 1);
    }

    #[test]
    fn float_then_dock_back_round_trips_the_window() {
        let (mut l, m, _o) = two_windows();
        let f = l.float("chart").unwrap();
        assert_ne!(f, m);
        assert_eq!(l.window_of("chart"), Some(f.as_str()));
        l.dock_back("chart").unwrap();
        assert_eq!(l.window_of("chart"), Some(m.as_str()));
        assert!(l.window(&f).is_none());
    }

    #[test]
    fn closing_a_window_keeps_its_panels_alive_elsewhere() {
        let (mut l, m, o) = two_windows();
        let before: Vec<String> = {
            let mut v: Vec<String> = l.all_panels().iter().map(|s| s.to_string()).collect();
            v.sort();
            v
        };
        l.close(&m).unwrap();
        assert_eq!(l.windows().len(), 1);
        let mut after: Vec<String> = l.all_panels().iter().map(|s| s.to_string()).collect();
        after.sort();
        assert_eq!(before, after);
        assert_eq!(l.window_of("chart"), Some(o.as_str()));
        l.close(&o).unwrap();
        assert!(l.windows().is_empty());
    }

    #[test]
    fn reopening_a_closed_window_reclaims_its_panels() {
        let (mut l, m, _o) = two_windows();
        l.close(&m).unwrap();
        let m2 = l.open("market").unwrap();
        assert_eq!(l.window_of("chart"), Some(m2.as_str()));
        assert_eq!(l.windows().len(), 2);
    }

    #[test]
    fn merge_keeps_every_panel_and_split_restores_the_arrangement() {
        let (mut l, m, o) = two_windows();
        let before_market = l.window(&m).unwrap().clone();
        let before_ops = l.window(&o).unwrap().clone();
        let mut panels: Vec<String> = l.all_panels().iter().map(|s| s.to_string()).collect();
        panels.sort();

        l.merge_into(&o).unwrap();
        assert_eq!(l.windows().len(), 1);
        assert!(l.is_merged(&o));
        let mut merged: Vec<String> = l.all_panels().iter().map(|s| s.to_string()).collect();
        merged.sort();
        assert_eq!(merged, panels);

        l.split_all(&o).unwrap();
        assert!(!l.is_merged(&o));
        assert_eq!(l.windows().len(), 2);
        assert_eq!(l.window(&o).unwrap(), &before_ops);
        assert_eq!(l.window(&m).unwrap(), &before_market);
        let ids: Vec<&str> = l.windows().iter().map(|w| w.id.as_str()).collect();
        assert_eq!(ids, [m.as_str(), o.as_str()], "window order restored");
    }

    #[test]
    fn split_out_of_one_panel_restores_its_window_with_its_siblings() {
        let (mut l, m, o) = two_windows();
        l.merge_into(&o).unwrap();
        let back = l.split_out("chart").unwrap();
        assert_eq!(back, m);
        assert_eq!(l.window_of("chart"), Some(m.as_str()));
        assert_eq!(l.window_of("instrument"), Some(m.as_str()));
        assert_eq!(l.window_of("detail"), Some(o.as_str()));
        assert!(!l.is_merged(&o));
    }

    #[test]
    fn active_tab_and_pane_sizes_are_remembered() {
        let (mut l, m, _) = two_windows();
        l.set_active(&m, "dom").unwrap();
        l.set_weights(&m, &[], &[0.5, 0.5]).unwrap();
        match &l.window(&m).unwrap().root {
            Node::Split {
                weights, children, ..
            } => {
                assert_eq!(weights, &vec![0.5, 0.5]);
                match &children[1] {
                    Node::Tabs { active, .. } => assert_eq!(*active, 1),
                    other => panic!("{other:?}"),
                }
            }
            other => panic!("{other:?}"),
        }
        assert!(l.set_active(&m, "detail").is_err());
    }

    #[test]
    fn layout_round_trips_through_json() {
        let (l, m, _) = two_windows();
        let v: serde_json::Value = serde_json::from_str(&l.window_json(&m)).unwrap();
        assert_eq!(v["composition"], "market");
        let root: Node = serde_json::from_value(v["root"].clone()).unwrap();
        assert_eq!(&root, &l.window(&m).unwrap().root);
    }
}
