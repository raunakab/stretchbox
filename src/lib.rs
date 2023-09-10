use std::collections::BTreeMap;

use cherrytree::{Node, Tree};
use indexmap::IndexSet;
use slotmap::new_key_type;

new_key_type! { pub struct ConstraintKey; }

new_key_type! { pub struct FrameKey; }

pub struct Solver {
    constraint_tree: Tree<ConstraintKey, Constraint>,
    frame_tree: Tree<FrameKey, Frame>,
    key_map: BTreeMap<ConstraintKey, FrameKey>,
    is_dirty: bool,
}

impl Solver {
    // Creation methods:

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            constraint_tree: Tree::with_capacity(capacity),
            frame_tree: Tree::with_capacity(capacity),
            key_map: BTreeMap::default(),
            is_dirty: false,
        }
    }

    // Checking/assertion methods:

    pub fn is_empty(&self) -> bool {
        self.constraint_tree.is_empty()
    }

    pub fn contains(&self, constraint_key: ConstraintKey) -> bool {
        self.constraint_tree.contains(constraint_key)
    }

    // Insertion/removal methods:

    pub fn insert_root(&mut self, constraint: Constraint) -> ConstraintKey {
        self.insert_root_with_capacity(constraint, 0)
    }

    pub fn insert_root_with_capacity(
        &mut self,
        constraint: Constraint,
        capacity: usize,
    ) -> ConstraintKey {
        let root_key = self
            .constraint_tree
            .insert_root_with_capacity(constraint, capacity);
        self.is_dirty = true;
        root_key
    }

    pub fn insert(
        &mut self,
        constraint: Constraint,
        parent_constraint_key: ConstraintKey,
    ) -> Option<ConstraintKey> {
        self.insert_with_capacity(constraint, parent_constraint_key, 0)
    }

    pub fn insert_with_capacity(
        &mut self,
        constraint: Constraint,
        parent_constraint_key: ConstraintKey,
        capacity: usize,
    ) -> Option<ConstraintKey> {
        let root_key =
            self.constraint_tree
                .insert_with_capacity(constraint, parent_constraint_key, capacity);
        if root_key.is_some() {
            self.is_dirty = true;
        };
        root_key
    }

    pub fn remove(
        &mut self,
        constraint_key: ConstraintKey,
        size_hint: Option<usize>,
    ) -> Option<Constraint> {
        let old_value = self.constraint_tree.remove(constraint_key, size_hint);
        if old_value.is_some() {
            self.is_dirty = true;
        };
        old_value
    }

    pub fn clear(&mut self) {
        self.constraint_tree.clear();
        self.frame_tree.clear();
        self.key_map.clear();
        self.is_dirty = false;
    }

    // Getter/setter methods:

    pub fn root_constraint_key(&self) -> Option<ConstraintKey> {
        self.constraint_tree.root_key()
    }

    pub fn root_constraint_key_value(
        &self,
    ) -> Option<(ConstraintKey, Node<'_, ConstraintKey, Constraint>)> {
        self.constraint_tree.root_key_value()
    }

    pub fn get(
        &self,
        constraint_key: ConstraintKey,
    ) -> Option<Node<'_, ConstraintKey, Constraint>> {
        self.constraint_tree.get(constraint_key)
    }

    pub fn get_frame(&self, constraint_key: ConstraintKey) -> Option<Frame> {
        let contains_constraint_key = self.constraint_tree.contains(constraint_key);
        let is_dirty = self.is_dirty;

        match (contains_constraint_key, is_dirty) {
            (false, _) => None,

            (true, true) => None,

            (true, false) => {
                let frame_key = *self.key_map.get(&constraint_key).unwrap();
                let frame = *self.frame_tree.get(frame_key).unwrap().value;
                Some(frame)
            }
        }
    }

    pub fn set(
        &mut self,
        constraint_key: ConstraintKey,
        new_constraint: Constraint,
    ) -> Option<Constraint> {
        let old_constraint = self.constraint_tree.set(constraint_key, new_constraint);
        if old_constraint.is_some() {
            self.is_dirty = true;
        };
        old_constraint
    }

    // Solve method:

    pub fn solve(&mut self, length_x: f64) -> bool {
        let is_dirty = self.is_dirty;
        let is_empty = self.constraint_tree.is_empty();

        match (is_dirty, is_empty) {
            (true, true) => {
                self.is_dirty = false;
                true
            }

            (true, false) => {
                let length_x = length_x.max(0.);
                let did_solve = solve(
                    &self.constraint_tree,
                    &mut self.frame_tree,
                    &mut self.key_map,
                    length_x,
                );
                if did_solve {
                    self.is_dirty = false;
                };
                did_solve
            }

            (false, _) => true,
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum Constraint {
    Exact(f64),
    Scale(usize),
    Minimize,
}

impl Default for Constraint {
    fn default() -> Self {
        Self::Scale(1)
    }
}

#[derive(Clone, Copy, PartialEq)]
pub struct Frame {
    offset_x: f64,
    length_x: f64,
}

fn solve(
    constraint_tree: &Tree<ConstraintKey, Constraint>,
    frame_tree: &mut Tree<FrameKey, Frame>,
    key_map: &mut BTreeMap<ConstraintKey, FrameKey>,
    length_x: f64,
) -> bool {
    let (root_constraint_key, root_constraint_node) = constraint_tree.root_key_value().unwrap();

    match *root_constraint_node.value {
        Constraint::Exact(..) | Constraint::Minimize => false,
        Constraint::Scale(scale) => {
            let length_x = match scale {
                0 => 0.,
                _ => length_x,
            };

            let root_frame = Frame {
                offset_x: 0.,
                length_x,
            };

            let root_frame_key = frame_tree.insert_root(root_frame);
            key_map.insert(root_constraint_key, root_frame_key);

            solve_child_keys(
                constraint_tree,
                frame_tree,
                key_map,
                root_constraint_node.child_keys,
                root_frame_key,
                length_x,
            );

            true
        }
    }
}

fn solve_child_keys(
    constraint_tree: &Tree<ConstraintKey, Constraint>,
    frame_tree: &mut Tree<FrameKey, Frame>,
    key_map: &mut BTreeMap<ConstraintKey, FrameKey>,
    constraint_keys: &IndexSet<ConstraintKey>,
    parent_frame_key: FrameKey,
    length_x: f64,
) {
    let mut data = iter(constraint_tree, constraint_keys)
        .map(|(constraint_key, constraint_node)| (constraint_key, constraint_node, None::<f64>))
        .collect::<Vec<_>>();

    let mut remaining_length_x = length_x;
    let mut total_scale: usize = 0;

    for (_, constraint_node, current_length_x) in data.iter_mut() {
        match *constraint_node.value {
            Constraint::Exact(exact) => {
                let length_x = exact.min(remaining_length_x);
                *current_length_x = Some(length_x);
                remaining_length_x -= length_x;
            }
            Constraint::Scale(scale) => {
                total_scale = total_scale.saturating_add(scale);
            }
            Constraint::Minimize => {
                let length_x = find_minimizing_length_x(
                    constraint_tree,
                    constraint_node.child_keys,
                    remaining_length_x,
                );
                *current_length_x = Some(length_x);
                remaining_length_x -= length_x;
            }
        }
    }

    if total_scale != 0 {
        for (_, constraint_node, current_length_x) in data.iter_mut() {
            if let Constraint::Scale(scale) = *constraint_node.value {
                let length_x = ((scale as f64) / (total_scale as f64)) * remaining_length_x;
                *current_length_x = Some(length_x);
            }
        }
    }

    let mut offset_x: f64 = 0.;
    for (constraint_key, constraint_node, length_x) in data {
        let length_x = length_x.unwrap_or_default();

        let frame = Frame { offset_x, length_x };

        let frame_key = frame_tree.insert(frame, parent_frame_key).unwrap();
        key_map.insert(constraint_key, frame_key);

        offset_x += length_x;

        solve_child_keys(
            constraint_tree,
            frame_tree,
            key_map,
            constraint_node.child_keys,
            frame_key,
            length_x,
        );
    }
}

fn find_minimizing_length_x(
    constraint_tree: &Tree<ConstraintKey, Constraint>,
    constraint_keys: &IndexSet<ConstraintKey>,
    length_x: f64,
) -> f64 {
    let mut total_length_x: f64 = 0.;

    for (_, constraint_node) in iter(constraint_tree, constraint_keys) {
        match *constraint_node.value {
            Constraint::Exact(exact) => total_length_x = (total_length_x + exact).min(length_x),
            Constraint::Scale(..) => (),
            Constraint::Minimize => {
                let remaining_length_x = length_x - total_length_x;
                let minimize = find_minimizing_length_x(
                    constraint_tree,
                    constraint_node.child_keys,
                    remaining_length_x,
                );
                total_length_x += minimize;
            }
        }
    }

    total_length_x
}

fn iter<'a>(
    constraint_tree: &'a Tree<ConstraintKey, Constraint>,
    constraint_keys: &'a IndexSet<ConstraintKey>,
) -> impl Iterator<Item = (ConstraintKey, Node<'a, ConstraintKey, Constraint>)> {
    constraint_keys.iter().map(|&constraint_key| {
        let constraint_node = constraint_tree.get(constraint_key).unwrap();
        (constraint_key, constraint_node)
    })
}
