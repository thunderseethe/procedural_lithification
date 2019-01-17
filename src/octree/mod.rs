#![allow(dead_code)]
extern crate amethyst;
extern crate array_init;

use amethyst::core::nalgebra::geometry::Point3;
use std::{borrow::Borrow, collections::VecDeque, sync::Arc};

#[derive(Debug)]
enum OctreeData<E> {
    Node([Arc<Octree<E>>; 8]),
    Leaf(Arc<E>),
    Empty,
}
impl<E: PartialEq> PartialEq for OctreeData<E> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Empty, Empty) => true,
            (Leaf(ref a_elem), Leaf(ref b_elem)) => *a_elem == *b_elem,
            (Node(ref a_nodes), Node(ref b_nodes)) => *a_nodes == *b_nodes,
            (_, _) => false,
        }
    }
}
impl<E: Eq> Eq for OctreeData<E> {}

impl<E> Clone for OctreeData<E> {
    fn clone(&self) -> Self {
        match *self {
            Node(ref nodes) => Node(nodes.clone()),
            Leaf(ref arc_elem) => Leaf(arc_elem.clone()),
            Empty => Empty,
        }
    }
}

impl<E> OctreeData<E> {
    fn with_leaf<R: Into<Arc<E>>>(elem: R) -> Self {
        Leaf(elem.into())
    }
}

// Alias to allow for easy swapping of position type.
pub type Number = u16;

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct OctantDimensions {
    top_right: Point3<Number>,
    diameter: Number,
}

impl OctantDimensions {
    pub fn new(top_right: Point3<Number>, diameter: Number) -> Self {
        OctantDimensions {
            top_right,
            diameter,
        }
    }

    pub fn x_min(&self) -> Number {
        self.top_right.x - self.diameter
    }
    pub fn x_max(&self) -> Number {
        self.top_right.x
    }
    pub fn y_min(&self) -> Number {
        self.top_right.y - self.diameter
    }
    pub fn y_max(&self) -> Number {
        self.top_right.y
    }
    pub fn z_min(&self) -> Number {
        self.top_right.z - self.diameter
    }
    pub fn z_max(&self) -> Number {
        self.top_right.z
    }

    pub fn top_right(&self) -> Point3<Number> {
        self.top_right.clone()
    }

    pub fn center(&self) -> Point3<Number> {
        let radius = self.diameter / 2;
        Point3::new(
            self.top_right.x - radius,
            self.top_right.y - radius,
            self.top_right.z - radius,
        )
    }

    /// Checks that an octree is contained by this octree
    fn contains<B: Borrow<Self>>(&self, other: B) -> bool {
        let other_ref = other.borrow();
        self.x_max() >= other_ref.x_max()
            && self.x_min() <= other_ref.x_min()
            && self.y_max() >= other_ref.y_max()
            && self.y_min() <= other_ref.y_min()
            && self.z_max() >= other_ref.z_max()
            && self.z_min() <= other_ref.z_min()
    }
}

#[derive(Debug)]
pub struct Octree<E> {
    data: OctreeData<E>,
    bounds: OctantDimensions,
    height: usize,
}

impl<E: PartialEq> PartialEq for Octree<E> {
    fn eq(&self, other: &Octree<E>) -> bool {
        self.height == other.height && self.bounds == other.bounds && self.data == other.data
    }
}
impl<E: Eq> Eq for Octree<E> {}

impl<E> Clone for Octree<E> {
    fn clone(&self) -> Self {
        Octree {
            data: self.data.clone(),
            bounds: self.bounds.clone(),
            height: self.height.clone(),
        }
    }
}

// Represnt each possible Octant as a sum type.
#[derive(PartialEq, Clone, Eq, Copy, Debug)]
enum Octant {
    // x, y, z
    HighHighHigh,
    HighHighLow,
    HighLowHigh,
    HighLowLow,
    LowHighHigh,
    LowHighLow,
    LowLowHigh,
    LowLowLow,
}

