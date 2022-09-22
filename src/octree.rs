use vecmath::{vec3_add, Vector3};

use crate::aabc::Aabc;

pub struct Octree<T: Copy> {
    root: Option<Box<Node<T>>>,
}

#[derive(PartialEq, Debug)]
struct Node<T: Copy> {
    data: NodeData<T>,
    aabc: Aabc,
}

#[derive(PartialEq, Debug)]
enum NodeData<T: Copy> {
    Children([Option<Box<Node<T>>>; 8]),
    Value(T),
}

impl<T: Copy> Clone for Box<Node<T>> {
    fn clone(&self) -> Self {
        match &self.data {
            NodeData::Children(children) => {
                let mut new_children = [None, None, None, None, None, None, None, None];
                for i in 0..children.len() {
                    match &children[i] {
                        Some(child) => new_children[i] = Some(child.clone()),
                        _ => (),
                    }
                }
                Box::new(Node {
                    data: NodeData::Children(new_children),
                    aabc: self.aabc,
                })
            }
            NodeData::Value(v) => Box::new(Node {
                data: NodeData::Value(*v),
                aabc: self.aabc,
            }),
        }
    }
}

impl<T: Copy> Node<T> {
    fn empty(origin: Vector3<i32>, size: u32) -> Box<Node<T>> {
        Box::new(Node {
            data: NodeData::Children([None, None, None, None, None, None, None, None]),
            aabc: Aabc { origin, size },
        })
    }

    pub fn new_leaf(data: T, pos: Vector3<i32>) -> Box<Node<T>> {
        Box::new(Node {
            data: NodeData::Value(data),
            aabc: Aabc {
                origin: pos,
                size: 1,
            },
        })
    }

    fn get_child_idx(&self, child: &Box<Node<T>>) -> usize {
        let min = self.aabc.origin;
        let p = child.aabc.origin;
        let off = child.aabc.size as i32;
        if p == [min[0], min[1], min[2]] {
            return 6;
        } else if p == [min[0] + off, min[1], min[2]] {
            return 7;
        } else if p == [min[0], min[1] + off, min[2]] {
            return 5;
        } else if p == [min[0] + off, min[1] + off, min[2]] {
            return 4;
        } else if p == [min[0], min[1], min[2] + off] {
            return 2;
        } else if p == [min[0] + off, min[1], min[2] + off] {
            return 3;
        } else if p == [min[0], min[1] + off, min[2] + off] {
            return 1;
        } else if p == [min[0] + off, min[1] + off, min[2] + off] {
            return 0;
        } else {
            panic!("child misaligned");
        }
    }

    fn octant_contains(&self, offs: [bool; 3], target_aabc: Aabc) -> bool {
        let half = (self.aabc.size / 2) as i32;
        let mut off = [0, 0, 0];
        for i in 0..3 {
            if offs[i] {
                off[i] = half;
            }
        }
        let octant = Aabc {
            origin: vec3_add(self.aabc.origin, off),
            size: self.aabc.size / 2,
        };
        return octant.contains_aabc(target_aabc);
    }

    fn get_octant_idx(&self, target: Aabc) -> usize {
        if self.octant_contains([false, false, false], target) {
            return 6;
        } else if self.octant_contains([true, false, false], target) {
            return 7;
        } else if self.octant_contains([false, true, false], target) {
            return 5;
        } else if self.octant_contains([true, true, false], target) {
            return 4;
        } else if self.octant_contains([false, false, true], target) {
            return 2;
        } else if self.octant_contains([true, false, true], target) {
            return 3;
        } else if self.octant_contains([false, true, true], target) {
            return 1;
        } else if self.octant_contains([true, true, true], target) {
            return 0;
        } else {
            panic!("target not contained within any octant");
        }
    }

    fn count_children(&self) -> (u32, Option<usize>) {
        let mut idx = None;
        let mut assigned = false;
        match &self.data {
            NodeData::Value(_) => (0, None),
            NodeData::Children(children) => {
                let mut n = 0;
                for i in 0..children.len() {
                    if children[i].is_some() {
                        n += 1;
                        if !assigned {
                            assigned = true;
                            idx = Some(i);
                        } else {
                            idx = None
                        }
                    }
                }
                return (n, idx);
            }
        }
    }

