use std::ops::Range;

/// Implement this for any custom random number generator
pub trait Rng: Send + Sync + 'static {
    fn gen_range(&mut self, bounds: Range<usize>) -> usize;
}
pub trait RngProvider: Rng {
    fn init() -> Self;
}

#[cfg(feature = "nanorand")]
mod default_rng {
    use nanorand::WyRand;
    use std::ops::Range;

    pub struct DefaultRng(WyRand);

    impl super::RngProvider for DefaultRng {
        fn init() -> Self {
            DefaultRng(WyRand::new())
        }
    }

    impl super::Rng for DefaultRng {
        fn gen_range(&mut self, bounds: Range<usize>) -> usize {
            use nanorand::Rng;
            self.0.generate_range(bounds)
        }
    }
}

#[cfg(feature = "nanorand")]
pub use default_rng::*;