impl Into<usize> for Octant {
    fn into(self) -> usize {
        match self {
            HighHighHigh => 0,
            HighHighLow => 1,
            HighLowHigh => 2,
            HighLowLow => 3,
            LowHighHigh => 4,
            LowHighLow => 5,
            LowLowHigh => 6,
            LowLowLow => 7,
        }
    }
}

use self::Octant::*;
impl Octant {
    fn is_x_high(&self) -> bool {
        match self {
            HighHighHigh | HighHighLow | HighLowHigh | HighLowLow => true,
            _ => false,
        }
    }

    fn is_y_high(&self) -> bool {
        match self {
            HighHighHigh | HighHighLow | LowHighHigh | LowHighLow => true,
            _ => false,
        }
    }

    fn is_z_high(&self) -> bool {
        match self {
            HighHighHigh | HighLowHigh | LowHighHigh | LowLowHigh => true,
            _ => false,
        }
    }

    fn sub_octant_bounds(&self, containing_bounds: &OctantDimensions) -> OctantDimensions {
        let (top_right, radius) = (containing_bounds.top_right, containing_bounds.diameter / 2);
        // Bound radius to be 1 at minimum.
        //let bounded_radius: i32 = max(radius, 1);

        let x_center = if self.is_x_high() {
            top_right.x
        } else {
            top_right.x - radius
        };
        let y_center = if self.is_y_high() {
            top_right.y
        } else {
            top_right.y - radius
        };
        let z_center = if self.is_z_high() {
            top_right.z
        } else {
            top_right.z - radius
        };

        OctantDimensions::new(Point3::new(x_center, y_center, z_center), radius)
    }
}

use self::OctreeData::*;

#[inline]
/// Check if all elements of an iterator are equal.
fn all_element_equal<I, E>(iter: &mut I) -> bool
where
    I: Iterator<Item = E>,
    E: PartialEq,
{
    iter.next().map_or(true, |head| iter.all(|ele| head == ele))
}

impl<E: PartialEq> Octree<E> {
    pub fn new(pos: Point3<Number>, opt: Option<E>, height: u32) -> Self {
        let data = opt.map_or(Empty, |elem| Leaf(Arc::new(elem)));
        Octree {
            data,
            bounds: OctantDimensions::new(pos, Number::pow(2, height)),
            height: height as usize,
        }
    }

    pub fn with_root_default(power_of_2: u32) -> Self {
        let diameter: Number = Number::pow(2, power_of_2);
        //let radius = diameter / 2;
        let bounds = OctantDimensions::new(Point3::new(diameter, diameter, diameter), diameter);
        Octree {
            data: Empty,
            bounds: bounds,
            height: power_of_2 as usize,
        }
    }

    pub fn get<P>(&self, pos: P) -> Option<Arc<E>>
    where
        P: Borrow<Point3<Number>>,
    {
        match self.data {
            Empty => None,
            Leaf(ref elem) => Some(elem.clone()),
            Node(ref octants) => {
                let index: usize = self.get_octant(pos.borrow()).into();
                octants[index].get(pos)
            }
        }
    }

    fn with_data(&self, data: OctreeData<E>) -> Self {
        let octree: Self = (*self).clone();
        Octree {
            data: data,
            ..octree
        }
    }

    pub fn delete<P>(&self, pos: P) -> Self
    where
        P: Borrow<Point3<Number>>,
    {
        if self.height == 0 {
            return self.with_data(Empty);
        }
        match self.data {
            // Nothing to do if node is empty
            Empty => self.clone(),
            Leaf(ref curr_leaf) => self.create_sub_nodes(pos, Empty, Leaf(curr_leaf.clone())),
            Node(ref old_nodes) => {
                let mut nodes = old_nodes.clone();
                let index: usize = self.get_octant(pos.borrow()).into();
                let old_octant: &Arc<Octree<E>> = &old_nodes[index];
                nodes[index] = Arc::new(old_octant.delete(pos));
                self.with_data(Node(nodes)).compress_nodes()
            }
        }
    }

    fn outside_bounds<P>(&self, pos_ref: P) -> bool
    where
        P: Borrow<Point3<Number>>,
    {
        let pos = pos_ref.borrow();
        pos.x > self.bounds.x_max()
            || pos.x <= self.bounds.x_min()
            || pos.y > self.bounds.y_max()
            || pos.y <= self.bounds.y_min()
            || pos.z > self.bounds.z_max()
            || pos.z <= self.bounds.z_min()
    }

