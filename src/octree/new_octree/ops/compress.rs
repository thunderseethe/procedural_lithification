use crate::octree::new_octree::*;
use amethyst::core::nalgebra::Scalar;

pub trait Compress {
    fn compress_nodes(self) -> Self;
}
impl<O> Compress for OctreeLevel<O>
where
    O: HasData + OctreeTypes,
    <O as HasData>::Data: PartialEq,
{
    fn compress_nodes(self) -> Self {
        use crate::octree::new_octree::LevelData::*;
        match self.data {
            Node(ref nodes) => {
                let mut iter = nodes.iter().map(|node| node.data());
                if iter.next().map_or(true, |head| iter.all(|ele| head == ele)) {
                    let head = nodes[0].data();
                    self.with_data(if head.is_empty() {
                        LevelData::empty()
                    } else if head.is_leaf() {
                        LevelData::leaf(Rc::clone(head.get_leaf()))
                    } else {
                        panic!("Attempted to compress Node(..) node which should be impossible.");
                    })
                } else {
                    self
                }
            }
            _ => self,
        }
    }
}
impl<E, N: Scalar> Compress for OctreeBase<E, N> {
    // Compress is the identity function for BaseNodes
    fn compress_nodes(self) -> Self {
        self
    }
}
