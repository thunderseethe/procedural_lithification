use crate::octree::octant::OctantId;
use crate::octree::*;

/// Trait for internal bookkeeping of Octree duing insertion and deletion.
/// Not a publicly available method.
///
pub trait CreateSubNodes: OctreeTypes {
    type SubData;

    /// create_sub_nodes will copy Empty and Leaf nodes into a new Node nodes and then modify the relevant child denoted by pos
    fn create_sub_nodes<P>(
        &self,
        pos: P,
        elem: Option<Self::Element>,
        default: Self::SubData,
    ) -> Self
    where
        P: Borrow<Point3<FieldOf<Self>>>;

    /// Place behaves as an abstraction over insertion and deletion.
    /// It performs the recursion into the tree based on pos.
    /// For deletion data will be None
    /// For insertion data will be Some
    fn place<P>(&self, pos: P, data: Option<Self::Element>) -> Self
    where
        P: Borrow<Point3<FieldOf<Self>>>;
}
impl<E: Clone, N: Number> CreateSubNodes for OctreeBase<E, N> {
    /// We can't subdivide OctreeBase so it's SubData is unit
    type SubData = ();

    #[inline]
    fn create_sub_nodes<P>(&self, _pos: P, _elem: Option<Self::Element>, _default: ()) -> Self
    where
        P: Borrow<Point3<FieldOf<Self>>>,
    {
        (*self).clone()
    }

    /// There is no more recursion to be done so we return a copy of our node with updated data
    #[inline]
    fn place<P>(&self, _pos: P, data: Option<Self::Element>) -> Self {
        self.with_data(data)
    }
}

impl<O> CreateSubNodes for OctreeLevel<O>
where
    O: OctreeTypes + HasData + New + Diameter + CreateSubNodes,
    ElementOf<O>: PartialEq + Clone,
    DataOf<O>: PartialEq + Clone,
    DataOf<Self>: From<DataOf<O>>,
{
    type SubData = O::Data;

    #[inline]
    fn create_sub_nodes<P>(
        &self,
        pos: P,
        elem: Option<ElementOf<O>>,
        default: Self::SubData,
    ) -> Self
    where
        P: Borrow<Point3<FieldOf<O>>>,
    {
        use crate::octree::LevelData::Node;
        let modified_octant = self.get_octant(pos.borrow());
        let octree_nodes: [Ref<O>; 8] = array_init::from_iter(OctantId::iter().map(|octant| {
            let data = default.clone();
            let sub_bottom_left = octant.sub_octant_bottom_left(self.bottom_left, O::diameter());
            let octree = O::new(data, sub_bottom_left);
            let octree = if modified_octant == octant {
                octree.place(pos.borrow(), elem.clone())
            } else {
                octree
            };
            Ref::new(octree)
        }))
        .expect("Failed to construct array from iterator");
        self.with_data(Node(octree_nodes)).compress_nodes()
    }

    #[inline]
    fn place<P>(&self, pos: P, data: Option<ElementOf<O>>) -> Self
    where
        P: Borrow<Point3<FieldOf<O>>>,
    {
        use crate::octree::LevelData::*;
        match &self.data {
            // Create 8 empty children and recurse into one to place data
            Empty => self.create_sub_nodes(pos, data, O::Data::empty()),
            // If our old leaf and new leaf are equal we return our node with new leaf.
            // The reason we return new_leaf is in case two elements that equate have different semantics.
            // E.g. an Entry<K, V> struct that is equal when it's key K is equal
            // If our leaves are not equal we create sub nodes containing old leaf and recurse into the relevant one to place data.
            Leaf(old_elem) => {
                if data
                    .as_ref()
                    .map(|new_elem| old_elem == new_elem)
                    .unwrap_or(false)
                {
                    self.with_data(data.map(Leaf).unwrap_or(Empty))
                } else {
                    self.create_sub_nodes(pos, data, O::Data::leaf(old_elem.clone()))
                }
            }
            // This is the simplest case, we select the octant pos is in and then place our element in it.
            Node(ref old_nodes) => {
                let mut nodes = old_nodes.clone();
                let index: usize = self.get_octant_index(pos.borrow());
                let old_octant: &Ref<O> = &old_nodes[index];
                nodes[index] = Ref::new(old_octant.place(pos, data));
                self.with_data(Node(nodes)).compress_nodes()
            }
        }
    }
}
