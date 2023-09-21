mod solver;

use std::collections::BTreeMap;

use cherrytree::{Node, Tree};
use indexmap::IndexSet;
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

    pub fn insert_root(&mut self, constraint: Constraint) -> Option<ConstraintKey> {
        self.insert_root_with_capacity(constraint, 0)
    }

    pub fn insert_root_with_capacity(
        &mut self,
        constraint: Constraint,
        capacity: usize,
    ) -> Option<ConstraintKey> {
        let both_fills_are_absolute_scales = matches! { constraint.fill, Fill::Absolute { x: FillType::Scale(..), y: FillType::Scale(..) }};
        let both_fills_are_relative_scales = matches! { constraint.fill, Fill::Relative { main: FillType::Scale(..), cross: FillType::Scale(..) }};

        let both_fills_are_scales = both_fills_are_absolute_scales | both_fills_are_relative_scales;

        both_fills_are_scales.then(|| {
            let root_key = self
                .constraint_tree
                .insert_root_with_capacity(constraint, capacity);
            self.is_dirty = true;
            root_key
        })
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

    pub fn reorder_children<F>(
        &mut self,
        constraint_key: ConstraintKey,
        get_reordered_constraint_keys: F,
    ) -> bool
    where
        F: FnOnce(&IndexSet<ConstraintKey>) -> IndexSet<ConstraintKey>,
    {
        let did_reorder = self
            .constraint_tree
            .reorder_children(constraint_key, get_reordered_constraint_keys);
        if did_reorder {
            self.is_dirty = true;
        };
        did_reorder
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

    pub fn rebase(
        &mut self,
        consraint_key: ConstraintKey,
        new_parent_consraint_key: ConstraintKey,
    ) -> bool {
        let did_rebase = self
            .constraint_tree
            .rebase(consraint_key, new_parent_consraint_key);
        if did_rebase {
            self.is_dirty = true;
        };
        did_rebase
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

    pub fn solve(&mut self, length_x: f64, length_y: f64) {
        let is_dirty = self.is_dirty;
        let is_empty = self.constraint_tree.is_empty();

        match (is_dirty, is_empty) {
            (true, true) => self.is_dirty = false,

            (true, false) => {
                let length_x = length_x.max(0.);
                let length_y = length_y.max(0.);

                solve(
                    &self.constraint_tree,
                    &mut self.frame_tree,
                    &mut self.key_map,
                    length_x,
                    length_y,
                );

                self.is_dirty = false;
            }

            (false, _) => (),
        }
    }
}

#[derive(Default, Debug, Clone, Copy, PartialEq)]
pub struct Constraint {
    pub fill: Fill,
    pub content: Content,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Fill {
    Absolute { x: FillType, y: FillType },
    Relative { main: FillType, cross: FillType },
}

impl Fill {
    fn to_absolute_fill(self, direction: Direction) -> AbsoluteFill {
        match self {
            Self::Absolute { x, y } => AbsoluteFill { x, y },
            Self::Relative { main, cross } => match direction {
                Direction::Horizontal => AbsoluteFill { x: main, y: cross },
                Direction::Vertical => AbsoluteFill { x: cross, y: main },
            },
        }
    }

    fn to_absolute_fill_horizontal(self) -> AbsoluteFill {
        match self {
            Self::Absolute { x, y } => AbsoluteFill { x, y },
            Self::Relative { main, cross } => AbsoluteFill { x: main, y: cross },
        }
    }

    fn to_absolute_fill_vertical(self) -> AbsoluteFill {
        match self {
            Self::Absolute { x, y } => AbsoluteFill { x, y },
            Self::Relative { main, cross } => AbsoluteFill { x: cross, y: main },
        }
    }
}

impl Default for Fill {
    fn default() -> Self {
        Self::Relative {
            main: FillType::default(),
            cross: FillType::default(),
        }
    }
}

#[derive(Default, Debug, Clone, Copy, PartialEq)]
struct AbsoluteFill {
    x: FillType,
    y: FillType,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FillType {
    Exact(f64),
    Scale(usize),
    Minimize,
}

impl Default for FillType {
    fn default() -> Self {
        Self::Scale(1)
    }
}

#[derive(Default, Debug, Clone, Copy, PartialEq)]
pub struct Content {
    pub direction: Direction,
    pub padding: Padding,
    pub align_main: Align,
    pub align_cross: Align,
}

#[derive(Default, Debug, Clone, Copy, PartialEq)]
pub struct Padding {
    pub left: f64,
    pub right: f64,

    pub top: f64,
    pub bottom: f64,
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Horizontal,

    #[default]
    Vertical,
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Align {
    #[default]
    Start,
    Middle,
    End,
}

#[derive(Default, Debug, Clone, Copy, PartialEq)]
pub struct Frame {
    pub offset_x: f64,
    pub length_x: f64,

    pub offset_y: f64,
    pub length_y: f64,
}
