use crate::octree::new_octree::{
    ElementType, LevelData, Number, OctreeBase, OctreeLevel, OctreeTypes, Ref,
};

pub trait Map: ElementType {
    type Children: Map;

    /// Maps over a single level of the Octree using 3 functions to handle each possible case.
    /// This should not be confused with the more traditional concept of map from Functor.
    /// To accomplish something like that use `&octree.into_iter().map(...)`
    ///
    /// Example usage:
    ///
    /// ```
    /// let octree = Octree::<u32, u8, U64>::at_origin(None);
    ///
    /// let number_of_leaves = octree.map(
    ///     || 0,
    ///     |leaf_elem| 1,
    ///     |node_children| 8,
    /// );
    /// assert_eq!(number_of_leaves, 0);
    /// ```
    fn map<EFn, LFn, NFn, Output>(&self, empty_fn: EFn, leaf_fn: LFn, node_fn: NFn) -> Output
    where
        EFn: FnOnce() -> Output,
        LFn: FnOnce(&Self::Element) -> Output,
        NFn: FnOnce(&[Ref<Self::Children>; 8]) -> Output;
}

impl<O> Map for OctreeLevel<O>
where
    O: OctreeTypes + Map,
{
    type Children = O;

    fn map<EFn, LFn, NFn, Output>(&self, empty_fn: EFn, leaf_fn: LFn, node_fn: NFn) -> Output
    where
        EFn: FnOnce() -> Output,
        LFn: FnOnce(&Self::Element) -> Output,
        NFn: FnOnce(&[Ref<Self::Children>; 8]) -> Output,
    {
        use LevelData::*;
        match &self.data {
            Empty => empty_fn(),
            Leaf(ref elem) => leaf_fn(elem),
            Node(ref nodes) => node_fn(nodes),
        }
    }
}

impl<E, N> Map for OctreeBase<E, N>
where
    N: Number,
{
    type Children = ();

    fn map<EFn, LFn, NFn, Output>(&self, empty_fn: EFn, leaf_fn: LFn, node_fn: NFn) -> Output
    where
        EFn: FnOnce() -> Output,
        LFn: FnOnce(&Self::Element) -> Output,
        NFn: FnOnce(&[Ref<Self::Children>; 8]) -> Output,
    {
        match &self.data {
            None => empty_fn(),
            Some(ref elem) => leaf_fn(elem),
        }
    }
}

// This is all nonesense and exists to satisfy the borrow checker.
impl Map for () {
    type Children = ();

    fn map<EFn, LFn, NFn, Output>(&self, empty_fn: EFn, leaf_fn: LFn, node_fn: NFn) -> Output
    where
        EFn: FnOnce() -> Output,
        LFn: FnOnce(&Self::Element) -> Output,
        NFn: FnOnce(&[Ref<Self::Children>; 8]) -> Output,
    {
        empty_fn()
    }
}
impl ElementType for () {
    type Element = ();
}
