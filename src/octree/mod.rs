use crate::mut_ptr::MultiThreadMutPtr;
use amethyst::core::nalgebra::geometry::Point3;
use num_traits::FromPrimitive;
use rayon::{iter::plumbing::*, prelude::*};
use std::{borrow::Borrow, collections::VecDeque, fmt, sync::Arc};

pub mod octant;
pub mod octant_dimensions;
pub mod octant_face;
pub mod octree_data;

use octant::Octant;
use octant_dimensions::OctantDimensions;
use octant_face::OctantFace;
use octree_data::{OctreeData, OctreeData::*};

// Alias to allow for easy swapping of position type.
pub type Number = u8;

#[derive(Serialize, Deserialize)]
pub struct Octree<E> {
    data: OctreeData<E>,
    bounds: OctantDimensions,
    height: u32,
}

impl<E: fmt::Debug> fmt::Debug for Octree<E> {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct("Octree")
            .field("data", &self.data)
            .field("bounds", &self.bounds)
            .finish()
    }
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

impl<E: PartialEq> Octree<E> {
    pub fn new(pos: Point3<Number>, opt: Option<E>, height: u32) -> Self {
        let data = opt.map_or(Empty, |elem| Leaf(Arc::new(elem)));
        Octree {
            data,
            bounds: OctantDimensions::new(pos, u16::pow(2, height)),
            height: height,
        }
    }

