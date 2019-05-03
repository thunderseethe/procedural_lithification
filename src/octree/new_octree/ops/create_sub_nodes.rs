use crate::octree::new_octree::*;

/// Trait for internal bookkeeping of Octree duing insertion and deletion.
/// Not a publicly available method.
pub trait CreateSubNodes: OctreeTypes {
    type SubData;

    fn create_sub_nodes<P>(
        &self,
        pos: P,
        elem: Option<Ref<Self::Element>>,
        default: Self::SubData,
    ) -> Self
    where
        P: Borrow<Point3<Self::Field>>;

    fn place<P>(&self, pos: P, data: Option<Ref<Self::Element>>) -> Self
    where
        P: Borrow<Point3<Self::Field>>;
}
impl<E, N: Number> CreateSubNodes for OctreeBase<E, N> {
    type SubData = ();
    fn create_sub_nodes<P>(&self, pos: P, elem: Option<Ref<Self::Element>>, default: ()) -> Self
    where
        P: Borrow<Point3<Self::Field>>,
    {
        (*self).clone()
    }

    fn place<P>(&self, _pos: P, data: Option<Ref<Self::Element>>) -> Self {
        use crate::octree::new_octree::BaseData::*;
        OctreeBase {
            data: data.map(Leaf).unwrap_or(Empty),
            ..(*self).clone()
        }
    }
}

impl<O> CreateSubNodes for OctreeLevel<O>
where
    O: OctreeTypes + HasData + New + Diameter + CreateSubNodes,
    Self::Element: PartialEq,
    <O as HasData>::Data: PartialEq,
{
    type SubData = O::Data;

    fn create_sub_nodes<P>(
        &self,
        pos: P,
        elem: Option<Ref<Self::Element>>,
        default: Self::SubData,
    ) -> Self
    where
        P: Borrow<Point3<Self::Field>>,
    {
        use crate::octree::new_octree::LevelData::Node;
        use crate::octree::octant::OctantId;
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

    fn place<P>(&self, pos: P, data: Option<Ref<Self::Element>>) -> Self
    where
        P: Borrow<Point3<Self::Field>>,
    {
        use crate::octree::new_octree::LevelData::*;
        match &self.data {
            Empty => self.create_sub_nodes(pos, data, O::Data::empty()),
            Leaf(old_elem) => {
                if data
                    .as_ref()
                    .map(|new_elem| old_elem == new_elem)
                    .unwrap_or(false)
                {
                    self.with_data(data.map(Leaf).unwrap_or(Empty))
                } else {
                    self.create_sub_nodes(pos, data, O::Data::leaf(Ref::clone(old_elem)))
                }
            }
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
