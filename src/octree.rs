use vecmath::{vec3_add, Vector3};

use crate::aabc::Aabc;

pub struct Octree<T: Copy + Into<i32>> {
    n_leaves: u32,
    root: Option<Box<Node<T>>>,
}

#[derive(PartialEq, Debug)]
struct Node<T: Copy + Into<i32>> {
    data: NodeData<T>,
    aabc: Aabc,
}

#[derive(PartialEq, Debug)]
enum NodeData<T: Copy + Into<i32>> {
    Children([Option<Box<Node<T>>>; 8]),
    Value(T),
}

impl<T: Copy + Into<i32>> Clone for Box<Node<T>> {
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

impl<T: Copy + Into<i32>> Node<T> {
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

    fn get_octant_idx(&self, target: Aabc) -> usize {
        fn octant_contains(offs: [bool; 3], target: Aabc, parent: Aabc) -> bool {
            let half = (parent.size / 2) as i32;
            let mut off = [0, 0, 0];
            for i in 0..3 {
                if offs[i] {
                    off[i] = half;
                }
            }
            let octant = Aabc {
                origin: vec3_add(parent.origin, off),
                size: parent.size / 2,
            };
            return octant.contains_aabc(target);
        }
        if octant_contains([false, false, false], target, self.aabc) {
            return 6;
        } else if octant_contains([true, false, false], target, self.aabc) {
            return 5;
        } else if octant_contains([false, true, false], target, self.aabc) {
            return 2;
        } else if octant_contains([true, true, false], target, self.aabc) {
            return 1;
        } else if octant_contains([false, false, true], target, self.aabc) {
            return 7;
        } else if octant_contains([true, false, true], target, self.aabc) {
            return 4;
        } else if octant_contains([false, true, true], target, self.aabc) {
            return 3;
        } else if octant_contains([true, true, true], target, self.aabc) {
            return 0;
        } else {
            panic!("target not contained within any octant");
        }
    }

    // returns the number of children, and if there was only 1, its index
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

    fn remove_child(&mut self, target: Aabc) -> bool {
        let idx = self.get_octant_idx(target);
        match &mut self.data {
            NodeData::Children(ref mut children) => match children[idx] {
                Some(ref mut node) if node.aabc == target => {
                    children[idx] = None;
                    self.count_children().0 == 0
                }
                Some(ref mut node) => {
                    let remove_node = node.remove_child(target);
                    if remove_node {
                        children[idx] = None;
                    }
                    self.count_children().0 == 0
                }
                None => panic!("child not found"),
            },
            NodeData::Value(_) => panic!("????"),
        }
    }

    fn add_down(&mut self, target_leaf: Box<Node<T>>) {
        if self.aabc.size > 2 {
            let idx = self.get_octant_idx(target_leaf.aabc);
            match &mut self.data {
                NodeData::Children(ref mut children) => match children[idx] {
                    Some(ref mut child) => Self::add_down(child, target_leaf),
                    None => {
                        let shrunken = self.aabc.shrink_towards(target_leaf.aabc.origin);
                        let n = Node::empty(shrunken.origin, shrunken.size);
                        let idx2 = self.add_child(n);
                        // TODO how to do this in a smarter way
                        match &mut self.data {
                            NodeData::Children(ref mut children) => match children[idx2] {
                                Some(ref mut child) => Self::add_down(child, target_leaf),
                                None => unreachable!(),
                            },
                            NodeData::Value(_) => unreachable!(),
                        }
                    }
                },
                NodeData::Value(_) => unreachable!(),
            }
        } else {
            self.add_child(target_leaf);
        }
    }

    fn add_child(&mut self, child: Box<Node<T>>) -> usize {
        if !self.aabc.contains(child.aabc.origin) {
            panic!("child outside parent");
        }
        if self.aabc.size != child.aabc.size * 2 {
            panic!("parent not twice as big as child");
        }
        let idx = self.get_octant_idx(child.aabc);
        match self.data {
            NodeData::Children(ref mut children) => {
                if children[idx].is_some() {
                    panic!("attempted to overwrite child at {:?}", child.aabc)
                }
                children[idx] = Some(child);
                idx
            }
            NodeData::Value(_) => panic!("cannot add a child to a leaf node"),
        }
    }
}

impl<T: Copy + Into<i32>> Octree<T> {
    pub fn new() -> Self {
        Octree {
            n_leaves: 0,
            root: None,
        }
    }