    pub fn with_uniform_dimension(power_of_2: u32) -> Self {
        let diameter: u16 = u16::pow(2, power_of_2);
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
            bounds: OctantDimensions::new(pos, u16::pow(2, height)),
            height,
        }
        .compress_nodes()
    }

    pub fn with_fields(data: OctreeData<E>, bounds: OctantDimensions, height: u32) -> Self {
        Octree {
            data,
            bounds,
            height,
        }
    }

    pub fn face_boundary_adjacent(&self, dim: &OctantDimensions, face: OctantFace) -> bool {
        use octant_face::OctantFace::*;
        match face {
            Back => dim.z_max() >= self.bounds.z_max(),
            Up => dim.y_max() >= self.bounds.y_max(),
            Front => dim.z_min() <= self.bounds.z_min(),
            Down => dim.y_min() <= self.bounds.y_min(),
            Right => dim.x_max() >= self.bounds.x_max(),
            Left => dim.x_min() <= self.bounds.x_min(),
        }
    }

    pub fn check_octant_face_visible<P>(&self, pos: P, diameter: u16) -> bool
    where
        P: Borrow<Point3<Number>>,
    {
        if self.outside_bounds(pos.borrow()) {
            true
        } else {
            self.check_octant_face_visible_(pos, diameter)
        }
    }
    fn check_octant_face_visible_<P>(&self, pos: P, diameter: u16) -> bool
    where
        P: Borrow<Point3<Number>>,
    {
        match self.data {
            Empty => true,
            // When we encounter a leaf node our face is visible if the leaf node is smaller than the node we're drawing
            Leaf(_) => self.bounds.diameter() < diameter,
            Node(ref octants) => {
                let index: usize = self.bounds.get_octant_index(pos.borrow());
                octants[index].check_octant_face_visible_(pos, diameter)
            }
        }
    }

    pub fn get<P>(&self, pos: P) -> Option<&E>
    where
        P: Borrow<Point3<Number>>,
    {
        if self.outside_bounds(pos.borrow()) {
            None
        } else {
            self.get_(pos)
        }
    }

    fn get_<P>(&self, pos: P) -> Option<&E>
    where
        P: Borrow<Point3<Number>>,
    {
        match self.data {
            Empty => None,
            Leaf(ref elem) => Some(elem.as_ref()),
            Node(ref octants) => {
                let index: usize = self.bounds.get_octant_index(pos.borrow());
                octants[index].get_(pos)
            }
        }
    }

    pub fn get_octant<P>(&self, pos: P) -> Option<&Octree<E>>
    where
        P: Borrow<Point3<Number>>,
    {
        if self.outside_bounds(pos.borrow()) {
            None
        } else {
            self.get_octant_unchecked(pos)
        }
    }
    fn get_octant_unchecked<P>(&self, pos: P) -> Option<&Octree<E>>
    where
        P: Borrow<Point3<u8>>,
    {
        match self.data {
            Empty => None,
            Leaf(_) => Some(self),
            Node(ref octants) => {
                let index: usize = self.bounds.get_octant_index(pos.borrow());
                octants[index].get_octant_unchecked(pos)
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

    pub(crate) fn set_height(mut self, height: u32) -> Self {
        self.height = height;
        self
    }

    pub fn root_point(&self) -> Point3<Number> {
        self.bounds.bottom_left()
    }
    pub fn bounds<'a>(&'a self) -> &'a OctantDimensions {
        &self.bounds
    }

    pub fn data<'a>(&'a self) -> &'a OctreeData<E> {
        &self.data
    }
    pub fn set_data(&mut self, data: OctreeData<E>) {
        self.data = data;
    }

    pub fn is_empty(&self) -> bool {
        match self.data {
            Empty => true,
            _ => false,
        }
    }

    pub fn is_node(&self) -> bool {
        match self.data {
            Node(_) => true,
            _ => false,
        }
    }

    pub fn map<EF, LF, NF, O>(&self, empty_fn: EF, leaf_fn: LF, node_fn: NF) -> O
    where
        EF: FnOnce() -> O,
        LF: FnOnce(&E) -> O,
        NF: FnOnce(&[Arc<Octree<E>>; 8]) -> O,
    {
        match &self.data {
            Empty => empty_fn(),
            Leaf(arc_e) => leaf_fn(arc_e.as_ref()),
            Node(ref nodes) => node_fn(nodes),
        }
    }

    pub fn mut_children<'a>(&'a mut self) -> &'a mut [Arc<Octree<E>>; 8] {
        match self.data {
            Node(ref mut nodes) => nodes,
            _ => panic!("Unexpected mut_children() on Leaf or Empty data"),
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
                let index: usize = self.bounds.get_octant_index(pos.borrow());
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
                let index: usize = self.bounds.get_octant_index(pos.borrow());
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

    pub fn compress(&mut self) {
        match &mut self.data {
            Node(ref mut nodes) => {
                unsafe {
                    par_for_each_node(nodes, |arc_child: &mut Arc<Octree<E>>| {
                        Arc::get_mut(arc_child).unwrap().compress();
                    });
                }
                let mut data = nodes.iter().map(|node| &node.data);
                if data.next().map_or(true, |head| data.all(|ele| head == ele)) {
                    self.data = nodes[0].data.clone();
                }
            }
            _ => (),
        }
    }

    fn create_sub_nodes<P>(&self, pos: P, elem: OctreeData<E>, default: OctreeData<E>) -> Self
    where
        P: Borrow<Point3<Number>>,
    {
        let modified_octant = self.bounds.get_octant(pos.borrow());

        let octree_nodes: [Arc<Octree<E>>; 8] = array_init::array_init(|i| {
            let octant =
                Octant::from_usize(i).expect("Tried to create more than 8 elements in an octree");

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

    pub fn neighbor_iter<'a>(&'a self) -> NeighborOctreeIter<'a, E> {
        let mut stack = VecDeque::new();
        stack.push_back(OctreeContext::with_root(self));
        NeighborOctreeIter { stack }
    }

    pub fn par_neighbor_iter<'a>(&'a self) -> ParallelNeighborOctreeIter<'a, E> {
        ParallelNeighborOctreeIter {
            curr_ctxt: OctreeContext::with_root(self),
        }
    }
}

