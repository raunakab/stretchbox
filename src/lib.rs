mod solver;

use std::collections::BTreeMap;

use cherrytree::{Node, Tree};
use slotmap::new_key_type;

use crate::solver::solve;

new_key_type! { pub struct ConstraintKey; }

new_key_type! { pub struct FrameKey; }

#[derive(Default, Clone)]
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

    pub fn is_dirty(&self) -> bool {
        self.is_dirty
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

#[derive(Default, Debug, Clone, Copy, PartialEq)]
pub struct Constraint {
    pub fill_x: Fill,
    pub padding: Padding,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Fill {
    Exact(f64),
    Scale(usize),
    Minimize,
}

impl Default for Fill {
    fn default() -> Self {
        Self::Scale(1)
    }
}
#[derive(Default, Debug, Clone, Copy, PartialEq)]
pub struct Padding {
    pub left: f64,
    pub right: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Frame {
    pub offset_x: f64,
    pub length_x: f64,
}