    fn remove_child(&mut self, leaf: Box<Node<T>>) -> bool {
        let idx = self.get_octant_idx(leaf.aabc);
        match &mut self.data {
            NodeData::Children(ref mut children) => match children[idx] {
                Some(ref mut node) if node.aabc == leaf.aabc => {
                    children[idx] = None;
                    self.count_children().0 == 0
                }
                Some(ref mut node) => {
                    let remove_node = node.remove_child(leaf);
                    if remove_node {
                        children[idx] = None;
                    }
                    self.count_children().0 == 0
                }
                None => panic!("leaf not found"),
            },
            NodeData::Value(_) => panic!("????"),
        }
    }

    fn add_down(&mut self, target: Box<Node<T>>) {
        if self.aabc.size > 2 {
            let shrunken = self.aabc.shrink_towards(target.aabc.origin);
            let n = Node::empty(shrunken.origin, shrunken.size);
            let idx = self.add_child(n);
            match &mut self.data {
                NodeData::Children(ref mut children) => match children[idx] {
                    Some(ref mut child) => Self::add_down(child, target),
                    None => unreachable!(),
                },
                NodeData::Value(_) => unreachable!(),
            }
        } else {
            self.add_child(target);
        }
    }

    fn add_child(&mut self, child: Box<Node<T>>) -> usize {
        if !self.aabc.contains(child.aabc.origin) {
            panic!("child outside parent");
        }
        if self.aabc.size != child.aabc.size * 2 {
            panic!("parent not twice as big as child");
        }
        let idx = self.get_child_idx(&child);
        match self.data {
            NodeData::Children(ref mut children) => {
                if children[idx].is_some() {
                    panic!("attempted to overwrite leaf at {:?}", child.aabc)
                }
                children[idx] = Some(child);
                idx
            }
            NodeData::Value(_) => panic!("cannot add a child to a leaf node"),
        }
    }
}

impl<T: Copy> Octree<T> {
    pub fn new() -> Self {
        Octree { root: None }
    }

    fn shrink_root(&mut self) {
        match self.root {
            Some(ref mut root_node) => {
                let (n, i) = root_node.count_children();
                match root_node.data {
                    NodeData::Value(_) => (),
                    NodeData::Children(ref mut children) => {
                        if n == 1 {
                            self.root = std::mem::replace(&mut children[i.unwrap()], None);
                            self.shrink_root();
                        }
                    }
                }
            }
            None => panic!("root is none"),
        }
    }

    pub fn remove_leaf(&mut self, leaf: Box<Node<T>>) {
        match self.root {
            None => panic!("cannot remove from empty tree"),
            Some(ref mut node) => {
                if node.aabc == leaf.aabc {
                    self.root = None
                } else {
                    let remove_node = node.remove_child(leaf);
                    if remove_node {
                        self.root = None
                    } else {
                        self.shrink_root()
                    }
                }
            }
        }
    }

