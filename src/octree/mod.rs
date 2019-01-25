extern crate amethyst;
extern crate array_init;
extern crate num_traits;

use amethyst::core::nalgebra::geometry::Point3;
use num_traits::Num;
use std::{
    borrow::Borrow,
    cmp::{Ord, Ordering, PartialOrd},
    collections::VecDeque,
    sync::Arc,
};

#[derive(Debug)]
pub enum OctreeData<E> {
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
    bottom_left: Point3<Number>,
    diameter: Number,
}

impl PartialOrd for OctantDimensions {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for OctantDimensions {
    fn cmp(&self, other: &Self) -> Ordering {
        use std::cmp::Ordering::*;
        let cmps = (
            self.bottom_left.x.cmp(&other.bottom_left.x),
            self.bottom_left.y.cmp(&other.bottom_left.y),
            self.bottom_left.z.cmp(&other.bottom_left.z),
            self.diameter.cmp(&other.diameter),
        );
        match cmps {
            (Greater, Greater, Greater, _) => Greater,
            (Greater, _, _, _) => Greater,
            (Equal, Greater, _, _) => Greater,
            (Equal, Equal, Greater, _) => Greater,
            (Equal, Equal, Equal, Greater) => Greater,
            (Equal, Equal, Equal, Equal) => Equal,
            (_, _, _, _) => Less,
        }
    }
}

impl OctantDimensions {
    pub fn new(bottom_left: Point3<Number>, diameter: Number) -> Self {
        OctantDimensions {
            bottom_left,
            diameter,
        }
    }

    pub fn nearest_octant_point(p: Point3<Number>, height: u32) -> Point3<Number> {
        let multiple = Number::pow(2, height);
        let mut new_point = p.clone();
        for e in new_point.iter_mut() {
            *e = (*e as f32 / multiple as f32).floor() as Number * multiple;
        }
        return new_point;
    }

    pub fn x_min(&self) -> Number {
        self.bottom_left.x
    }
    pub fn x_max(&self) -> Number {
        self.bottom_left.x + self.diameter - 1
    }
    pub fn y_min(&self) -> Number {
        self.bottom_left.y
    }
    pub fn y_max(&self) -> Number {
        self.bottom_left.y + self.diameter - 1
    }
    pub fn z_min(&self) -> Number {
        self.bottom_left.z
    }
    pub fn z_max(&self) -> Number {
        self.bottom_left.z + self.diameter - 1
    }

    pub fn top_right(&self) -> Point3<Number> {
        let mut top_right = self.bottom_left.clone();
        for e in top_right.iter_mut() {
            *e += self.diameter - 1;
        }
        return top_right;
    }

    pub fn center(&self) -> Point3<Number> {
        let radius = self.diameter / 2;
        let mut center = self.bottom_left.clone();
        for e in center.iter_mut() {
            *e += radius;
        }
        return center;
    }

    pub fn bottom_left(&self) -> Point3<Number> {
        self.bottom_left.clone()
    }

    pub fn diameter(&self) -> Number {
        self.diameter
    }

