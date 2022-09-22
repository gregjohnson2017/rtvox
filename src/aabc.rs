use vecmath::Vector3;

#[derive(PartialEq, Debug, Copy, Clone)]
pub struct Aabc {
    pub origin: Vector3<i32>,
    pub size: u32,
}

impl Aabc {
    pub fn contains(&self, p: Vector3<i32>) -> bool {
        for i in 0..3 {
            if p[i] < self.origin[i] || p[i] >= self.origin[i] + self.size as i32 {
                return false;
            }
        }
        true
    }

    pub fn contains_aabc(&self, aabc: Aabc) -> bool {
        for i in 0..3 {
            if aabc.origin[i] < self.origin[i]
                || aabc.origin[i] + aabc.size as i32 > self.origin[i] + self.size as i32
            {
                return false;
            }
        }
        true
    }

    pub fn expand_towards(&self, target: Vector3<i32>) -> Aabc {
        if self.contains(target) {
            panic!(
                "cannot expand towards target: {:?} inside aabc: {:?}",
                target, self
            )
        }
        let mut expanded = Aabc {
            origin: self.origin,
            size: self.size * 2,
        };
        for i in 0..3 {
            if target[i] < expanded.origin[i] {
                expanded.origin[i] -= self.size as i32;
            }
        }
        expanded
    }

    pub fn shrink_towards(&self, target: Vector3<i32>) -> Aabc {
        if !self.contains(target) {
            panic!(
                "cannot shrink towards target: {:?} outside aabc: {:?}",
                target, self
            )
        }
        let half_size = self.size / 2;
        let mut shrunken = Aabc {
            origin: self.origin,
            size: half_size,
        };
        for i in 0..3 {
            if target[i] >= self.origin[i] + half_size as i32 {
                shrunken.origin[i] += half_size as i32;
            }
        }
        return shrunken;
    }
}

#[cfg(test)]

mod tests {
    use super::Aabc;

    #[test]
    fn contains_point_edge_cases() {
        let aabc = Aabc {
            origin: [0, 0, 0],
            size: 1,
        };
        assert!(aabc.contains([0, 0, 0])); // inclusive
        assert!(!aabc.contains([1, 0, 0])); // exclusive
    }

    #[test]
    fn contains_point_inside_large() {
        let aabc = Aabc {
            origin: [0, 0, 0],
            size: 16,
        };
        assert!(aabc.contains([3, 4, 5]));
    }

    #[test]
    fn contains_point_outside_large() {
        let aabc = Aabc {
            origin: [0, 0, 0],
            size: 16,
        };
        assert!(!aabc.contains([-3, -4, -5]));
    }

    #[test]
    fn contains_point_offset_origin() {
        let aabc = Aabc {
            origin: [8, 8, 8],
            size: 8,
        };
        assert!(aabc.contains([9, 9, 9]));
    }

    #[test]
    #[should_panic]
    fn expand_towards_panics() {
        let aabc = Aabc {
            origin: [8, 8, 8],
            size: 8,
        };
        _ = aabc.expand_towards([9, 9, 9]);
    }

    #[test]
    fn expand_towards_valid() {
        let aabc = Aabc {
            origin: [2, 2, 2],
            size: 2,
        };
        let expect = Aabc {
            origin: [0, 0, 0],
            size: 4,
        };
        let result = aabc.expand_towards([0, 0, 0]);
        assert_eq!(expect, result)
    }

    #[test]
    #[should_panic]
    fn shrink_towards_panics() {
        let aabc = Aabc {
            origin: [8, 8, 8],
            size: 8,
        };
        _ = aabc.shrink_towards([7, 7, 7]);
    }

    #[test]
    fn shrink_towards_min_inclusive() {
        let aabc = Aabc {
            origin: [0, 0, 0],
            size: 4,
        };
        let expect = Aabc {
            origin: [2, 2, 2],
            size: 2,
        };
        let result = aabc.shrink_towards([2, 2, 2]);
        assert_eq!(expect, result)
    }

    #[test]
    fn shrink_towards_max_exclusive() {
        let aabc = Aabc {
            origin: [0, 0, 0],
            size: 4,
        };
        let expect = Aabc {
            origin: [0, 0, 0],
            size: 2,
        };
        let result = aabc.shrink_towards([1, 1, 1]);
        assert_eq!(expect, result)
    }

    #[test]
    fn contains_aabc_self() {
        let aabc = Aabc {
            origin: [0, 0, 0],
            size: 4,
        };
        assert!(aabc.contains_aabc(aabc))
    }

    #[test]
    fn does_not_contain_aabc() {
        let aabc = Aabc {
            origin: [0, 0, 0],
            size: 4,
        };
        let target = Aabc {
            origin: [2, 2, 2],
            size: 4,
        };
        assert!(!aabc.contains_aabc(target))
    }

    #[test]
    fn contains_aabc_enclosed() {
        let aabc = Aabc {
            origin: [0, 0, 0],
            size: 4,
        };
        let target = Aabc {
            origin: [1, 1, 1],
            size: 2,
        };
        assert!(aabc.contains_aabc(target))
    }

    #[test]
    fn contains_aabc_min() {
        let aabc = Aabc {
            origin: [0, 0, 0],
            size: 4,
        };
        let target = Aabc {
            origin: [0, 0, 0],
            size: 2,
        };
        assert!(aabc.contains_aabc(target))
    }

    #[test]
    fn contains_aabc_max() {
        let aabc = Aabc {
            origin: [0, 0, 0],
            size: 4,
        };
        let target = Aabc {
            origin: [2, 2, 2],
            size: 2,
        };
        assert!(aabc.contains_aabc(target))
    }
}