// A stack of octrees up to the root tree at the top level, acts as a linked list of references.
//#[derive(Clone)]
pub struct OctreeContext<'a, E> {
    tree: &'a Octree<E>,
    parent: Option<Arc<OctreeContext<'a, E>>>,
}
impl<'a, E: PartialEq> OctreeContext<'a, E> {
    pub fn with_root(tree: &'a Octree<E>) -> Self {
        OctreeContext { parent: None, tree }
    }

    pub fn current_value(&self) -> &'a Octree<E> {
        self.tree
    }

    pub fn check_position<P>(&self, pos: P) -> bool
    where
        P: Borrow<Point3<Number>>,
    {
        self.parent.as_ref().map_or(false, |parent_ctx| {
            if parent_ctx.tree.outside_bounds(pos.borrow()) {
                parent_ctx.check_position(pos)
            } else {
                parent_ctx.tree.get(pos).is_some()
            }
        })
    }

    pub fn append_child(self, child: &'a Octree<E>) -> Self {
        OctreeContext {
            parent: Some(Arc::new(self)),
            tree: child,
        }
    }
}
impl<'a, E> Clone for OctreeContext<'a, E> {
    fn clone(&self) -> Self {
        OctreeContext {
            tree: self.tree,
            parent: self.parent.clone(),
        }
    }
}

pub struct ParallelNeighborOctreeIter<'a, E> {
    curr_ctxt: OctreeContext<'a, E>,
}
impl<'a, E: Send + Sync + PartialEq> ParallelIterator for ParallelNeighborOctreeIter<'a, E> {
    type Item = OctreeContext<'a, E>;

    fn drive_unindexed<C>(self, consumer: C) -> C::Result
    where
        C: UnindexedConsumer<Self::Item>,
    {
        let tree = &self.curr_ctxt.tree;
        let data = &tree.data;
        let child_ctxt = self.curr_ctxt.clone().append_child(tree);
        match data {
            Empty => consumer.into_folder().complete(),
            Leaf(_) => consumer.into_folder().consume(child_ctxt).complete(),
            Node(nodes) => parallel_drive_node_children(&nodes, consumer, |_, consumer| {
                (ParallelNeighborOctreeIter {
                    curr_ctxt: child_ctxt.clone(),
                })
                .into_par_iter()
                .drive_unindexed(consumer)
            }),
        }
    }
}

pub struct NeighborOctreeIter<'a, E> {
    stack: VecDeque<OctreeContext<'a, E>>,
}
impl<'a, E: fmt::Debug + PartialEq> Iterator for NeighborOctreeIter<'a, E> {
    type Item = OctreeContext<'a, E>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let mut opt_context = self.stack.pop_front();
            while let Some(context) = opt_context {
                match &context.tree.data {
                    Empty => {}
                    Leaf(_) => {
                        //let context = context;
                        let tree = context.tree;
                        return Some(context.append_child(tree));
                    }
                    Node(ref children) => {
                        let children_iter = children
                            .into_iter()
                            .map(|arc| arc.as_ref())
                            .filter(|node_ref| !node_ref.is_empty())
                            .map(|node| context.clone().append_child(node));
                        self.stack.extend(children_iter);
                    }
                }
                opt_context = self.stack.pop_front();
            }
            return None;
        }
    }
}

impl<E: Send + Sync> IntoParallelIterator for Octree<E> {
    type Iter = ParallelOctreeIter<E>;
    type Item = <<Octree<E> as IntoParallelIterator>::Iter as ParallelIterator>::Item;

    fn into_par_iter(self) -> Self::Iter {
        ParallelOctreeIter { node: self }
    }
}
impl<'a, E: Send + Sync> IntoParallelIterator for &'a Octree<E> {
    type Iter = ParallelOctreeRefIter<'a, E>;
    type Item = <<&'a Octree<E> as IntoParallelIterator>::Iter as ParallelIterator>::Item;

    fn into_par_iter(self) -> Self::Iter {
        ParallelOctreeRefIter { node: self }
    }
}

