#[derive(Clone)]
pub struct MultiThreadMutPtr<T>(pub *mut T);
impl<T> MultiThreadMutPtr<T> {
    pub fn new(ptr: *mut T) -> Self {
        MultiThreadMutPtr(ptr)
    }
}
unsafe impl<T> Send for MultiThreadMutPtr<T> {}
unsafe impl<T> Sync for MultiThreadMutPtr<T> {}
