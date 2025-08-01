/// RAII handle, which allows perform some operation on drop
pub struct Handle {
    drop: Option<Box<dyn FnOnce()>>,
}

impl<F> From<F> for Handle
where
    F: FnOnce() + 'static,
{
    fn from(value: F) -> Self {
        let drop = Box::new(value);

        Self { drop: Some(drop) }
    }
}

impl Drop for Handle {
    fn drop(&mut self) {
        if let Some(drop) = self.drop.take() {
            drop();
        }
    }
}

unsafe impl Send for Handle {}

unsafe impl Sync for Handle {}
