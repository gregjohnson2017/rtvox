use vecmath::{vec3_scale, Vector3};

use crate::aabc::Aabc;

pub struct Octree<T: Copy> {
    root: Child<T>,
}

#[derive(PartialEq, Debug)]
enum Child<T: Copy> {
    Leaf(Leaf<T>),
    Node(Box<Node<T>>),
    None,
}

#[derive(PartialEq, Debug, Copy, Clone)]
pub struct Leaf<T: Copy> {
    pos: Vector3<i32>,
    data: T,
}

#[derive(PartialEq, Debug)]
struct Node<T: Copy> {
    children: [Child<T>; 8],
    aabc: Aabc,
}

#[derive(Debug, PartialEq)]
enum ChildAddingError {
    ChildOutsideParent,
    ChildMisaligned,
    ParentNotSize2,
    ParentNotTwiceChild,
    ParentIsNone,
}

impl<T: Copy> Node<T> {
    fn empty(aabc: Aabc) -> Box<Node<T>> {
        Box::new(Node {
            children: [
                Child::None,
                Child::None,
                Child::None,
                Child::None,
                Child::None,
                Child::None,
                Child::None,
                Child::None,
            ],
            aabc,
        })
    }

    fn get_child_idx(&self, p: Vector3<i32>, off: i32) -> Result<usize, ChildAddingError> {
        let min = self.aabc.origin;
        if p == [min[0], min[1], min[2]] {
            return Ok(6);
        } else if p == [min[0] + off, min[1], min[2]] {
            return Ok(7);
        } else if p == [min[0], min[1] + off, min[2]] {
            return Ok(5);
        } else if p == [min[0] + off, min[1] + off, min[2]] {
            return Ok(4);
        } else if p == [min[0], min[1], min[2] + off] {
            return Ok(2);
        } else if p == [min[0] + off, min[1], min[2] + off] {
            return Ok(3);
        } else if p == [min[0], min[1] + off, min[2] + off] {
            return Ok(1);
        } else if p == [min[0] + off, min[1] + off, min[2] + off] {
            return Ok(0);
        } else {
            return Err(ChildAddingError::ChildMisaligned);
        }
    }

    fn add_child(&mut self, child: Child<T>) -> Result<(), ChildAddingError> {
        match child {
            Child::Leaf(leaf) => {
                if !self.aabc.contains(leaf.pos) {
                    return Err(ChildAddingError::ChildOutsideParent);
                }
                if self.aabc.size != 2 {
                    return Err(ChildAddingError::ParentNotSize2);
                }
                match self.get_child_idx(leaf.pos, 1) {
                    Ok(idx) => Ok(self.children[idx] = child),
                    Err(e) => Err(e),
                }
            }
            Child::Node(ref node) => {
                if !self.aabc.contains(node.aabc.origin) {
                    return Err(ChildAddingError::ChildOutsideParent);
                }
                if self.aabc.size != node.aabc.size * 2 {
                    return Err(ChildAddingError::ParentNotTwiceChild);
                }
                let off = self.aabc.size as i32 / 2;
                match self.get_child_idx(node.aabc.origin, off) {
                    Ok(idx) => Ok(self.children[idx] = child),
                    Err(e) => Err(e),
                }
            }
            Child::None => Err(ChildAddingError::ParentIsNone),
        }
    }
}

impl<T: Copy> Octree<T> {
    pub fn new() -> Self {
        Octree { root: Child::None }
    }

