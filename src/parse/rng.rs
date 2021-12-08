use std::ops::RangeInclusive;

pub trait Rng {
    fn gen(&mut self, range: RangeInclusive<u32>) -> u32;
}

pub struct RngMock<const N: usize>(pub [u32; N]);

impl<const N: usize> Rng for RngMock<N> {
    fn gen(&mut self, _range: std::ops::RangeInclusive<u32>) -> u32 {
        self.0.rotate_left(1);
        self.0[N - 1]
    }
}
