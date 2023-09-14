#[cfg(test)]
mod tests;

use std::collections::BTreeMap;

use cherrytree::{Tree, Node};
use indexmap::IndexSet;

use crate::{ConstraintKey, Constraint, Fill, FrameKey, Frame, Padding};

pub(super) fn solve(
    constraint_tree: &Tree<ConstraintKey, Constraint>,
    frame_tree: &mut Tree<FrameKey, Frame>,
    key_map: &mut BTreeMap<ConstraintKey, FrameKey>,
    length_x: f64,
) -> bool {
    let (root_constraint_key, root_constraint_node) = constraint_tree.root_key_value().unwrap();

    match root_constraint_node.value.fill_x {
        Fill::Exact(..) | Fill::Minimize => false,
        Fill::Scale(scale) => {
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

            let root_content_frame = generate_content_frame(length_x, root_constraint_node.value.padding);

            solve_child_keys(
                constraint_tree,
                frame_tree,
                key_map,
                root_constraint_node.child_keys,
                root_frame_key,
                root_content_frame,
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
    content_frame: Frame,
) {
    let mut data = iter(constraint_tree, constraint_keys)
        .map(|(constraint_key, constraint_node)| (constraint_key, constraint_node, None::<f64>))
        .collect::<Vec<_>>();

    let mut remaining_length_x = content_frame.length_x;
    let mut total_scale: usize = 0;

    for (_, constraint_node, current_length_x) in data.iter_mut() {
        match constraint_node.value.fill_x {
            Fill::Exact(exact) => {
                let length_x = exact.min(remaining_length_x);
                *current_length_x = Some(length_x);
                remaining_length_x -= length_x;
            }
            Fill::Scale(scale) => {
                total_scale = total_scale.saturating_add(scale);
            }
            Fill::Minimize => {
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
            if let Fill::Scale(scale) = constraint_node.value.fill_x {
                let length_x = ((scale as f64) / (total_scale as f64)) * remaining_length_x;
                *current_length_x = Some(length_x);
            }
        }
    }

    let mut offset_x: f64 = content_frame.offset_x;
    for (constraint_key, constraint_node, length_x) in data {
        let length_x = length_x.unwrap_or_default();

        let frame = Frame { offset_x, length_x };

        let frame_key = frame_tree.insert(frame, parent_frame_key).unwrap();
        key_map.insert(constraint_key, frame_key);

        offset_x += length_x;

        let content_frame = generate_content_frame(length_x, constraint_node.value.padding);

        solve_child_keys(
            constraint_tree,
            frame_tree,
            key_map,
            constraint_node.child_keys,
            frame_key,
            content_frame,
        );
    }
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

fn find_minimizing_length_x(
    constraint_tree: &Tree<ConstraintKey, Constraint>,
    constraint_keys: &IndexSet<ConstraintKey>,
    length_x: f64,
) -> f64 {
    let mut total_length_x: f64 = 0.;

    for (_, constraint_node) in iter(constraint_tree, constraint_keys) {
        match constraint_node.value.fill_x {
            Fill::Exact(exact) => total_length_x = (total_length_x + exact).min(length_x),
            Fill::Scale(..) => (),
            Fill::Minimize => {
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

fn generate_content_frame(
    length_x: f64,
    padding: Padding,
) -> Frame {
    let content_start_x = padding.left.min(length_x);
    let content_end_x = (length_x - padding.right).max(0.);
    let content_length_x = (content_end_x - content_start_x).max(0.);

    Frame { offset_x: content_start_x, length_x: content_length_x }
}
