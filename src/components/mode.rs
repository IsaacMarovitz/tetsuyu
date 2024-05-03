use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone, Copy, PartialEq)]
pub enum GBMode {
    DMG,
    CGB,
}

#[derive(Deserialize, Serialize, Debug, Clone, Copy, PartialEq)]
pub enum CCMode {
    True,
    CGB,
    GBA,
    SGB
}