    fn get_size_recurse(node: &Box<Node<T>>) -> usize {
        match &node.data {
            NodeData::Children(children) => {
                let mut count = 8;
                for child in children {
                    match child {
                        Some(ref c) => count += Self::get_size_recurse(c),
                        None => (),
                    }
                }
                count
            }
            NodeData::Value(_) => 0,
        }
    }

    fn get_serialized_size(&self) -> usize {
        match &self.root {
            Some(r) => 4 + Self::get_size_recurse(r),
            None => 1,
        }
    }

    pub fn count_leaves(&self) -> u32 {
        self.n_leaves
    }

    fn serialize_recurse(idx: usize, arr: &mut Vec<i32>, curr: &Box<Node<T>>) -> usize {
        match &curr.data {
            NodeData::Children(children) => {
                let mut start = idx + 8;
                if curr.aabc.size == 2 {
                    for i in 0..children.len() {
                        match &children[i] {
                            Some(c) => match c.data {
                                NodeData::Children(_) => unreachable!(),
                                NodeData::Value(d) => arr[idx + i] = d.clone().into(),
                            },
                            None => (),
                        }
                    }
                } else {
                    for i in 0..children.len() {
                        match &children[i] {
                            Some(c) => {
                                arr[idx + i] = start as i32;
                                start += Self::serialize_recurse(start, arr, c)
                            }
                            None => (),
                        }
                    }
                }
                start - idx
            }
            NodeData::Value(_) => panic!("single leaf tree not supported"),
        }
    }