    pub fn get_octant<P>(&self, pos_ref: P) -> Octant
    where
        P: Borrow<Point3<Number>>,
    {
        let pos = pos_ref.borrow();
        let center = self.center();
        match (pos.x >= center.x, pos.y >= center.y, pos.z >= center.z) {
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
}

#[derive(Debug)]
pub struct Octree<E> {
    data: OctreeData<E>,
    bounds: OctantDimensions,
    height: u32,
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
pub enum Octant {
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

macro_rules! octant_num_conversions {
    ($( $num:ty ),* ) => {
        $(
            impl Into<$num> for Octant {
                fn into(self) -> $num {
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

            impl From<$num> for Octant {
                fn from(num: $num) -> Self {
                    match num {
                        0 => HighHighHigh,
                        1 => HighHighLow,
                        2 => HighLowHigh,
                        3 => HighLowLow,
                        4 => LowHighHigh,
                        5 => LowHighLow,
                        6 => LowLowHigh,
                        7 => LowLowLow,
                        _ => panic!("Tried to create more than 8 elements in an octree"),
                    }
                }
            }
        )*
    };
}

octant_num_conversions!(u8, u16, u32, u64, usize, i8, i16, i32, i64, isize);

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
        let (bottom_left, center) = (containing_bounds.bottom_left(), containing_bounds.center());

        let x_center = if self.is_x_high() {
            center.x
        } else {
            bottom_left.x
        };
        let y_center = if self.is_y_high() {
            center.y
        } else {
            bottom_left.y
        };
        let z_center = if self.is_z_high() {
            center.z
        } else {
            bottom_left.z
        };

        OctantDimensions::new(
            Point3::new(x_center, y_center, z_center),
            containing_bounds.diameter / 2,
        )
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
            height: height,
        }
    }

    pub fn with_uniform_dimension(power_of_2: u32) -> Self {
        let diameter: Number = Number::pow(2, power_of_2);
        //let radius = diameter / 2;
        let bounds = OctantDimensions::new(Point3::new(0, 0, 0), diameter);
        Octree {
            data: Empty,
            bounds: bounds,
            height: power_of_2,
        }
    }

    pub fn with_children<I>(children: I, pos: Point3<Number>, height: u32) -> Self
    where
        I: Into<[Arc<Octree<E>>; 8]>,
    {
        let nodes: [Arc<Octree<E>>; 8] = children.into();
        Octree {
            data: Node(nodes),
            bounds: OctantDimensions::new(pos, Number::pow(2, height)),
            height,
        }
        .compress_nodes()
    }

    pub fn get<P>(&self, pos: P) -> Option<Arc<E>>
    where
        P: Borrow<Point3<Number>>,
    {
        match self.data {
            Empty => None,
            Leaf(ref elem) => Some(elem.clone()),
            Node(ref octants) => {
                let index: usize = self.bounds.get_octant(pos.borrow()).into();
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

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn root_point(&self) -> Point3<Number> {
        self.bounds.bottom_left()
    }

    pub fn data<'a>(&'a self) -> &'a OctreeData<E> {
        &self.data
    }

    fn is_empty(&self) -> bool {
        match self.data {
            Empty => true,
            _ => false,
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
                let index: usize = self.bounds.get_octant(pos.borrow()).into();
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
            || pos.x < self.bounds.x_min()
            || pos.y > self.bounds.y_max()
            || pos.y < self.bounds.y_min()
            || pos.z > self.bounds.z_max()
            || pos.z < self.bounds.z_min()
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
                let index: usize = self.bounds.get_octant(pos.borrow()).into();
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
        let modified_octant = self.bounds.get_octant(pos.borrow());

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

    pub fn iter<'a>(&'a self) -> OctreeIterator<'a, E> {
        let mut stack = VecDeque::new();
        stack.push_back(self);
        OctreeIterator { node_stack: stack }
    }
}

pub struct OctreeIterator<'a, E> {
    node_stack: VecDeque<&'a Octree<E>>,
}
impl<'a, E: PartialEq> Iterator for OctreeIterator<'a, E> {
    type Item = (&'a OctantDimensions, &'a E);

    fn next(&mut self) -> Option<Self::Item> {
        let mut opt_node = self.node_stack.pop_front();
        while let Some(node) = opt_node {
            match node.data {
                Empty => {}
                Leaf(ref data) => {
                    return Some((&node.bounds, data.as_ref()));
                }
                Node(ref children) => {
                    let children_iter = children
                        .into_iter()
                        .map(|arc| arc.as_ref())
                        .filter(|node_ref| !node_ref.is_empty());
                    self.node_stack.extend(children_iter);
                }
            }
            opt_node = self.node_stack.pop_front();
        }
        return None;
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn octree_new_constructs_expected_tree() {
        let octree: Octree<i32> = Octree::with_uniform_dimension(8);

        assert_eq!(
            octree,
            Octree {
                data: Empty,
                bounds: OctantDimensions::new(Point3::new(0, 0, 0), 256),
                height: 8
            }
        );
    }

    #[test]
    fn octree_dimensions_bounds_are_correct() {
        let dims: OctantDimensions = OctantDimensions::new(Point3::new(1, 1, 1), 2);
        assert_eq!(dims.x_max(), 2);
        assert_eq!(dims.x_min(), 1);
        assert_eq!(dims.y_max(), 2);
        assert_eq!(dims.y_min(), 1);
        assert_eq!(dims.z_max(), 2);
        assert_eq!(dims.z_min(), 1);
        assert_eq!(dims.center(), Point3::new(2, 2, 2));
    }

    #[test]
    fn octree_creates_dimensions() {
        let octree: Octree<()> = Octree::with_uniform_dimension(8);
        assert_eq!(octree.bounds.x_max(), 255);
        assert_eq!(octree.bounds.x_min(), 0);
        assert_eq!(octree.bounds.y_max(), 255);
        assert_eq!(octree.bounds.y_min(), 0);
        assert_eq!(octree.bounds.z_max(), 255);
        assert_eq!(octree.bounds.z_min(), 0);
        assert_eq!(octree.bounds.center(), Point3::new(128, 128, 128));
    }

    #[test]
    fn octree_subnodes_constructed_correctly() {
        let octree: Octree<i32> = Octree::with_uniform_dimension(1);

        let points = vec![
            (Point3::new(0, 0, 0), false),
            (Point3::new(0, 0, 1), false),
            (Point3::new(0, 1, 0), false),
            (Point3::new(0, 1, 1), false),
            (Point3::new(1, 0, 0), false),
            (Point3::new(1, 0, 1), false),
            (Point3::new(1, 1, 0), false),
            (Point3::new(1, 1, 1), false),
            (Point3::new(1, 1, 2), true),
            (Point3::new(1, 2, 1), true),
            (Point3::new(1, 2, 2), true),
            (Point3::new(2, 1, 1), true),
            (Point3::new(2, 1, 2), true),
            (Point3::new(2, 2, 1), true),
            (Point3::new(2, 2, 2), true),
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
        let octree: Octree<i32> = Octree::with_uniform_dimension(4);
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
        let octree: Octree<i32> = Octree::with_uniform_dimension(2)
            .insert(&p1, 1234)
            .insert(&p2, 5678);

        assert_eq!(octree.get(&p1), Some(Arc::new(1234)));
        assert_eq!(octree.get(&p2), Some(Arc::new(5678)));
    }

    #[test]
    fn octree_element_retrieved_after_inserterion_in_diff_octants() {
        let p1 = Point3::new(1, 1, 1);
        let p2 = Point3::new(7, 7, 7);
        let octree: Octree<i32> = Octree::with_uniform_dimension(3)
            .insert(&p1, 1234)
            .insert(&p2, 5678);

        assert_eq!(octree.get(&p1), Some(Arc::new(1234)));
        assert_eq!(octree.get(&p2), Some(Arc::new(5678)));
    }

    #[test]
    fn octree_insert_updates_element() {
        let p = Point3::new(1, 1, 1);
        let octree: Octree<i32> = Octree::with_uniform_dimension(4).insert(&p, 1234);

        assert_eq!(octree.get(&p), Some(Arc::new(1234)));

        let octree = octree.insert(&p, 5678);
        assert_eq!(octree.get(&p), Some(Arc::new(5678)));
    }

    #[test]
    fn octree_deletes_expected_element() {
        let p = Point3::new(4, 1, 1);
        let octree: Octree<i32> = Octree::with_uniform_dimension(5)
            .insert(Point3::new(1, 1, 1), 1234)
            .insert(Point3::new(1, 1, 2), 4567)
            .insert(&p, 7890);

        assert_eq!(octree.get(&p), Some(Arc::new(7890)));
        let octree = octree.delete(&p);
        assert_eq!(octree.get(&p), None);
    }

    #[test]
    fn octree_print_test() {
        let octree = Octree::with_uniform_dimension(8)
            .insert(Point3::new(1, 1, 1), 1)
            .insert(Point3::new(1, 1, 2), 2)
            .insert(Point3::new(1, 2, 1), 3)
            .insert(Point3::new(1, 2, 2), 4)
            .insert(Point3::new(2, 1, 1), 5)
            .insert(Point3::new(2, 1, 2), 6)
            .insert(Point3::new(2, 2, 1), 7)
            .insert(Point3::new(2, 2, 2), 8);
        println!("{:#?}", octree);
    }

    #[test]
    fn octree_delete_is_idempotent() {
        let p = Point3::new(1, 1, 1);
        let octree: Octree<i32> = Octree::with_uniform_dimension(5).insert(&p, 1234);

        let result = octree.delete(&p).delete(&p);
        assert_eq!(result.get(&p), None);
    }

    #[test]
    fn octree_iterator_length_is_correct() {
        let octree: Octree<i32> = Octree::with_uniform_dimension(5)
            .insert(Point3::new(2, 2, 2), 1234)
            .insert(Point3::new(1, 1, 2), 4567)
            .insert(Point3::new(2, 1, 1), 7890);

        assert_eq!(octree.iter().count(), 3);
    }

    #[test]
    fn octree_iterator_contains_correct_elements() {
        let octree = Octree::with_uniform_dimension(3)
            .insert(Point3::new(2, 2, 2), 1)
            .insert(Point3::new(2, 4, 2), 2)
            .insert(Point3::new(4, 4, 4), 3)
            .insert(Point3::new(2, 2, 4), 4);
        let result_set: BTreeSet<(&OctantDimensions, &u32)> = octree.iter().collect();
        let mut expected_set = BTreeSet::new();
        expected_set.insert((OctantDimensions::new(Point3::new(4, 4, 4), 1), 3u32));
        expected_set.insert((OctantDimensions::new(Point3::new(2, 4, 2), 1), 2u32));
        expected_set.insert((OctantDimensions::new(Point3::new(2, 2, 4), 1), 4u32));
        expected_set.insert((OctantDimensions::new(Point3::new(2, 2, 2), 1), 1u32));

        assert_eq!(
            result_set,
            expected_set.iter().map(|(dim, i)| (dim, i)).collect()
        );
    }

    #[test]
    fn octree_insertion_compresses_common_subnodes_in_single_level() {
        let octree = Octree::with_uniform_dimension(1)
            .insert(Point3::new(1, 1, 1), 1)
            .insert(Point3::new(1, 1, 0), 1)
            .insert(Point3::new(1, 0, 1), 1)
            .insert(Point3::new(0, 1, 0), 1)
            .insert(Point3::new(0, 1, 1), 1)
            .insert(Point3::new(1, 0, 0), 1)
            .insert(Point3::new(0, 0, 1), 1)
            .insert(Point3::new(0, 0, 0), 1);

        assert_eq!(
            octree,
            Octree {
                data: Leaf(Arc::new(1)),
                bounds: OctantDimensions::new(Point3::new(0, 0, 0), 2),
                height: 1
            }
        );
    }

    #[test]
    fn octree_insertion_compresses_common_nodes_in_subtree() {
        let octree = Octree::with_uniform_dimension(8)
            .insert(Point3::new(1, 1, 1), 1234)
            .insert(Point3::new(1, 1, 0), 1234)
            .insert(Point3::new(1, 0, 1), 1234)
            .insert(Point3::new(0, 1, 0), 1234)
            .insert(Point3::new(0, 1, 1), 1234)
            .insert(Point3::new(1, 0, 0), 1234)
            .insert(Point3::new(0, 0, 1), 1234)
            .insert(Point3::new(0, 0, 0), 1234);

        let mut iter = octree.iter();
        assert_eq!(
            iter.next(),
            Some((&OctantDimensions::new(Point3::new(0, 0, 0), 2), &1234))
        );
    }

    #[test]
    fn octree_test_large_amount_of_insertions() {
        let mut octree = Octree::with_uniform_dimension(8);
        for _ in 0..1000 {
            octree = octree.insert(
                Point3::new(
                    rand::random::<u8>().into(),
                    rand::random::<u8>().into(),
                    rand::random::<u8>().into(),
                ),
                1234,
            );
        }
        println!("{:?}", octree.root_point());
    }
}
