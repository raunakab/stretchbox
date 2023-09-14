#[macro_export]
macro_rules! node {
    ($value:expr, [$($child:expr),*$(,)?]) => {{
        use common::DeclarativeNode;

        DeclarativeNode {
            value: $value,
            children: vec![$($child),*],
        }
    }};

    ($value:expr$(,)?) => {{
        use common::DeclarativeNode;

        DeclarativeNode {
            value: $value,
            children: vec![],
        }
    }};
}

use indexmap::IndexSet;
use stretchbox::{Constraint, ConstraintKey, Frame, Solver};

pub use node;

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct DeclarativeNode<V> {
    pub value: V,
    pub children: Vec<Self>,
}

pub fn make_solver(declarative_node: Option<&DeclarativeNode<Constraint>>) -> Option<Solver> {
    fn insert_root_node(
        mut solver: Solver,
        declarative_node: &DeclarativeNode<Constraint>,
    ) -> Option<Solver> {
        solver
            .insert_root(declarative_node.value)
            .map(|root_constraint_key| {
                insert_nodes(solver, &declarative_node.children, root_constraint_key)
            })
    }

    fn insert_nodes(
        mut solver: Solver,
        declarative_nodes: &Vec<DeclarativeNode<Constraint>>,
        parent_constraint_key: ConstraintKey,
    ) -> Solver {
        let mut to_visit_keys_and_nodes = vec![(parent_constraint_key, declarative_nodes)];

        while let Some((parent_constraint_key, declarative_nodes)) = to_visit_keys_and_nodes.pop() {
            for declarative_node in declarative_nodes {
                let constraint_key = solver
                    .insert(declarative_node.value, parent_constraint_key)
                    .unwrap();
                to_visit_keys_and_nodes.push((constraint_key, &declarative_node.children));
            }
        }

        solver
    }

    declarative_node.map_or_else(
        || Some(Solver::default()),
        |declarative_node| {
            let solver = Solver::default();
            insert_root_node(solver, declarative_node)
        },
    )
}

pub fn make_frame_tree(solver: &Solver) -> Option<DeclarativeNode<Frame>> {
    fn get_frame(solver: &Solver, constraint_key: ConstraintKey) -> DeclarativeNode<Frame> {
        let frame = solver.get_frame(constraint_key).unwrap();
        let child_constraint_keys = solver.get(constraint_key).unwrap().child_keys;
        let children = get_frames(solver, child_constraint_keys);

        DeclarativeNode {
            value: frame,
            children,
        }
    }

    fn get_frames(
        solver: &Solver,
        constraint_keys: &IndexSet<ConstraintKey>,
    ) -> Vec<DeclarativeNode<Frame>> {
        let mut frames = vec![];

        for &constraint_key in constraint_keys {
            let frame = get_frame(solver, constraint_key);
            frames.push(frame);
        }

        frames
    }

    solver
        .root_constraint_key()
        .and_then(|root_constraint_key| {
            let is_dirty = solver.is_dirty();

            if is_dirty {
                None
            } else {
                let frame_tree = get_frame(solver, root_constraint_key);
                Some(frame_tree)
            }
        })
}
