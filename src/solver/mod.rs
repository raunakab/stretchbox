use std::collections::BTreeMap;

use cherrytree::{Node, Tree};
use indexmap::IndexSet;

use crate::{
    Align, Constraint, ConstraintKey, Content, Direction, FillType, Frame,
    FrameKey, Padding,
};

pub(super) fn solve(
    constraint_tree: &Tree<ConstraintKey, Constraint>,
    frame_tree: &mut Tree<FrameKey, Frame>,
    key_map: &mut BTreeMap<ConstraintKey, FrameKey>,
    length_x: f64,
    length_y: f64,
) {
    let (root_constraint_key, root_constraint_node) = constraint_tree.root_key_value().unwrap();

    let absolute_fill = root_constraint_node
        .value
        .fill
        .to_absolute_fill(Direction::Vertical);

    let length_x = if let FillType::Scale(scale_x) = absolute_fill.x {
        match scale_x {
            0 => 0.,
            _ => length_x,
        }
    } else {
        unreachable!()
    };

    let length_y = if let FillType::Scale(scale_y) = absolute_fill.y {
        match scale_y {
            0 => 0.,
            _ => length_y,
        }
    } else {
        unreachable!()
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

    let root_content_frame = generate_content_frame(
        root_constraint_node.value.content.padding,
        length_x,
        length_y,
    );

    solve_child_keys(
        constraint_tree,
        frame_tree,
        key_map,
        root_constraint_node.child_keys,
        root_frame_key,
        root_content_frame,
        root_constraint_node.value.content,
    );
}

fn solve_child_keys(
    constraint_tree: &Tree<ConstraintKey, Constraint>,
    frame_tree: &mut Tree<FrameKey, Frame>,
    key_map: &mut BTreeMap<ConstraintKey, FrameKey>,
    constraint_keys: &IndexSet<ConstraintKey>,
    parent_frame_key: FrameKey,
    content_frame: Frame,
    parent_content: Content,
) {
    match parent_content.direction {
        Direction::Horizontal => {
            let mut remaining_length_x = content_frame.length_x;
            let mut total_scale_x: usize = 0;

            let mut lengths = iter(constraint_tree, constraint_keys)
                .map(|(_, constraint_node)| {
                    let absolute_fill = constraint_node.value.fill.to_absolute_fill_horizontal();
                    let mut minimizing_length_y_cache = None;

                    let length_x = match absolute_fill.x {
                        FillType::Exact(exact_x) => {
                            let exact_x = exact_x.min(remaining_length_x);
                            remaining_length_x -= exact_x;
                            Some(exact_x)
                        }
                        FillType::Scale(scale_x) => {
                            total_scale_x = total_scale_x.checked_add(scale_x).unwrap();
                            None
                        }
                        FillType::Minimize => {
                            let (minimizing_length_x, minimizing_length_y) = find_minimizing_length(constraint_tree, constraint_node.child_keys, constraint_node.value.content.direction, remaining_length_x, content_frame.length_y);
                            minimizing_length_y_cache = Some(minimizing_length_y);
                            remaining_length_x -= minimizing_length_x;
                            Some(minimizing_length_x)
                        },
                    };

                    let length_y = match absolute_fill.y {
                        FillType::Exact(exact_y) => exact_y.min(content_frame.length_y),
                        FillType::Scale(scale_y) => match scale_y {
                            0 => 0.,
                            _ => content_frame.length_y,
                        },
                        FillType::Minimize => {
                            minimizing_length_y_cache.unwrap_or_else(|| {
                                let (_, minimizing_length_y) = find_minimizing_length(constraint_tree, constraint_node.child_keys, constraint_node.value.content.direction, remaining_length_x, content_frame.length_y);
                                minimizing_length_y
                            })
                        },
                    };

                    let remaining_length_y = content_frame.length_y - length_y;
                    let offset_y = content_frame.offset_y
                        + if remaining_length_y == 0. {
                            0.
                        } else {
                            match parent_content.align_cross {
                                Align::Start => 0.,
                                Align::Middle => remaining_length_y / 2.,
                                Align::End => remaining_length_y,
                            }
                        };

                    (absolute_fill, length_x, length_y, offset_y)
                })
                .collect::<Vec<_>>();

            let offset_x = content_frame.offset_x
                + match total_scale_x {
                    0 => {
                        if remaining_length_x == 0. {
                            0.
                        } else {
                            match parent_content.align_main {
                                Align::Start => 0.,
                                Align::Middle => remaining_length_x / 2.,
                                Align::End => remaining_length_x,
                            }
                        }
                    }
                    _ => {
                        for (absolute_fill, length_x, _, _) in &mut lengths {
                            if let FillType::Scale(scale_x) = absolute_fill.x {
                                let proportion = (scale_x as f64) / (total_scale_x as f64);
                                *length_x = Some(proportion * remaining_length_x);
                            };
                        }

                        0.
                    }
                };

            for ((constraint_key, consraint_node), (_, length_x, length_y, offset_y)) in
                iter(constraint_tree, constraint_keys).zip(lengths)
            {
                let length_x = length_x.unwrap_or_default();

                let frame = Frame {
                    offset_x,
                    length_x,
                    offset_y,
                    length_y,
                };

                let number_of_child_keys = consraint_node.child_keys.len();
                let frame_key = frame_tree
                    .insert_with_capacity(frame, parent_frame_key, number_of_child_keys)
                    .unwrap();
                key_map.insert(constraint_key, frame_key);

                let content_frame = generate_content_frame(
                    consraint_node.value.content.padding,
                    frame.length_x,
                    frame.length_y,
                );

                solve_child_keys(
                    constraint_tree,
                    frame_tree,
                    key_map,
                    consraint_node.child_keys,
                    frame_key,
                    content_frame,
                    consraint_node.value.content,
                );
            }
        }
        Direction::Vertical => todo!(),
    }
}

fn find_minimizing_length(
    constraint_tree: &Tree<ConstraintKey, Constraint>,
    constraint_keys: &IndexSet<ConstraintKey>,
    direction: Direction,
    max_length_x: f64,
    max_length_y: f64,
) -> (f64, f64) {
    match direction {
        Direction::Horizontal => {
            let mut remaining_length_x: f64 = max_length_x;
            let mut max_seen_length_y: f64 = 0.;

            for (_, constraint_node) in iter(constraint_tree, constraint_keys) {
                let Padding { left, right, top, bottom } = constraint_node.value.content.padding;

                let absolute_fill = constraint_node.value.fill.to_absolute_fill_horizontal();

                let mut sub_minimizing_length_y_cache = None;

                let length_x = match absolute_fill.x {
                    FillType::Exact(exact_x) => exact_x + left + right,

                    FillType::Scale(..) => left + right,

                    FillType::Minimize => {
                        let (sub_minimizing_length_x, sub_minimizing_length_y) = find_minimizing_length(constraint_tree, constraint_keys, constraint_node.value.content.direction, remaining_length_x, max_length_y);

                        sub_minimizing_length_y_cache = Some(sub_minimizing_length_y);

                        sub_minimizing_length_x + left + right
                    },
                }.min(remaining_length_x);

                let length_y = match absolute_fill.y {
                    FillType::Exact(exact_y) => exact_y + top + bottom,
                    FillType::Scale(..) => top + bottom,
                    FillType::Minimize => sub_minimizing_length_y_cache.unwrap_or_else(|| {
                        let (_, sub_minimizing_length_y) = find_minimizing_length(constraint_tree, constraint_keys, constraint_node.value.content.direction, remaining_length_x, max_length_y);
                        sub_minimizing_length_y
                    }),
                };

                remaining_length_x -= length_x;
                max_seen_length_y = max_seen_length_y.max(length_y);
            }

            let minimizing_length_x = max_length_x - remaining_length_x;
            let minimizing_length_y = max_seen_length_y.min(max_length_y);

            (
                minimizing_length_x,
                minimizing_length_y,
            )
        },

        Direction::Vertical => todo!(),
    }
}

fn generate_content_frame(padding: Padding, length_x: f64, length_y: f64) -> Frame {
    let content_start_x = padding.left.min(length_x);
    let content_end_x = (length_x - padding.right).max(0.);
    let content_length_x = (content_end_x - content_start_x).max(0.);

    let content_start_y = padding.top.min(length_y);
    let content_end_y = (length_y - padding.bottom).max(0.);
    let content_length_y = (content_end_y - content_start_y).max(0.);

    Frame {
        offset_x: content_start_x,
        length_x: content_length_x,

        offset_y: content_start_y,
        length_y: content_length_y,
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
