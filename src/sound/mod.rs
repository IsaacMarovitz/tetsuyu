pub mod apu;
mod ch1;
mod ch2;
mod ch3;
mod ch4;

#[allow(unused_imports)]
mod prelude {
    pub use crate::sound::{apu::*, ch1::*, ch2::*, ch3::*, ch4::*};
}
