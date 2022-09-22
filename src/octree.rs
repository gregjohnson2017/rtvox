use vecmath::Vector3;

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

    fn add_down(curr: &mut Box<Node<T>>, target: Box<Node<T>>) {
        if curr.aabc.size > 2 {
            let shrunken = curr.aabc.shrink_towards(target.aabc.origin);
            let n = Node::empty(shrunken.origin, shrunken.size);
            let idx = curr.add_child(n);
            match &mut curr.data {
                NodeData::Children(ref mut children) => match children[idx] {
                    Some(ref mut child) => Self::add_down(child, target),
                    None => unreachable!(),
                },
                NodeData::Value(_) => unreachable!(),
            }
        } else {
            curr.add_child(target);
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
                if node.aabc.contains(leaf.aabc.origin) {
                    match node.data {
                        NodeData::Value(_) => panic!("tried to replace leaf"),
                        NodeData::Children(ref children) => {
                            if children[node.get_child_idx(&leaf)].is_some() {
                                panic!("tried to replace leaf")
                            }
                        }
                    }
                }
                while !node.aabc.contains(leaf.aabc.origin) {
                    let expanded = node.aabc.expand_towards(leaf.aabc.origin);
                    let mut n = Node::empty(expanded.origin, expanded.size);
                    n.add_child(node);
                    node = n;
                }
                Self::add_down(&mut node, leaf);
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
            NodeData::Value(_) => assert!(false, "node was a leaf somehow"),
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
}
