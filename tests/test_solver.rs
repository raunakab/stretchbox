use stretchbox::{Constraint, Fill, Frame, Padding, Solver};

#[test]
fn test_solver() {
    let mut solver = Solver::default();

    assert!(!solver.is_dirty());

    let root_constraint_key = solver.insert_root(Constraint {
        fill_x: Fill::Scale(1),
        padding: Padding {
            left: 1.,
            right: 1.,
        },
    });
    let child_constraint_key_1 = solver
        .insert(
            Constraint {
                fill_x: Fill::Scale(1),
                padding: Padding {
                    left: 100.,
                    right: 100.,
                },
            },
            root_constraint_key,
        )
        .unwrap();
    let child_constraint_key_2 = solver
        .insert(
            Constraint {
                fill_x: Fill::Exact(10.),
                padding: Padding {
                    left: 100.,
                    right: 100.,
                },
            },
            root_constraint_key,
        )
        .unwrap();
    let child_constraint_key_3 = solver
        .insert(
            Constraint {
                fill_x: Fill::Minimize,
                padding: Padding {
                    left: 100.,
                    right: 100.,
                },
            },
            root_constraint_key,
        )
        .unwrap();

    assert!(solver.is_dirty());

    let did_solve = solver.solve(12.);
    assert!(did_solve);
    assert!(!solver.is_dirty());

    let root_frame = solver.get_frame(root_constraint_key).unwrap();
    let child_frame_1 = solver.get_frame(child_constraint_key_1).unwrap();
    let child_frame_2 = solver.get_frame(child_constraint_key_2).unwrap();
    let child_frame_3 = solver.get_frame(child_constraint_key_3).unwrap();

    assert_eq!(
        root_frame,
        Frame {
            offset_x: 0.,
            length_x: 12.
        }
    );
    assert_eq!(
        child_frame_1,
        Frame {
            offset_x: 1.,
            length_x: 0.
        }
    );
    assert_eq!(
        child_frame_2,
        Frame {
            offset_x: 1.,
            length_x: 10.
        }
    );
    assert_eq!(
        child_frame_3,
        Frame {
            offset_x: 11.,
            length_x: 0.
        }
    );
}
