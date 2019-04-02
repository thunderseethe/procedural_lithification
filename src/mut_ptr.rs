/// This is all very unsafe and should be used really carefully.
pub struct MultiThreadMutPtr<T>(pub *mut T);
impl<T> MultiThreadMutPtr<T> {
    pub fn new(ptr: *mut T) -> Self {
        MultiThreadMutPtr(ptr)
    }

    pub unsafe fn element_at(&self, index: usize) -> &mut T {
        self.0.add(index).as_mut().unwrap()
    }
}
unsafe impl<T> Send for MultiThreadMutPtr<T> {}
unsafe impl<T> Sync for MultiThreadMutPtr<T> {}

impl<T> Clone for MultiThreadMutPtr<T> {
    fn clone(&self) -> Self {
        MultiThreadMutPtr::new(self.0)
    }
}
impl<T> Copy for MultiThreadMutPtr<T> {}
