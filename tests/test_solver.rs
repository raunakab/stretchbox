#[path = "../common/mod.rs"]
mod common;

use common::{make_frame_tree, make_solver};
use stretchbox::{Constraint, Fill, FillType, Frame};

#[test]
fn test_solver_with_empty_tree() {
    let mut solver = make_solver(None).unwrap();

    solver.solve(10., 10.);

    let actual_frame_tree = make_frame_tree(&solver);
    let expected_frame_tree = None;
    assert_eq!(actual_frame_tree, expected_frame_tree);
}

#[test]
fn test_solver_with_invalid_root_constraint() {
    let solver = make_solver(Some(
        &node! { Constraint { fill: Fill::Absolute { x: FillType::Exact(10.), y: FillType::Scale(1) }, ..Default::default() } },
    ));

    assert!(solver.is_none());
}

#[test]
fn test_solver_with_single_element_tree() {
    let mut solver = make_solver(Some(&node! { Constraint::default() })).unwrap();

    solver.solve(10., 10.);

    let actual_frame_tree = make_frame_tree(&solver);
    let expected_frame_tree =
        Some(node! { Frame { offset_x: 0., length_x: 10., offset_y: 0., length_y: 10. }});
    assert_eq!(actual_frame_tree, expected_frame_tree);
}
