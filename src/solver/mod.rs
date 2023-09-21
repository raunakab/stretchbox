use std::collections::BTreeMap;

use cherrytree::{Tree, Node};
use indexmap::IndexSet;

use crate::{ConstraintKey, Constraint, FrameKey, Frame, Content, RelativePadding, RelativeFrame, FillType, Align, Direction};

pub(super) fn solve(
    constraint_tree: &Tree<ConstraintKey, Constraint>,
    frame_tree: &mut Tree<FrameKey, Frame>,
    key_map: &mut BTreeMap<ConstraintKey, FrameKey>,
    length_x: f64,
    length_y: f64,
) {
    let (root_constraint_key, root_constraint_node) = constraint_tree.root_key_value().unwrap();

    let relative_fill = root_constraint_node.value.fill.to_relative_fill(Direction::Vertical);

    let length_x = match relative_fill.cross {
        FillType::Scale(0) => 0.,
        FillType::Scale(_) => length_x,
        _ => unreachable!(),
    };

    let length_y = match relative_fill.main {
        FillType::Scale(0) => 0.,
        FillType::Scale(_) => length_y,
        _ => unreachable!(),
    };

    let root_frame = Frame {
        offset_x: 0.,
        length_x,
        offset_y: 0.,
        length_y,
    };

    let number_of_child_keys = root_constraint_node.child_keys.len();
    let root_frame_key = frame_tree.insert_root_with_capacity(root_frame, number_of_child_keys);
    key_map.insert(root_constraint_key, root_frame_key);

    let relative_padding = root_constraint_node.value.content.padding.to_relative_padding(Direction::Vertical);
    let root_relative_content_frame = generate_content_frame_relative(relative_padding, length_y, length_x);

    solve_child_keys_relative(constraint_tree, frame_tree, key_map, root_constraint_node.child_keys, root_frame_key, root_relative_content_frame, root_constraint_node.value.content);
}

fn solve_child_keys_relative(
    constraint_tree: &Tree<ConstraintKey, Constraint>,
    frame_tree: &mut Tree<FrameKey, Frame>,
    key_map: &mut BTreeMap<ConstraintKey, FrameKey>,
    constraint_keys: &IndexSet<ConstraintKey>,
    parent_frame_key: FrameKey,
    relative_content_frame: RelativeFrame,
    parent_content: Content,
) {
    let mut remaining_length_main = relative_content_frame.length_main;
    let mut total_scale_main: usize = 0;

    let mut relative_lengths = iter(constraint_tree, constraint_keys)
        .map(|(_, constraint_node)| {
            let relatve_fill = constraint_node.value.fill.to_relative_fill(parent_content.direction);

            let mut cache = None;

            let length_main = match relatve_fill.main {
                FillType::Exact(exact_main) => {
                    let exact_main = exact_main.min(remaining_length_main);
                    remaining_length_main -= exact_main;
                    Some(exact_main)
                },
                FillType::Scale(scale_main) => {
                    total_scale_main = total_scale_main.checked_add(scale_main).unwrap();
                    None
                },
                FillType::Minimize => {
                    let (minimizing_length_main, minimizing_length_cross) = find_minimizing_length_relative(constraint_tree, constraint_node.child_keys, constraint_node.value.content.direction, remaining_length_main, relative_content_frame.length_cross);
                    cache = Some(minimizing_length_cross);
                    Some(minimizing_length_main)
                },
            };

            let length_cross = match relatve_fill.cross {
                FillType::Exact(exact_cross) => exact_cross.min(relative_content_frame.length_cross),
                FillType::Scale(0) => 0.,
                FillType::Scale(_) => relative_content_frame.length_cross,
                FillType::Minimize => cache.unwrap_or_else(|| {
                    let (_, minimizing_length_cross) = find_minimizing_length_relative(constraint_tree, constraint_node.child_keys, constraint_node.value.content.direction, remaining_length_main, relative_content_frame.length_cross);
                    minimizing_length_cross
                }),
            };

            let remaining_length_cross = relative_content_frame.length_cross - length_cross;
            let offset_cross = relative_content_frame.offset_cross + match parent_content.align_cross {
                Align::Start => 0.,
                Align::Middle => remaining_length_cross / 2.,
                Align::End => remaining_length_cross,
            };

            (relatve_fill, length_main, length_cross, offset_cross)
        })
        .collect::<Vec<_>>();

    let mut offset_main = match total_scale_main {
        0 => relative_content_frame.offset_main + match parent_content.align_main {
            Align::Start => 0.,
            Align::Middle => remaining_length_main / 2.,
            Align::End => remaining_length_main,
        },
        _ => {
            for (relative_fill, length_main, _, _) in &mut relative_lengths {
                if let FillType::Scale(scale_main) = relative_fill.main {
                    let proportion = (scale_main as f64) / (total_scale_main as f64);
                    *length_main = Some(proportion * remaining_length_main);
                };
            }

            0.
        },
    };

    for ((constraint_key, constraint_node), (_, length_main, length_cross, offset_cross)) in iter(constraint_tree, constraint_keys).zip(relative_lengths) {
        let length_main = length_main.unwrap_or_default();

        let relative_frame = RelativeFrame {
            offset_main,
            length_main,
            offset_cross,
            length_cross,
        };

        offset_main -= length_main;

        let number_of_child_keys = constraint_node.child_keys.len();
        let frame = relative_frame.to_frame(parent_content.direction);
        let frame_key = frame_tree.insert_with_capacity(frame, parent_frame_key, number_of_child_keys).unwrap();
        key_map.insert(constraint_key, frame_key);

        let relative_padding = constraint_node.value.content.padding.to_relative_padding(parent_content.direction);
        let relative_content_frame = generate_content_frame_relative(relative_padding, length_main, length_cross);

        solve_child_keys_relative(constraint_tree, frame_tree, key_map, constraint_node.child_keys, frame_key, relative_content_frame, constraint_node.value.content);
    }
}