    pub fn insert<P, R>(&self, pos: P, elem: R) -> Self
    where
        P: Borrow<Point3<Number>>,
        R: Into<Arc<E>>,
    {
        if self.outside_bounds(pos.borrow()) {
            panic!("Position out of bounds");
        }
        let leaf = Leaf(elem.into());
        self.ins(pos, leaf)
    }

    fn ins<P>(&self, pos: P, data: OctreeData<E>) -> Self
    where
        P: Borrow<Point3<Number>>,
    {
        // We cannot subdivide any further past this point
        if self.height == 0 {
            return self.with_data(data);
        }
        match self.data {
            Empty => self.create_sub_nodes(pos, data, Empty),
            ref leaf @ Leaf(_) => {
                if leaf == &data {
                    self.with_data(data)
                } else {
                    self.create_sub_nodes(pos, data, leaf.clone())
                }
            }
            Node(ref old_nodes) => {
                let mut nodes = old_nodes.clone();
                let index: usize = self.get_octant(pos.borrow()).into();
                let old_octant: &Arc<Octree<E>> = &old_nodes[index];
                nodes[index] = Arc::new(old_octant.ins(pos, data));
                self.with_data(Node(nodes)).compress_nodes()
            }
        }
    }

    fn compress_nodes(self) -> Self {
        match self.data {
            Node(ref nodes) => {
                let mut iter = nodes.iter().map(|node| &node.data);
                if iter.next().map_or(true, |head| iter.all(|ele| head == ele)) {
                    self.with_data(nodes[0].data.clone())
                } else {
                    self
                }
            }
            _ => self,
        }
    }

    fn create_sub_nodes<P>(&self, pos: P, elem: OctreeData<E>, default: OctreeData<E>) -> Self
    where
        P: Borrow<Point3<Number>>,
    {
        let modified_octant = self.get_octant(pos.borrow());

        let octree_nodes: [Arc<Octree<E>>; 8] = array_init::array_init(|i| {
            let octant = match i {
                0 => HighHighHigh,
                1 => HighHighLow,
                2 => HighLowHigh,
                3 => HighLowLow,
                4 => LowHighHigh,
                5 => LowHighLow,
                6 => LowLowHigh,
                7 => LowLowLow,
                _ => panic!("Tried to create more than 8 elements in an octree"),
            };

            let data = default.clone();
            let bounds = octant.sub_octant_bounds(&self.bounds);
            let height = self.height - 1;
            let octree = Octree {
                data,
                bounds,
                height,
            };
            let octree = if modified_octant == octant {
                octree.ins(pos.borrow(), elem.clone())
            } else {
                octree
            };
            Arc::new(octree)
        });
        self.with_data(Node(octree_nodes))
    }

    fn get_octant<P>(&self, pos_ref: P) -> Octant
    where
        P: Borrow<Point3<Number>>,
    {
        let pos = pos_ref.borrow();
        let center = self.bounds.center();
        match (pos.x > center.x, pos.y > center.y, pos.z > center.z) {
            (true, true, true) => HighHighHigh,
            (true, true, false) => HighHighLow,
            (true, false, true) => HighLowHigh,
            (true, false, false) => HighLowLow,
            (false, true, true) => LowHighHigh,
            (false, true, false) => LowHighLow,
            (false, false, true) => LowLowHigh,
            (false, false, false) => LowLowLow,
        }
    }

    pub fn iter<'a>(&'a self) -> OctreeIterator<'a, E> {
        let mut stack = VecDeque::new();
        stack.push_back(self);
        OctreeIterator { node_stack: stack }
    }
}

pub struct OctreeIterator<'a, E> {
    node_stack: VecDeque<&'a Octree<E>>,
}
impl<'a, E> Iterator for OctreeIterator<'a, E> {
    type Item = (&'a OctantDimensions, &'a E);

