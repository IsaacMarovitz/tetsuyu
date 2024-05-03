const GB_COLOR_LUT_LEN: usize = 0x8000;
const GAMMA: f32 = 2.2;
const CGB_COLOR_CURVE: [u8; 32] = [ 0, 6, 12, 20, 28, 36, 45, 56, 66, 76, 88, 100, 113, 125, 137, 149, 161, 172, 182, 192, 202, 210, 218, 225, 232, 238, 243, 247, 250, 252, 254, 255 ];

// Adapted from SameBoy Color Correction
pub struct ColorCorrection {
    pub true_color_lut: [[u8; 3]; GB_COLOR_LUT_LEN],
    pub cgb_color_lut: [[u8; 3]; GB_COLOR_LUT_LEN]
}

impl ColorCorrection {
    pub fn new() -> Self {
        let mut true_color_lut = [[0; 3]; GB_COLOR_LUT_LEN];
        let mut cgb_color_lut = [[0; 3]; GB_COLOR_LUT_LEN];

        for i in 0..GB_COLOR_LUT_LEN {
            let r = (i & 0x1F) as u8;
            let g = ((i >> 5) & 0x1F) as u8;
            let b = ((i >> 10) & 0x1F) as u8;

            true_color_lut[i] = Self::to_true_color(r, g, b);
            cgb_color_lut[i] = Self::to_cgb_color(r, g, b);
        }

        Self {
            true_color_lut,
            cgb_color_lut
        }
    }
    
    fn to_true_color(r: u8, g: u8, b: u8) -> [u8; 3] {
        let r = (r * 0xFF + 0xF) / 0x1F;
        let g = (g * 0xFF + 0xF) / 0x1F;
        let b = (b * 0xFF + 0xF) / 0x1F;
        [r, g, b]
    }

    fn to_cgb_color(r: u8, g: u8, b: u8) -> [u8; 3] {
        let r = CGB_COLOR_CURVE[r as usize];
        let mut g = CGB_COLOR_CURVE[g as usize];
        let b = CGB_COLOR_CURVE[b as usize];

        if g != b {
            g = ((((g as f32 / 255.0).powf(GAMMA) * 3.0 + (b as f32 / 255.0).powf(GAMMA)) / 4.0).powf(1.0 / GAMMA) * 255.0).round() as u8;
        }

        [r, g, b]
    }
}