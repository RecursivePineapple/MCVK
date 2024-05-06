pub trait Extract: Default {
    fn extract(&mut self) -> Self;
}

impl<T: Default> Extract for T {
    fn extract(&mut self) -> Self {
        std::mem::take(self)
    }
}