    pub fn insert_leaf(&mut self, new_leaf: Leaf<T>) {
        match self.root {
            Child::None => self.root = Child::Leaf(new_leaf),
            Child::Leaf(old_leaf) if old_leaf.pos == new_leaf.pos => {
                panic!("overwrote root leaf")
            }
            Child::Leaf(old_leaf) => {
                /*
                The root of the tree pointed to 1 leaf. Create a node structure that includes the old and new leaf.
                */
                let leaf_aabc = Aabc {
                    origin: old_leaf.pos,
                    size: 1,
                };
                let mut curr = Node::empty(leaf_aabc.expand_towards(new_leaf.pos));
                curr.add_child(Child::Leaf(old_leaf)).unwrap();
                while !curr.aabc.contains(new_leaf.pos) {
                    let mut n = Node::empty(curr.aabc.expand_towards(new_leaf.pos));
                    n.add_child(Child::Node(curr)).unwrap();
                    curr = n;
                }
                self.root = Child::Node(curr);
                while curr.aabc.size > 2 {
                    let mut n = Node::empty(curr.aabc.shrink_towards(new_leaf.pos));
                    curr.add_child(Child::Node(n));
                    curr = n;
                }
                curr.add_child(Child::Leaf(new_leaf));
            }
            Child::Node(_) => {
                /*
                The root of the tree pointed to a node. Insert the new leaf into the node structure, creating new nodes if necessary.
                */
                panic!("")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{aabc::Aabc, octree::Node};

    use super::{Child, ChildAddingError, Leaf, Octree};

    #[test]
    fn insert_leaf() {
        let mut tree = Octree::new();
        let expected_leaf = Leaf {
            pos: [0, 0, 0],
            data: 0,
        };
        tree.insert_leaf(expected_leaf);
        assert_eq!(tree.root, Child::Leaf(expected_leaf))
    }

    #[test]
    #[should_panic]
    fn insert_duplicate_leaf_panics() {
        let mut tree = Octree::new();
        tree.insert_leaf(Leaf {
            pos: [0, 0, 0],
            data: 0,
        });
        tree.insert_leaf(Leaf {
            pos: [0, 0, 0],
            data: 0,
        });
    }

    #[test]
    fn add_child_leaf_outside_node_panics() {
        let mut node = Node::empty(Aabc {
            origin: [0, 0, 0],
            size: 2,
        });
        match node.add_child(Child::Leaf(Leaf {
            pos: [2, 2, 2],
            data: 0,
        })) {
            Err(e) => assert_eq!(e, ChildAddingError::ChildOutsideParent),
            _ => assert!(false),
        }
    }

    #[test]
    fn add_child_leaf_to_large_node_panics() {
        let mut node = Node::empty(Aabc {
            origin: [0, 0, 0],
            size: 4,
        });
        match node.add_child(Child::Leaf(Leaf {
            pos: [0, 0, 0],
            data: 0,
        })) {
            Err(e) => assert_eq!(e, ChildAddingError::ParentNotSize2),
            _ => assert!(false),
        }
    }

    #[test]
    fn add_child_node_to_incompatibly_sized_node_panics() {
        let mut node: Box<Node<i32>> = Node::empty(Aabc {
            origin: [0, 0, 0],
            size: 8,
        });
        match node.add_child(Child::Node(Node::empty(Aabc {
            origin: [0, 0, 0],
            size: 2,
        }))) {
            Err(e) => assert_eq!(e, ChildAddingError::ParentNotTwiceChild),
            _ => assert!(false),
        }
    }

    #[test]
    fn add_child_node_outside_node_panics() {
        let mut node: Box<Node<i32>> = Node::empty(Aabc {
            origin: [0, 0, 0],
            size: 4,
        });
        match node.add_child(Child::Node(Node::empty(Aabc {
            origin: [4, 4, 4],
            size: 2,
        }))) {
            Err(e) => assert_eq!(e, ChildAddingError::ChildOutsideParent),
            _ => assert!(false),
        }
    }

    #[test]
    fn add_none_child_to_node_panics() {
        let mut node: Box<Node<i32>> = Node::empty(Aabc {
            origin: [0, 0, 0],
            size: 4,
        });
        match node.add_child(Child::None) {
            Err(e) => assert_eq!(e, ChildAddingError::ParentIsNone),
            _ => assert!(false),
        }
    }

    #[test]
    fn add_children_leaves_to_node() {
        let mut node: Box<Node<i32>> = Node::empty(Aabc {
            origin: [0, 0, 0],
            size: 2,
        });
        let expected_children = [
            Leaf {
                pos: [1, 1, 1],
                data: 0,
            },
            Leaf {
                pos: [0, 1, 1],
                data: 0,
            },
            Leaf {
                pos: [0, 0, 1],
                data: 0,
            },
            Leaf {
                pos: [1, 0, 1],
                data: 0,
            },
            Leaf {
                pos: [1, 1, 0],
                data: 0,
            },
            Leaf {
                pos: [0, 1, 0],
                data: 0,
            },
            Leaf {
                pos: [0, 0, 0],
                data: 0,
            },
            Leaf {
                pos: [1, 0, 0],
                data: 0,
            },
        ];
        for i in 0..expected_children.len() {
            node.add_child(Child::Leaf(expected_children[i])).unwrap();
        }
        for i in 0..expected_children.len() {
            assert_eq!(Child::Leaf(expected_children[i]), node.children[i])
        }
    }

    #[test]
    fn add_children_nodes_to_node() {
        let mut node: Box<Node<i32>> = Node::empty(Aabc {
            origin: [0, 0, 0],
            size: 4,
        });
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
            node.add_child(Child::Node(Node::empty(expected_aabcs[i])))
                .unwrap();
        }
        for i in 0..expected_aabcs.len() {
            assert_eq!(
                Child::Node(Node::empty(expected_aabcs[i])),
                node.children[i]
            )
        }
    }

    #[test]
    fn insert_two_leaves() {
        let mut tree = Octree::new();
        let leaf1 = Leaf {
            pos: [0, 0, 0],
            data: 0,
        };
        let leaf2 = Leaf {
            pos: [1, 0, 0],
            data: 1,
        };
        tree.insert_leaf(leaf1);
        tree.insert_leaf(leaf2);
        let mut expected_node = Node::empty(Aabc {
            origin: [0, 0, 0],
            size: 2,
        });
        expected_node.children[6] = Child::Leaf(leaf1);
        expected_node.children[7] = Child::Leaf(leaf2);
        match tree.root {
            Child::Node(ref x) => {
                assert_eq!(
                    x.aabc,
                    Aabc {
                        origin: [0, 0, 0],
                        size: 2,
                    }
                );
                assert_eq!(x.children, expected_node.children);
                // TODO can we compare like this?
                // assert_eq!(x, &expected_node);
            }
            _ => assert!(false, "root was {:?}", tree.root),
        };
    }
}