    fn next(&mut self) -> Option<Self::Item> {
        let opt_node = self.node_stack.pop_front();
        opt_node.and_then(|node| match node.data {
            Empty => return self.next(),
            Node(ref children) => {
                let children_iter = children.into_iter().map(|arc| arc.as_ref());
                self.node_stack.extend(children_iter);
                self.next()
            }
            Leaf(ref data) => Some((&node.bounds, data.as_ref())),
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn octree_new_constructs_expected_tree() {
        let octree: Octree<i32> = Octree::new(8);

        assert_eq!(
            octree,
            Octree {
                data: Empty,
                bounds: OctantDimensions::new(Point3::new(256, 256, 256), 256),
                height: 8
            }
        );
    }

    #[test]
    fn octree_dimensions_bounds_are_correct() {
        let dims: OctantDimensions = OctantDimensions::new(Point3::new(1, 1, 1), 2);
        assert_eq!(dims.x_max(), 1);
        assert_eq!(dims.x_min(), -1);
        assert_eq!(dims.y_max(), 1);
        assert_eq!(dims.y_min(), -1);
        assert_eq!(dims.z_max(), 1);
        assert_eq!(dims.z_min(), -1);
        assert_eq!(dims.center(), Point3::new(0, 0, 0));
    }

    #[test]
    fn octree_creates_dimensions() {
        let octree: Octree<()> = Octree::new(8);
        assert_eq!(octree.bounds.x_max(), 256);
        assert_eq!(octree.bounds.x_min(), 0);
        assert_eq!(octree.bounds.y_max(), 256);
        assert_eq!(octree.bounds.y_min(), 0);
        assert_eq!(octree.bounds.z_max(), 256);
        assert_eq!(octree.bounds.z_min(), 0);
        assert_eq!(octree.bounds.center(), Point3::new(128, 128, 128));
    }

    #[test]
    fn octree_subnodes_constructed_correctly() {
        let octree: Octree<i32> = Octree::new(1);

        let points = vec![
            (Point3::new(0, 0, 0), true),
            (Point3::new(0, 0, 1), true),
            (Point3::new(0, 1, 0), true),
            (Point3::new(0, 1, 1), true),
            (Point3::new(1, 0, 0), true),
            (Point3::new(1, 0, 1), true),
            (Point3::new(1, 1, 0), true),
            (Point3::new(1, 1, 1), false),
            (Point3::new(1, 1, 2), false),
            (Point3::new(1, 2, 1), false),
            (Point3::new(1, 2, 2), false),
            (Point3::new(2, 1, 1), false),
            (Point3::new(2, 1, 2), false),
            (Point3::new(2, 2, 1), false),
            (Point3::new(2, 2, 2), false),
            (Point3::new(2, 2, 3), true),
            (Point3::new(2, 3, 2), true),
            (Point3::new(2, 3, 3), true),
        ];
        for (p, expected) in points {
            assert_eq!(octree.outside_bounds(p), expected, "{:?}", p);
        }
    }

    #[test]
    fn octree_insert_handles_center_point() {
        let octree: Octree<i32> = Octree::new(4);
        let center = octree.bounds.center();

        assert_eq!(
            octree.insert(&center, 1234).get(&center),
            Some(Arc::new(1234))
        );
    }

    #[test]
    fn octree_element_retrieved_after_insertion_in_same_octants() {
        let p1 = Point3::new(2, 2, 2);
        let p2 = Point3::new(1, 1, 1);
        let octree: Octree<i32> = Octree::new(2).insert(&p1, 1234).insert(&p2, 5678);

        assert_eq!(octree.get(&p1), Some(Arc::new(1234)));
        assert_eq!(octree.get(&p2), Some(Arc::new(5678)));
    }

    #[test]
    fn octree_element_retrieved_after_inserterion_in_diff_octants() {
        let p1 = Point3::new(1, 1, 1);
        let p2 = Point3::new(7, 7, 7);
        let octree: Octree<i32> = Octree::new(3).insert(&p1, 1234).insert(&p2, 5678);

        assert_eq!(octree.get(&p1), Some(Arc::new(1234)));
        assert_eq!(octree.get(&p2), Some(Arc::new(5678)));
    }

    #[test]
    fn octree_insert_updates_element() {
        let p = Point3::new(1, 1, 1);
        let octree: Octree<i32> = Octree::new(4).insert(&p, 1234);

        assert_eq!(octree.get(&p), Some(Arc::new(1234)));

        let octree = octree.insert(&p, 5678);
        assert_eq!(octree.get(&p), Some(Arc::new(5678)));
    }

    #[test]
    fn octree_deletes_expected_element() {
        let octree: Octree<i32> = Octree::new(5)
            .insert(Point3::new(1, 1, 1), 1234)
            .insert(Point3::new(1, 1, 2), 4567)
            .insert(Point3::new(4, 1, 1), 7890);

        assert_eq!(octree.get(Point3::new(4, 0, 0)), Some(Arc::new(7890)));
        let octree = octree.delete(Point3::new(4, 0, 0));
        assert_eq!(octree.get(Point3::new(4, 0, 0)), None);
    }

    #[test]
    fn octree_delete_is_idempotent() {
        let p = Point3::new(1, 1, 1);
        let octree: Octree<i32> = Octree::new(5).insert(&p, 1234);

        let result = octree.delete(&p).delete(&p);
        assert_eq!(result.get(&p), None);
    }

    #[test]
    fn octree_iterator_length_is_correct() {
        let octree: Octree<i32> = Octree::new(5)
            .insert(Point3::new(2, 2, 2), 1234)
            .insert(Point3::new(1, 1, 2), 4567)
            .insert(Point3::new(2, 1, 1), 7890);

        assert_eq!(octree.iter().count(), 3);
    }

    #[test]
    fn octree_iterator_contains_correct_elements() {
        let octree = Octree::new(3)
            .insert(Point3::new(2, 2, 2), 1)
            .insert(Point3::new(2, 4, 2), 2)
            .insert(Point3::new(4, 4, 4), 3)
            .insert(Point3::new(2, 2, 4), 4);
        let mut iter = octree.iter();

        assert_eq!(
            iter.next(),
            Some((&OctantDimensions::new(Point3::new(4, 4, 4), 1), &3))
        );
        assert_eq!(
            iter.next(),
            Some((&OctantDimensions::new(Point3::new(2, 4, 2), 1), &2))
        );
        assert_eq!(
            iter.next(),
            Some((&OctantDimensions::new(Point3::new(2, 2, 4), 1), &4))
        );
        assert_eq!(
            iter.next(),
            Some((&OctantDimensions::new(Point3::new(2, 2, 2), 1), &1))
        );
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn octree_insertion_compresses_common_subnodes_in_single_level() {
        let octree = Octree::new(1)
            .insert(Point3::new(2, 2, 2), 1)
            .insert(Point3::new(2, 2, 1), 1)
            .insert(Point3::new(2, 1, 2), 1)
            .insert(Point3::new(1, 2, 1), 1)
            .insert(Point3::new(1, 2, 2), 1)
            .insert(Point3::new(2, 1, 1), 1)
            .insert(Point3::new(1, 1, 2), 1)
            .insert(Point3::new(1, 1, 1), 1);

        assert_eq!(
            octree,
            Octree {
                data: Leaf(Arc::new(1)),
                bounds: OctantDimensions::new(Point3::new(2, 2, 2), 2),
                height: 1
            }
        );
    }

    #[test]
    fn octree_insertion_compresses_common_nodes_in_subtree() {
        let octree = Octree::new(8)
            .insert(Point3::new(2, 2, 2), 1234)
            .insert(Point3::new(2, 2, 1), 1234)
            .insert(Point3::new(2, 1, 2), 1234)
            .insert(Point3::new(1, 2, 1), 1234)
            .insert(Point3::new(1, 2, 2), 1234)
            .insert(Point3::new(2, 1, 1), 1234)
            .insert(Point3::new(1, 1, 2), 1234)
            .insert(Point3::new(1, 1, 1), 1234);

        let mut iter = octree.iter();
        assert_eq!(
            iter.next(),
            Some((&OctantDimensions::new(Point3::new(2, 2, 2), 2), &1234))
        );
    }
}