fn generate_content_frame_relative(relative_padding: RelativePadding, length_main: f64, length_cross: f64) -> RelativeFrame {
    let content_start_main = relative_padding.main_start.min(length_main);
    let content_end_main = (length_main - relative_padding.main_end).max(0.);
    let content_length_main = (content_end_main - content_start_main).max(0.);

    let content_start_cross = relative_padding.cross_start.min(length_cross);
    let content_end_cross = (length_cross - relative_padding.cross_end).max(0.);
    let content_length_cross = (content_end_cross - content_start_cross).max(0.);

    RelativeFrame {
        offset_main: content_start_main,
        length_main: content_length_main,
        offset_cross: content_start_cross,
        length_cross: content_length_cross,
    }
}

fn find_minimizing_length_relative(
    constraint_tree: &Tree<ConstraintKey, Constraint>,
    constraint_keys: &IndexSet<ConstraintKey>,
    direction: Direction,
    max_length_main: f64,
    max_length_cross: f64,
) -> (f64, f64) {
    let mut remaining_length_main: f64 = max_length_main;
    let mut max_seen_length_cross: f64 = 0.;

    let mut cache = None;

    for (_, constraint_node) in iter(constraint_tree, constraint_keys) {
        let relative_fill = constraint_node.value.fill.to_relative_fill(direction);
        let relative_padding = constraint_node.value.content.padding.to_relative_padding(direction);

        let length_main = match relative_fill.main {
            FillType::Exact(exact_main) => exact_main + relative_padding.main_start + relative_padding.main_end,
            FillType::Scale(..) => relative_padding.main_start + relative_padding.main_end,
            FillType::Minimize => {
                let (sub_minimizing_length_main, sub_minimizing_length_cross) = find_minimizing_length_relative(constraint_tree, constraint_node.child_keys, constraint_node.value.content.direction, remaining_length_main, max_length_cross);
                cache = Some(sub_minimizing_length_cross);
                sub_minimizing_length_main
            },
        }.min(remaining_length_main);

        let length_cross = match relative_fill.cross {
            FillType::Exact(exact_cross) => exact_cross + relative_padding.cross_start + relative_padding.cross_end,
            FillType::Scale(..) => relative_padding.cross_start + relative_padding.cross_end,
            FillType::Minimize => cache.unwrap_or_else(|| {
                let (_, sub_minimizing_length_cross) = find_minimizing_length_relative(constraint_tree, constraint_node.child_keys, constraint_node.value.content.direction, remaining_length_main, max_length_cross);
                sub_minimizing_length_cross
            }),
        };

        remaining_length_main -= length_main;
        max_seen_length_cross = max_seen_length_cross.max(length_cross);
    }

    let minimizing_length_main = max_length_main - remaining_length_main;
    let minimizing_length_cross = max_seen_length_cross.min(max_length_cross);

    (
        minimizing_length_main,
        minimizing_length_cross,
    )
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