pub unsafe fn par_for_each_node<E, F>(nodes: &mut [Arc<Octree<E>>; 8], node_op: F)
where
    F: Fn(&mut Arc<Octree<E>>) -> () + Send + Sync,
{
    let node = MultiThreadMutPtr::new(nodes.as_mut_ptr());
    rayon::join(
        || {
            rayon::join(
                || {
                    rayon::join({ || node_op(node.element_at(0)) }, {
                        || node_op(node.element_at(1))
                    })
                },
                || {
                    rayon::join(
                        || node_op(node.element_at(2)),
                        || node_op(node.element_at(3)),
                    )
                },
            )
        },
        || {
            rayon::join(
                || {
                    rayon::join(
                        || node_op(node.element_at(4)),
                        || node_op(node.element_at(5)),
                    )
                },
                || {
                    rayon::join(
                        || node_op(node.element_at(6)),
                        || node_op(node.element_at(7)),
                    )
                },
            )
        },
    );
}

pub fn parallel_drive_node_children<'a, ITEM, E, C, F>(
    nodes: &'a [Arc<Octree<E>>; 8],
    consumer: C,
    handle_child: F,
) -> C::Result
where
    E: Send + Sync,
    C: UnindexedConsumer<ITEM>,
    F: Fn(&'a Octree<E>, C) -> C::Result + Send + Sync,
{
    let reducer = consumer.to_reducer();
    let (left_half, right_half) = (consumer.split_off_left(), consumer);
    let (ll_quarter, lr_quarter, rl_quarter, rr_quarter) = (
        left_half.split_off_left(),
        left_half,
        right_half.split_off_left(),
        right_half,
    );
    let (lll_octet, llr_octet, lrl_octet, lrr_octet, rll_octet, rlr_octet, rrl_octet, rrr_octet) = (
        ll_quarter.split_off_left(),
        ll_quarter,
        lr_quarter.split_off_left(),
        lr_quarter,
        rl_quarter.split_off_left(),
        rl_quarter,
        rr_quarter.split_off_left(),
        rr_quarter,
    );
    let (left, right) = rayon::join(
        || {
            let reducer = lll_octet.to_reducer();
            let (left, right) = rayon::join(
                || {
                    let r = lll_octet.to_reducer();
                    r.reduce(
                        handle_child(&nodes[0], lll_octet),
                        handle_child(&nodes[1], llr_octet),
                    )
                },
                || {
                    let r = lrl_octet.to_reducer();
                    r.reduce(
                        handle_child(&nodes[2], lrl_octet),
                        handle_child(&nodes[3], lrr_octet),
                    )
                },
            );
            reducer.reduce(left, right)
        },
        || {
            let reducer = rll_octet.to_reducer();
            let (left, right) = rayon::join(
                || {
                    let r = rll_octet.to_reducer();
                    r.reduce(
                        handle_child(&nodes[4], rll_octet),
                        handle_child(&nodes[5], rlr_octet),
                    )
                },
                || {
                    let r = rrl_octet.to_reducer();
                    r.reduce(
                        handle_child(&nodes[6], rrl_octet),
                        handle_child(&nodes[7], rrr_octet),
                    )
                },
            );
            reducer.reduce(left, right)
        },
    );
    reducer.reduce(left, right)
}

pub struct ParallelOctreeIter<E> {
    node: Octree<E>,
}
impl<E: Send + Sync> ParallelIterator for ParallelOctreeIter<E> {
    type Item = (OctantDimensions, Arc<E>);

    fn drive_unindexed<C>(self, consumer: C) -> C::Result
    where
        C: UnindexedConsumer<Self::Item>,
    {
        match self.node.data {
            Empty => consumer.into_folder().complete(),
            Leaf(elem) => consumer
                .into_folder()
                .consume((self.node.bounds, elem))
                .complete(),
            Node(nodes) => parallel_drive_node_children(&nodes, consumer, |node, consumer| {
                node.clone().into_par_iter().drive_unindexed(consumer)
            }),
        }
    }
}

pub struct ParallelOctreeRefIter<'a, E> {
    node: &'a Octree<E>,
}
impl<'a, E: Send + Sync> ParallelIterator for ParallelOctreeRefIter<'a, E> {
    type Item = (&'a OctantDimensions, &'a E);