    pub fn insert_leaf(&mut self, leaf: Box<Node<T>>) {
        match leaf.data {
            NodeData::Children(_) => panic!("leaf had children"),
            NodeData::Value(_) if leaf.aabc.size != 1 => {
                panic!("leaf was not size 1")
            }
            _ => (),
        }

        let root = std::mem::replace(&mut self.root, None);
        match root {
            None => self.root = Some(leaf),
            Some(mut node) => {
                while !node.aabc.contains(leaf.aabc.origin) {
                    let expanded = node.aabc.expand_towards(leaf.aabc.origin);
                    let mut n = Node::empty(expanded.origin, expanded.size);
                    n.add_child(node);
                    node = n;
                }
                node.add_down(leaf);
                self.root = Some(node);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{aabc::Aabc, octree::Node};

    use super::*;

    #[test]
    fn insert_leaf() {
        let mut tree = Octree::new();
        let expected_leaf = Node::new_leaf(0, [0, 0, 0]);
        tree.insert_leaf(expected_leaf.clone());
        assert_eq!(tree.root, Some(expected_leaf))
    }

    #[test]
    #[should_panic]
    fn insert_fake_leaf_panics() {
        let mut tree: Octree<i32> = Octree::new();
        let expected_leaf = Box::new(Node {
            data: NodeData::Children([None, None, None, None, None, None, None, None]),
            aabc: Aabc {
                origin: [0, 0, 0],
                size: 1,
            },
        });
        tree.insert_leaf(expected_leaf.clone());
        assert_eq!(tree.root, Some(expected_leaf))
    }

    #[test]
    #[should_panic]
    fn insert_large_leaf_panics() {
        let mut tree: Octree<i32> = Octree::new();
        let expected_leaf = Box::new(Node {
            data: NodeData::Value(2),
            aabc: Aabc {
                origin: [0, 0, 0],
                size: 2,
            },
        });
        tree.insert_leaf(expected_leaf.clone());
        assert_eq!(tree.root, Some(expected_leaf))
    }

    #[test]
    #[should_panic]
    fn insert_duplicate_leaf_panics() {
        let mut tree = Octree::new();
        tree.insert_leaf(Node::new_leaf(0, [0, 0, 0]));
        tree.insert_leaf(Node::new_leaf(0, [0, 0, 0]));
    }

    #[test]
    #[should_panic]
    fn insert_duplicate_leaf_panics_2() {
        let mut tree = Octree::new();
        tree.insert_leaf(Node::new_leaf(0, [0, 0, 0]));
        tree.insert_leaf(Node::new_leaf(0, [2, 2, 2]));
        tree.insert_leaf(Node::new_leaf(0, [0, 0, 0]));
    }

    #[test]
    #[should_panic]
    fn add_leaf_outside_node_panics() {
        let mut node = Node::empty([0, 0, 0], 2);
        node.add_child(Node::new_leaf(0, [2, 2, 2]));
    }

    #[test]
    #[should_panic]
    fn add_leaf_to_large_node_panics() {
        let mut node = Node::empty([0, 0, 0], 4);
        node.add_child(Node::new_leaf(0, [0, 0, 0]));
    }

    #[test]
    #[should_panic]
    fn add_missized_child_panics() {
        let mut node: Box<Node<i32>> = Node::empty([0, 0, 0], 8);
        node.add_child(Node::empty([0, 0, 0], 2));
    }

    #[test]
    #[should_panic]
    fn add_child_node_outside_node_panics() {
        let mut node: Box<Node<i32>> = Node::empty([0, 0, 0], 4);
        node.add_child(Node::empty([4, 4, 4], 2));
    }

    #[test]
    fn add_children_leaves_to_node() {
        let mut node = Node::empty([0, 0, 0], 2);
        let expected_children = [
            Some(Node::new_leaf(0, [1, 1, 1])),
            Some(Node::new_leaf(0, [0, 1, 1])),
            Some(Node::new_leaf(0, [0, 0, 1])),
            Some(Node::new_leaf(0, [1, 0, 1])),
            Some(Node::new_leaf(0, [1, 1, 0])),
            Some(Node::new_leaf(0, [0, 1, 0])),
            Some(Node::new_leaf(0, [0, 0, 0])),
            Some(Node::new_leaf(0, [1, 0, 0])),
        ];
        for i in 0..expected_children.len() {
            node.add_child(expected_children[i].clone().unwrap());
        }
        assert_eq!(NodeData::Children(expected_children), node.data)
    }

    #[test]
    fn add_child_nodes_to_node() {
        let mut node: Box<Node<i32>> = Node::empty([0, 0, 0], 4);
        let expected_aabcs = [
            Aabc {
                origin: [2, 2, 2],
                size: 2,
            },
            Aabc {
                origin: [0, 2, 2],
                size: 2,
            },
            Aabc {
                origin: [0, 0, 2],
                size: 2,
            },
            Aabc {
                origin: [2, 0, 2],
                size: 2,
            },
            Aabc {
                origin: [2, 2, 0],
                size: 2,
            },
            Aabc {
                origin: [0, 2, 0],
                size: 2,
            },
            Aabc {
                origin: [0, 0, 0],
                size: 2,
            },
            Aabc {
                origin: [2, 0, 0],
                size: 2,
            },
        ];
        for i in 0..expected_aabcs.len() {
            node.add_child(Node::empty(expected_aabcs[i].origin, 2));
        }
        match node.data {
            NodeData::Children(arr) => {
                for i in 0..expected_aabcs.len() {
                    assert_eq!(expected_aabcs[i], arr[i].clone().unwrap().aabc)
                }
            }
            NodeData::Value(_) => unreachable!(),
        }
    }

    #[test]
    fn insert_two_leaves() {
        let mut tree = Octree::new();
        let leaf1 = Node::new_leaf(0, [0, 0, 0]);
        let leaf2 = Node::new_leaf(1, [1, 0, 0]);
        tree.insert_leaf(leaf1.clone());
        tree.insert_leaf(leaf2.clone());
        let mut expected_node = Node::empty([0, 0, 0], 2);
        expected_node.data =
            NodeData::Children([None, None, None, None, None, None, Some(leaf1), Some(leaf2)]);

        assert_eq!(tree.root, Some(expected_node));
    }

    #[test]
    #[should_panic]
    fn remove_leaf_empty_tree_panics() {
        let mut tree = Octree::new();
        tree.remove_leaf(Node::new_leaf(0, [0, 0, 0]));
    }

    #[test]
    #[should_panic]
    fn remove_unknown_leaf_panics() {
        let mut tree = Octree::new();
        tree.insert_leaf(Node::new_leaf(0, [0, 0, 0]));
        tree.insert_leaf(Node::new_leaf(0, [1, 0, 0]));
        tree.remove_leaf(Node::new_leaf(0, [1, 1, 1]));
    }

    #[test]
    fn insert_and_remove_leaf() {
        let mut tree = Octree::new();
        let leaf = Node::new_leaf(0, [0, 0, 0]);
        tree.insert_leaf(leaf.clone());
        tree.remove_leaf(leaf);
        assert!(tree.root.is_none());
    }

    #[test]
    fn insert_2_and_remove_1_leaf() {
        let mut tree = Octree::new();
        let leaf1 = Node::new_leaf(0, [0, 0, 0]);
        let leaf2 = Node::new_leaf(0, [1, 1, 1]);
        tree.insert_leaf(leaf1.clone());
        tree.insert_leaf(leaf2.clone());
        tree.remove_leaf(leaf1);
        assert_eq!(tree.root, Some(leaf2));
    }

    #[test]
    fn complex_insert_remove() {
        let mut tree = Octree::new();
        let leaf1 = Node::new_leaf(0, [0, 0, 0]);
        let leaf2 = Node::new_leaf(0, [1, 1, 1]);
        let leaf3 = Node::new_leaf(0, [2, 2, 2]);
        tree.insert_leaf(leaf1.clone());
        tree.insert_leaf(leaf2.clone());
        tree.insert_leaf(leaf3.clone());
        tree.remove_leaf(leaf1);
        let leaf4 = Node::new_leaf(5, [1, 2, 2]);
        tree.insert_leaf(leaf4.clone());
        tree.remove_leaf(leaf2);

        let expected_root = Box::new(Node {
            data: NodeData::Children([
                Some(Box::new(Node {
                    data: NodeData::Children([
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        Some(leaf3),
                        None,
                    ]),
                    aabc: Aabc {
                        origin: [2, 2, 2],
                        size: 2,
                    },
                })),
                Some(Box::new(Node {
                    data: NodeData::Children([
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        Some(leaf4),
                    ]),
                    aabc: Aabc {
                        origin: [0, 2, 2],
                        size: 2,
                    },
                })),
                None,
                None,
                None,
                None,
                None,
                None,
            ]),
            aabc: Aabc {
                origin: [0, 0, 0],
                size: 4,
            },
        });
        assert_eq!(tree.root, Some(expected_root));
    }
}
