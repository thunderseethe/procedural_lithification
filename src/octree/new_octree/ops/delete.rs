use crate::octree::new_octree::*;
use std::borrow::Borrow;
use std::rc::Rc;

pub trait Delete: FieldType {
    fn delete<P>(&self, pos: P) -> Self
    where
        P: Borrow<Point3<Self::Field>>;
}

impl<O> Delete for OctreeLevel<O>
where
    O: OctreeTypes + HasData + New + Diameter + CreateSubNodes + Delete + Clone,
    <Self as ElementType>::Element: PartialEq,
    <O as HasData>::Data: PartialEq,
{
    fn delete<P>(&self, pos: P) -> Self
    where
        P: Borrow<Point3<Self::Field>>,
    {
        use crate::octree::new_octree::LevelData::*;
        match self.data {
            Empty => (*self).clone(),
            Leaf(ref elem) => self.create_sub_nodes(pos, None, O::Data::leaf(Rc::clone(elem))),
            Node(ref old_nodes) => {
                let mut nodes = old_nodes.clone();
                let index: usize = self.get_octant_index(pos.borrow());
                let old_octant = &old_nodes[index];
                nodes[index] = Rc::new(old_octant.delete(pos));
                self.with_data(Node(nodes)).compress_nodes()
            }
        }
    }
}