    pub fn serialize(&self) -> Vec<i32> {
        let mut arr = vec![0 as i32; self.get_serialized_size()];
        match &self.root {
            Some(n) => {
                arr[0] = n.aabc.size as i32;
                arr[1] = n.aabc.origin[0];
                arr[2] = n.aabc.origin[1];
                arr[3] = n.aabc.origin[2];
                Self::serialize_recurse(4, &mut arr, n);
                arr
            }
            None => arr,
        }
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

    pub fn remove_leaf(&mut self, target: Vector3<i32>) {
        self.n_leaves -= 1;
        match self.root {
            None => panic!("cannot remove from empty tree"),
            Some(ref mut node) => {
                let target = Aabc::new(target, 1);
                if node.aabc == target {
                    self.root = None
                } else {
                    let remove_node = node.remove_child(target);
                    if remove_node {
                        self.root = None
                    } else {
                        self.shrink_root()
                    }
                }
            }
        }
    }

    pub fn insert_leaf(&mut self, data: T, pos: Vector3<i32>) {
        self.n_leaves += 1;
        let leaf = Node::new_leaf(data, pos);
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
        let expected_root = Node::new_leaf(0, [0, 0, 0]);
        tree.insert_leaf(0, [0, 0, 0]);
        assert_eq!(tree.root, Some(expected_root))
    }

    #[test]
    #[should_panic]
    fn insert_duplicate_leaf_panics() {
        let mut tree = Octree::new();
        tree.insert_leaf(0, [0, 0, 0]);
        tree.insert_leaf(0, [0, 0, 0]);
    }

    #[test]
    #[should_panic]
    fn insert_duplicate_leaf_panics_2() {
        let mut tree = Octree::new();
        tree.insert_leaf(0, [0, 0, 0]);
        tree.insert_leaf(0, [2, 2, 2]);
        tree.insert_leaf(0, [0, 0, 0]);
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
            Some(Node::new_leaf(0, [1, 1, 0])),
            Some(Node::new_leaf(0, [0, 1, 0])),
            Some(Node::new_leaf(0, [0, 1, 1])),
            Some(Node::new_leaf(0, [1, 0, 1])),
            Some(Node::new_leaf(0, [1, 0, 0])),
            Some(Node::new_leaf(0, [0, 0, 0])),
            Some(Node::new_leaf(0, [0, 0, 1])),
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
                origin: [2, 2, 0],
                size: 2,
            },
            Aabc {
                origin: [0, 2, 0],
                size: 2,
            },
            Aabc {
                origin: [0, 2, 2],
                size: 2,
            },
            Aabc {
                origin: [2, 0, 2],
                size: 2,
            },
            Aabc {
                origin: [2, 0, 0],
                size: 2,
            },
            Aabc {
                origin: [0, 0, 0],
                size: 2,
            },
            Aabc {
                origin: [0, 0, 2],
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
        tree.insert_leaf(0, [0, 0, 0]);
        tree.insert_leaf(1, [1, 0, 0]);
        let mut expected_node = Node::empty([0, 0, 0], 2);
        expected_node.data =
            NodeData::Children([None, None, None, None, None, Some(leaf2), Some(leaf1), None]);

        assert_eq!(tree.root, Some(expected_node));
    }

    #[test]
    #[should_panic]
    fn remove_leaf_empty_tree_panics() {
        let mut tree: Octree<i32> = Octree::new();
        tree.remove_leaf([0, 0, 0]);
    }

    #[test]
    #[should_panic]
    fn remove_unknown_leaf_panics() {
        let mut tree = Octree::new();
        tree.insert_leaf(0, [0, 0, 0]);
        tree.insert_leaf(0, [1, 0, 0]);
        tree.remove_leaf([1, 1, 1]);
    }

    #[test]
    fn insert_and_remove_leaf() {
        let mut tree = Octree::new();
        tree.insert_leaf(0, [0, 0, 0]);
        tree.remove_leaf([0, 0, 0]);
        assert!(tree.root.is_none());
    }

    #[test]
    fn insert_2_and_remove_1_leaf() {
        let mut tree = Octree::new();
        tree.insert_leaf(0, [0, 0, 0]);
        tree.insert_leaf(0, [1, 1, 1]);
        tree.remove_leaf([0, 0, 0]);
        assert_eq!(tree.root, Some(Node::new_leaf(0, [1, 1, 1])));
    }

    #[test]
    fn complex_insert_remove() {
        let mut tree = Octree::new();
        let leaf3 = Node::new_leaf(0, [2, 2, 2]);
        tree.insert_leaf(0, [0, 0, 0]);
        tree.insert_leaf(0, [1, 1, 1]);
        tree.insert_leaf(0, leaf3.aabc.origin);
        tree.remove_leaf([0, 0, 0]);
        let leaf4 = Node::new_leaf(5, [2, 2, 1]);
        tree.insert_leaf(5, leaf4.aabc.origin);
        tree.remove_leaf([1, 1, 1]);

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
                        origin: [2, 2, 0],
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

    #[test]
    fn count_leaves_empty_tree() {
        let tree: Octree<bool> = Octree::new();
        let expected_count = 0;
        assert_eq!(expected_count, tree.count_leaves());
    }

    #[test]
    fn count_inserted_leaves() {
        let mut tree = Octree::new();
        let leaf1 = Node::new_leaf(0, [0, 0, 0]);
        let leaf2 = Node::new_leaf(1, [1, 0, 0]);
        let leaf3 = Node::new_leaf(2, [1, 1, 0]);
        tree.insert_leaf(0, leaf1.aabc.origin);
        tree.insert_leaf(0, leaf2.aabc.origin);
        tree.insert_leaf(0, leaf3.aabc.origin);
        let expected_count = 3;
        assert_eq!(expected_count, tree.count_leaves());
    }

    #[test]
    fn count_insert_remove() {
        let mut tree = Octree::new();
        let leaf1 = Node::new_leaf(0, [0, 0, 0]);
        let leaf2 = Node::new_leaf(1, [1, 0, 0]);
        tree.insert_leaf(0, leaf1.aabc.origin);
        tree.insert_leaf(0, leaf2.aabc.origin);
        tree.remove_leaf(leaf2.aabc.origin);
        let expected_count = 1;
        assert_eq!(expected_count, tree.count_leaves());
    }

    #[test]
    fn serialize_empty_tree() {
        let tree: Octree<bool> = Octree::new();
        let expected = vec![0];
        assert_eq!(expected, tree.serialize());
    }

    #[test]
    fn serialize_size_2_tree() {
        let mut tree: Octree<i32> = Octree::new();
        tree.insert_leaf(1, [0, 0, 0]);
        tree.insert_leaf(2, [1, 1, 1]);

        let expected = vec![2, 0, 0, 0, 2, 0, 0, 0, 0, 0, 1, 0];
        assert_eq!(expected, tree.serialize());
    }

    #[test]
    fn serialize_size_2_tree_b() {
        let mut tree: Octree<i32> = Octree::new();
        tree.insert_leaf(1, [0, 0, -5]);
        tree.insert_leaf(2, [1, 0, -5]);

        let expected = vec![2, 0, 0, -5, 0, 0, 0, 0, 0, 2, 1, 0];
        assert_eq!(expected, tree.serialize());
    }

    #[test]
    fn serialize_size_4_tree() {
        let mut tree: Octree<i32> = Octree::new();
        tree.insert_leaf(1, [0, 0, 0]);
        tree.insert_leaf(2, [1, 1, 1]);
        tree.insert_leaf(3, [-1, -1, -1]);

        let expected = vec![
            4, // root size
            -2, -2, -2, // xyz
            12, 0, 0, 0, 0, 0, 20, 0, // size 4's children indices
            2, 0, 0, 0, 0, 0, 1, 0, // size 2's children leaf block type
            3, 0, 0, 0, 0, 0, 0, 0, // size 2's children leaf block type
        ];
        assert_eq!(expected, tree.serialize());
    }

    #[test]
    fn serialize_size_8_tree() {
        let mut tree: Octree<i32> = Octree::new();
        tree.insert_leaf(1, [0, 0, 0]);
        tree.insert_leaf(2, [1, 1, 1]);
        tree.insert_leaf(3, [2, 2, 2]);
        tree.insert_leaf(4, [4, 4, 4]);

        let expected = vec![
            8, // root size
            0, 0, 0, // xyz
            12, 0, 0, 0, 0, 0, 28, 0, // size 8's children indices
            0, 0, 0, 0, 0, 0, 20, 0, // size 4's children indices
            0, 0, 0, 0, 0, 0, 4, 0, // size 2's children leaf block type
            36, 0, 0, 0, 0, 0, 44, 0, // size 4's children indices
            0, 0, 0, 0, 0, 0, 3, 0, // size 2's children leaf block type
            2, 0, 0, 0, 0, 0, 1, 0, // size 2's children leaf block type
        ];
        assert_eq!(expected, tree.serialize());
    }

    #[test]
    fn get_size_serialize_empty_tree() {
        let tree: Octree<bool> = Octree::new();
        assert_eq!(1, tree.get_serialized_size());
    }

    #[test]
    fn get_size_serialize_size_2_tree() {
        let mut tree: Octree<i32> = Octree::new();
        tree.insert_leaf(1, [0, 0, 0]);
        tree.insert_leaf(2, [1, 1, 1]);
        assert_eq!(12, tree.get_serialized_size());
    }

    #[test]
    fn get_size_serialize_size_4_tree() {
        let mut tree: Octree<i32> = Octree::new();
        tree.insert_leaf(1, [0, 0, 0]);
        tree.insert_leaf(2, [1, 1, 1]);
        tree.insert_leaf(3, [-1, -1, -1]);
        assert_eq!(28, tree.get_serialized_size());
    }

    #[test]
    fn get_size_serialize_size_8_tree() {
        let mut tree: Octree<i32> = Octree::new();
        tree.insert_leaf(1, [0, 0, 0]);
        tree.insert_leaf(2, [1, 1, 1]);
        tree.insert_leaf(3, [2, 2, 2]);
        tree.insert_leaf(4, [4, 4, 4]);
        assert_eq!(52, tree.get_serialized_size());
    }

    #[test]
    fn insert_pattern_shouldnt_panic() {
        let mut tree = Octree::new();
        tree.insert_leaf(12, [0, 0, -5]);
        tree.insert_leaf(13, [1, 1, -4]);
        tree.insert_leaf(14, [2, 2, -3]);
        tree.insert_leaf(15, [3, 3, -2]);
    }
}
