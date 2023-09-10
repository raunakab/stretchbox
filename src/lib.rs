use std::collections::BTreeMap;

use cherrytree::{Tree, Node};
use indexmap::IndexSet;
use slotmap::new_key_type;

new_key_type! { pub struct ConstraintKey; }

new_key_type! { pub struct FrameKey; }

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

pub fn solve(
    constraint_tree: &Tree<ConstraintKey, Constraint>,
    frame_tree: &mut Tree<FrameKey, Frame>,
    map: &mut BTreeMap<ConstraintKey, FrameKey>,
    length_x: f64,
) -> bool {
    let (root_constraint_key, root_constraint_node) = constraint_tree.root_key_value().unwrap();

    match *root_constraint_node.value {
        Constraint::Exact(..) | Constraint::Minimize => false,
        Constraint::Scale(scale) => {
            if scale != 0 {
                let root_frame = Frame {
                    offset_x: 0.,
                    length_x,
                };

                let root_frame_key = frame_tree.insert_root(root_frame);
                map.insert(root_constraint_key, root_frame_key);

                solve_child_keys(constraint_tree, frame_tree, map, root_constraint_node.child_keys, root_frame_key, length_x);
            };

            true
        },
    }
}

fn solve_child_keys(
    constraint_tree: &Tree<ConstraintKey, Constraint>,
    frame_tree: &mut Tree<FrameKey, Frame>,
    map: &mut BTreeMap<ConstraintKey, FrameKey>,
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
            },
            Constraint::Scale(scale) => {
                total_scale = total_scale.saturating_add(scale);
            },
            Constraint::Minimize => {
                let length_x = find_minimizing_length_x(constraint_tree, constraint_node.child_keys, remaining_length_x);
                *current_length_x = Some(length_x);
                remaining_length_x -= length_x;
            },
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

        let frame = Frame {
            offset_x,
            length_x,
        };

        let frame_key = frame_tree.insert(frame, parent_frame_key).unwrap();
        map.insert(constraint_key, frame_key);

        offset_x += length_x;

        solve_child_keys(constraint_tree, frame_tree, map, constraint_node.child_keys, frame_key, length_x);
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
                let minimize = find_minimizing_length_x(constraint_tree, constraint_node.child_keys, remaining_length_x);
                total_length_x += minimize;
            },
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