    fn drive_unindexed<C>(self, consumer: C) -> C::Result
    where
        C: UnindexedConsumer<Self::Item>,
    {
        match &self.node.data {
            Empty => consumer.into_folder().complete(),
            Leaf(elem) => consumer
                .into_folder()
                .consume((&self.node.bounds, elem.as_ref()))
                .complete(),
            Node(nodes) => parallel_drive_node_children(&nodes, consumer, |node, consumer| {
                node.into_par_iter().drive_unindexed(consumer)
            }),
        }
    }
}

pub struct OctreeIterator<'a, E> {
    node_stack: VecDeque<&'a Octree<E>>,
}
impl<'a, E: fmt::Debug + PartialEq> Iterator for OctreeIterator<'a, E> {
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

        assert_eq!(octree.insert(&center, 1234).get(&center), Some(&1234));
    }

    #[test]
    fn octree_element_retrieved_after_insertion_in_same_octants() {
        let p1 = Point3::new(2, 2, 2);
        let p2 = Point3::new(1, 1, 1);
        let octree: Octree<i32> = Octree::with_uniform_dimension(2)
            .insert(&p1, 1234)
            .insert(&p2, 5678);

        assert_eq!(octree.get(&p1), Some(&1234));
        assert_eq!(octree.get(&p2), Some(&5678));
    }

    #[test]
    fn octree_element_retrieved_after_inserterion_in_diff_octants() {
        let p1 = Point3::new(1, 1, 1);
        let p2 = Point3::new(7, 7, 7);
        let octree: Octree<i32> = Octree::with_uniform_dimension(3)
            .insert(&p1, 1234)
            .insert(&p2, 5678);

        assert_eq!(octree.get(&p1), Some(&1234));
        assert_eq!(octree.get(&p2), Some(&5678));
    }

    #[test]
    fn octree_insert_updates_element() {
        let p = Point3::new(1, 1, 1);
        let octree: Octree<i32> = Octree::with_uniform_dimension(4).insert(&p, 1234);

        assert_eq!(octree.get(&p), Some(&1234));

        let octree = octree.insert(&p, 5678);
        assert_eq!(octree.get(&p), Some(&5678));
    }

    #[test]
    fn octree_deletes_expected_element() {
        let p = Point3::new(4, 1, 1);
        let octree: Octree<i32> = Octree::with_uniform_dimension(5)
            .insert(Point3::new(1, 1, 1), 1234)
            .insert(Point3::new(1, 1, 2), 4567)
            .insert(&p, 7890);

        assert_eq!(octree.get(&p), Some(&7890));
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

    #[test]
    fn octree_returns_expected_octant() {
        let octree = Octree::new(Point3::origin(), Some(1234), 8);
        let octant = octree.get_octant(Point3::new(1, 1, 1));
        assert!(octant.is_some());
        assert_eq!(octant.unwrap().bounds.diameter(), 256);
        assert_eq!(octant.unwrap().bounds.bottom_left, Point3::new(0, 0, 0));
    }

    #[test]
    fn octree_returns_correct_sub_octant() {
        let octree = Octree::new(Point3::origin(), Some(1234), 8);
        let octree = octree.delete(Point3::new(1, 1, 1));
        let octant = octree.get_octant(Point3::new(152, 140, 130));
        assert_eq!(octant.unwrap().bounds.diameter(), 128);
        assert_eq!(
            octant.unwrap().bounds.bottom_left,
            Point3::new(128, 128, 128)
        )
    }

    #[test]
    fn octree_reutrns_none_on_empty_octant() {
        let octree: Octree<usize> = Octree::new(Point3::origin(), None, 8);
        assert!(octree.get_octant(Point3::new(1, 2, 3)).is_none())
    }
}
