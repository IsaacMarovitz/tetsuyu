use crate::components::ppu::bgpi::BGPI;
use crate::components::ppu::cc::ColorCorrection;
use crate::components::ppu::fetcher::*;
use crate::components::ppu::structs::*;
use crate::components::prelude::*;
use crate::config::{Color, Config, PPUConfig, Palette};
use crate::framebuffer::FramebufferWriter;
use crate::hw::interrupt::Interrupts;

/// RGBA (4 bytes) per pixel
pub const FRAMEBUFFER_SIZE: usize = 4 * SCREEN_W * SCREEN_H;
pub const SCREEN_W: usize = 160;
pub const SCREEN_H: usize = 144;

/// Which flavour of the DMG OAM corruption a CPU M-cycle triggers. The row the
/// PPU is scanning is decided by the PPU; this only says *how* it corrupts.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum OamGlitch {
    /// 16-bit inc/dec (INC/DEC rp, PUSH's dec sp, ...) — a write corruption
    /// applied at the row asserted by the IDU at the M-cycle start (pre-dots).
    Increase,
    /// An actual OAM store during mode 2 (e.g. PUSH's push of a byte) — a write
    /// corruption applied at the row reached by the store transfer (post-dots).
    Write,
    /// A plain read of OAM during mode 2 (e.g. a stack-pop's high-byte read).
    Read,
    /// A read that shares its M-cycle with the IDU increment (a stack-pop's
    /// low-byte read) — the three-row "read + write" pattern.
    ReadIncrease,
}

const LY_153_ROLLOVER: u32 = 4;

/// Dots before a line formally starts at which its mode-2 (OAM) STAT source
/// is already asserted. On hardware the OAM STAT interrupt for a line is
/// raised at the tail of the previous line's HBlank, giving a mode-2 STAT
/// handler time to run before that line's Mode 3. This is the real hardware
/// edge position, measured against the mealybug m3 mode-2 handlers.
const MODE2_LOOKAHEAD: u32 = 4;

pub struct PPU {
    mode: GBMode,
    rom_is_cgb: bool,
    boot_rom_enabled: bool,
    ppu_config: PPUConfig,
    cc: ColorCorrection,
    ppu_mode: PPUMode,
    cycle_count: u32,
    scy: u8,
    scx: u8,
    ly: u8,
    lc: u8,
    wy: u8,
    wx: u8,
    wly: u8,
    bgp: u8,
    obp0: u8,
    obp1: u8,
    lcdc: LCDC,
    lcds: LCDS,
    bcps: BGPI,
    bcpd: [u8; 64],
    ocps: BGPI,
    ocpd: [u8; 64],
    vram: [u8; 0x4000],
    vram_bank: usize,
    oam: [u8; 0xA0],
    opri: bool,
    pub interrupts: Interrupts,
    framebuffer: FramebufferWriter,
    pub entered_hblank: bool,
    stat_line: bool,
    pipeline: Pipeline,
    /// Latched copy of `window_active` for the whole line: true once the
    /// window activated during this line's Mode 3, so `inc_ly` can advance the
    /// window line counter exactly when hardware does. Distinct from
    /// `window_active`, which is cleared each new Mode 3.
    window_line_active: bool,
    /// The window "Y condition" (Pandocs): cleared each VBlank, and latched
    /// true at the start of any scanline where WY == LY, staying true for the
    /// rest of the frame. The window can only trigger while this holds — it is
    /// sampled once per line, not compared live against LY.
    window_y_condition: bool,
    /// DMG BGP write artifact: `old | new`, visible for exactly one dot.
    bgp_glitch: Option<u8>,
    /// Set when the LCD is enabled, cleared when the first Mode 3 of the frame
    /// begins. During this window the freshly-enabled PPU scans OAM for line 0
    /// but STAT still reports mode 0 (and the mode-2 STAT source does not fire):
    /// the "mode-0 readback" first-frame quirk pinned by mooneye stat_lyc_onoff.
    first_line_after_on: bool,
}

impl PPU {
    pub fn new(config: Config, framebuffer: FramebufferWriter, rom_is_cgb: bool) -> Self {
        Self {
            mode: config.mode,
            rom_is_cgb,
            boot_rom_enabled: true,
            ppu_config: config.ppu_config,
            cc: ColorCorrection::new(),
            ppu_mode: PPUMode::OAMScan,
            cycle_count: 0,
            scy: 0x00,
            scx: 0x00,
            ly: 0x00,
            lc: 0x00,
            wy: 0x00,
            wx: 0x00,
            wly: 0x00,
            bgp: 0x00,
            obp0: 0x00,
            obp1: 0x01,
            lcdc: LCDC::empty(),
            lcds: LCDS::empty(),
            bcps: BGPI::new(),
            bcpd: [0; 64],
            ocps: BGPI::new(),
            ocpd: [0; 64],
            vram: [0; 0x4000],
            vram_bank: 0,
            oam: [0; 0xA0],
            opri: true,
            interrupts: Interrupts::empty(),
            framebuffer,
            entered_hblank: false,
            stat_line: false,
            pipeline: Pipeline::new(),
            window_line_active: false,
            window_y_condition: false,
            bgp_glitch: None,
            first_line_after_on: false,
        }
    }

    pub fn cycle(&mut self, cycles: u32) {
        if !self.lcdc.contains(LCDC::LCD_ENABLE) {
            return;
        }

        for _ in 0..cycles {
            self.dot();
        }
    }

    /// Advance the PPU exactly one dot. `cycle_count` is the dot position
    /// within the current line (0..456); LY is the line. Mode is derived from
    /// the dot position rather than tracked by ad-hoc transition events, so
    /// every mode edge — and the STAT interrupt it drives — lands on the exact
    /// dot hardware produces it.
    fn dot(&mut self) {
        // Mode 3 runs its own per-dot renderer; every other mode is idle
        // between the position-driven edges handled below.
        if self.ppu_mode == PPUMode::Draw {
            self.mode3_dot();
        }

        self.cycle_count += 1;

        match self.ppu_mode {
            PPUMode::OAMScan if self.cycle_count == 80 => {
                // 80 dots of OAM scan complete: begin Mode 3.
                self.ppu_mode = PPUMode::Draw;
                // The first-frame mode-0 readback window ends here.
                self.first_line_after_on = false;
                self.start_mode3_fifo();
                self.update_stat_line();
            }
            PPUMode::HBlank if self.cycle_count == 456 - MODE2_LOOKAHEAD && self.ly < 143 => {
                // Hardware asserts the *next* line's mode-2 STAT condition a
                // few dots before the line formally starts (the OAM STAT source
                // is evaluated against the upcoming line at the tail of HBlank).
                // This is a real hardware edge, not a timing fudge: the mode-2
                // interrupt handler must be able to run before the next line's
                // Mode 3.
                self.assert_early_oam_stat();
            }
            _ => {}
        }

        // End of a line.
        if self.cycle_count >= 456 {
            self.cycle_count = 0;
            self.next_line();
        }
    }

    /// Advance to the next line: bump LY, pick the new mode, re-latch LYC and
    /// the STAT line.
    fn next_line(&mut self) {
        self.inc_ly();

        if self.ly > 153 {
            // Wrap back to the top of the frame.
            self.reset_ly();
            self.ppu_mode = PPUMode::OAMScan;
        } else if self.ly == 144 {
            self.ppu_mode = PPUMode::VBlank;
            self.interrupts |= Interrupts::V_BLANK;
            self.framebuffer.submit_frame();
            // Y condition is cleared each VBlank (Pandocs).
            self.window_y_condition = false;
        } else if self.ly < 144 {
            self.ppu_mode = PPUMode::OAMScan;
        }
        // 145..=153 stay in VBlank.

        self.check_lyc();
    }

    /// The mode-2 STAT source, asserted early (during the tail of the previous
    /// line's HBlank) for the upcoming line. Only the mode-2 select bit can
    /// raise the line here; the full re-evaluation happens at `next_line`.
    fn assert_early_oam_stat(&mut self) {
        if self.lcds.contains(LCDS::MODE_2_SELECT) && !self.stat_line {
            self.interrupts |= Interrupts::LCD;
            self.stat_line = true;
        }
    }

    fn check_lyc(&mut self) {
        // The LY=LYC comparator only runs while the PPU is on. With the LCD
        // off the comparison "clock" is stopped: the LYC_EQUALS bit is frozen
        // at its last value and neither an LYC write nor the forced LY=0 may
        // change it (mooneye stat_lyc_onoff). It is re-evaluated when the PPU
        // is turned back on.
        if !self.lcdc.contains(LCDC::LCD_ENABLE) {
            return;
        }
        if self.ly == self.lc {
            self.lcds |= LCDS::LYC_EQUALS;
        } else {
            self.lcds &= !LCDS::LYC_EQUALS;
        }

        self.update_stat_line();
    }

    fn update_stat_line(&mut self) {
        // All enabled STAT sources feed one internal line; the LCD interrupt
        // fires only on its rising edge (STAT blocking). During the first-line
        // mode-0 readback window the mode-2 source is suppressed even though the
        // PPU is internally scanning OAM (mooneye stat_lyc_onoff).
        let mode2 = self.ppu_mode == PPUMode::OAMScan && !self.first_line_after_on;
        let line = (self.lcds.contains(LCDS::LYC_SELECT) && self.lcds.contains(LCDS::LYC_EQUALS))
            || (self.lcds.contains(LCDS::MODE_0_SELECT) && self.ppu_mode == PPUMode::HBlank)
            || (self.lcds.contains(LCDS::MODE_1_SELECT) && self.ppu_mode == PPUMode::VBlank)
            || (self.lcds.contains(LCDS::MODE_2_SELECT) && mode2);

        if line && !self.stat_line {
            self.interrupts |= Interrupts::LCD;
        }
        self.stat_line = line;
    }

    /// Set up the pixel pipeline for one Mode 3 (called at the OAMScan→Draw
    /// edge): latches the window Y condition and hands the pipeline this line's
    /// register snapshot + OAM so it can select sprites and reset itself.
    fn start_mode3_fifo(&mut self) {
        self.window_line_active = false;
        // Latch the Y condition for this scanline: once WY == LY has been seen
        // this frame it stays set (Pandocs "Window rendering criteria").
        if self.wy == self.ly {
            self.window_y_condition = true;
        }
        
        self.bgp_glitch = None;
        let r = self.regs();
        self.pipeline.start_line(&self.oam, r);
    }

    /// Register snapshot handed to the pixel pipeline for a dot / line start.
    fn regs(&self) -> Regs {
        Regs {
            mode: self.mode,
            lcdc: self.lcdc,
            scx: self.scx,
            scy: self.scy,
            ly: self.ly,
            wx: self.wx,
            wly: self.wly,
            window_y: self.window_y_condition,
            opri: self.opri,
        }
    }

    /// Advance the renderer one dot: drive the pixel pipeline and paint any
    /// pixel it emits. All fetch/FIFO/sprite state lives in the pipeline now.
    fn mode3_dot(&mut self) {
        let r = self.regs();
        if let Some(e) = self.pipeline.tick(&self.vram, r) {
            self.emit_pixel(e.x, e.bg, e.obj);
        }

        // The window's WLY counter advances on any line the window activated.
        if self.pipeline.window_triggered() {
            self.window_line_active = true;
        }

        // The BGP write artifact lives for exactly the one dot after the write
        // resolves; whether or not a pixel was emitted on it, it's gone now.
        self.bgp_glitch = None;

        if self.pipeline.finished() {
            self.ppu_mode = PPUMode::HBlank;
            self.entered_hblank = true;
            self.update_stat_line();
        }
    }

    /// Resolve BG-vs-OBJ priority for one pixel and write it.
    fn emit_pixel(&mut self, x: usize, bg: BgPixel, obj: ObjPixel) {
        let bg_attr = Attributes::from_bits_retain(bg.cgb_attr);
        let bg_disabled = !self.use_cgb_mode() && !self.lcdc.contains(LCDC::WINDOW_PRIORITY);
        let bg_color = if bg_disabled { 0 } else { bg.color };

        let prio = if bg_color == 0 {
            Priority::Color0
        } else if bg_attr.contains(Attributes::PRIORITY) {
            Priority::Priority
        } else {
            Priority::Normal
        };

        let obj_present = obj.color != 0 && self.lcdc.contains(LCDC::OBJ_ENABLE);
        let bg_wins = if !obj_present {
            true
        } else {
            match self.mode {
                GBMode::CGB => {
                    if self.lcdc.contains(LCDC::WINDOW_PRIORITY) {
                        if prio == Priority::Color0 {
                            false
                        } else if !obj.bg_prio && prio != Priority::Priority {
                            false
                        } else {
                            true
                        }
                    } else {
                        obj.bg_prio && prio != Priority::Color0
                    }
                }
                GBMode::DMG => obj.bg_prio && prio != Priority::Color0,
            }
        };

        if bg_wins {
            self.emit_bg(x, bg_color, bg_attr, bg_disabled);
        } else {
            self.emit_obj(x, obj);
        }
    }

    fn emit_bg(&mut self, x: usize, color: u8, attr: Attributes, bg_disabled: bool) {
        if self.mode == GBMode::CGB {
            if bg_disabled {
                self.set_rgb_mapped(x, 0x7FFF);
                return;
            }
            let palette_no = if self.use_cgb_mode() {
                (attr.bits() & 0b0000_0111) as usize
            } else {
                0
            };
            let final_color = if !self.use_cgb_mode() {
                ((self.bgp >> (color * 2)) & 0x03) as usize
            } else {
                color as usize
            };
            let pa = palette_no * 8 + final_color * 2;
            let c = (self.bcpd[pa] as u16) | ((self.bcpd[pa + 1] as u16) << 8) & 0x7FFF;
            self.set_rgb_mapped(x, c);
        } else {
            // The pixel emitted on the dot a BGP write lands shows old|new.
            let pal = self.bgp_glitch.unwrap_or(self.bgp);
            let c = Self::grey_to_l(self.ppu_config.palette, pal, color as usize);
            self.framebuffer
                .set_pixel(c.r(), c.g(), c.b(), x, self.ly as usize);
        }
    }

    fn emit_obj(&mut self, x: usize, obj: ObjPixel) {
        if self.mode == GBMode::CGB {
            let palette_no = if self.use_cgb_mode() {
                (obj.cgb_attr & 0b0000_0111) as usize
            } else if obj.palette {
                1
            } else {
                0
            };
            let final_color = if !self.use_cgb_mode() {
                let obp = if obj.palette { self.obp1 } else { self.obp0 };
                ((obp >> (obj.color * 2)) & 0x03) as usize
            } else {
                obj.color as usize
            };
            let pa = palette_no * 8 + final_color * 2;
            let c = (self.ocpd[pa] as u16) | ((self.ocpd[pa + 1] as u16) << 8) & 0x7FFF;
            self.set_rgb_mapped(x, c);
        } else {
            let c = if obj.palette {
                Self::grey_to_l(self.ppu_config.palette, self.obp1, obj.color as usize)
            } else {
                Self::grey_to_l(self.ppu_config.palette, self.obp0, obj.color as usize)
            };
            self.framebuffer
                .set_pixel(c.r(), c.g(), c.b(), x, self.ly as usize);
        }
    }

    fn inc_ly(&mut self) {
        // The window's internal line counter advances once per line on which
        // the window actually activated during Mode 3 (tracked by
        // `window_line_active`, latched when the trigger fires), never from a
        // fresh LCDC read: a line where WIN_EN is toggled off before HBlank
        // must not advance it even though the enable bit reads high now
        // (m2_win_en_toggle). The latch is consumed here so VBlank lines (no
        // Mode 3, no re-latch) never advance it.
        if self.window_line_active {
            self.wly += 1;
        }
        self.window_line_active = false;

        self.ly += 1;
    }

    fn reset_ly(&mut self) {
        self.ly = 0;
        self.wly = 0;
    }

    fn grey_to_l(palette: Palette, v: u8, i: usize) -> Color {
        match v >> (2 * i) & 0x03 {
            0x00 => palette.light,
            0x01 => palette.light_gray,
            0x02 => palette.dark_gray,
            _ => palette.dark,
        }
    }

    fn set_rgb_mapped(&mut self, x: usize, color: u16) {
        let color = match self.ppu_config.cc_mode {
            CCMode::True => self.cc.true_color_lut[color as usize],
            CCMode::CGB => self.cc.cgb_color_lut[color as usize],
            CCMode::GBA => self.cc.gba_color_lut[color as usize],
            CCMode::SGB => self.cc.sgb_color_lut[color as usize],
        };

        self.framebuffer
            .set_pixel(color[0], color[1], color[2], x, self.ly as usize);
    }

    pub fn disable_boot_rom(&mut self) {
        self.boot_rom_enabled = false;
    }

    /// Drain the interrupt requests and the HBlank edge produced since the last
    /// call, clearing them. Used by the peer-chip bus to build its `Ticked`
    /// result instead of reaching into the public fields.
    pub fn take_events(&mut self) -> (u8, bool) {
        let irq = self.interrupts.bits();
        self.interrupts = Interrupts::empty();
        let hblank = self.entered_hblank;
        self.entered_hblank = false;
        (irq, hblank)
    }

    fn use_cgb_mode(&self) -> bool {
        if self.boot_rom_enabled {
            true
        } else {
            self.rom_is_cgb
        }
    }

    fn read_vram(&self, a: u16, bank: usize) -> u8 {
        self.vram[(bank * 0x2000) + a as usize - 0x8000]
    }

    fn write_vram(&mut self, a: u16, v: u8, bank: usize) {
        self.vram[(bank * 0x2000) + a as usize - 0x8000] = v;
    }

    pub fn write_oam(&mut self, index: u16, v: u8) {
        self.oam[index as usize] = v;
    }

    /// Unconditional VRAM write at the current bank, for the HDMA/GPDMA engine
    /// (which drives VRAM directly, not through the CPU's mode-gated port).
    pub fn write_vram_direct(&mut self, a: u16, v: u8) {
        let bank = self.vram_bank;
        self.write_vram(a, v, bank);
    }

    /// Word accessor on the raw OAM byte array (little-endian, 16-bit bus).
    fn oam_w(&self, r: usize, w: usize) -> u16 {
        let i = r * 8 + w * 2;
        (self.oam[i] as u16) | ((self.oam[i + 1] as u16) << 8)
    }

    fn oam_set_w(&mut self, r: usize, w: usize, v: u16) {
        let i = r * 8 + w * 2;
        self.oam[i] = v as u8;
        self.oam[i + 1] = (v >> 8) as u8;
    }

    fn oam_copy_row(&mut self, from: usize, to: usize) {
        let mut buf = [0u8; 8];
        buf.copy_from_slice(&self.oam[from * 8..from * 8 + 8]);
        self.oam[to * 8..to * 8 + 8].copy_from_slice(&buf);
    }

    /// The OAM row the PPU is currently scanning, if a DMG mode-2 corruption
    /// can happen at all. One row is scanned per M-cycle.
    fn oam_scan_row(&self) -> Option<usize> {
        if self.mode != GBMode::DMG || self.ppu_mode != PPUMode::OAMScan {
            return None;
        }
        Some((self.cycle_count / 4) as usize)
    }

    /// Common tail of read/write corruption: the last three words of `row` are
    /// copied from the preceding row; only the first-word formula differs.
    fn oam_glitch_row(&mut self, row: usize, new_w0: u16) {
        self.oam_set_w(row, 0, new_w0);
        for w in 1..4 {
            let v = self.oam_w(row - 1, w);
            self.oam_set_w(row, w, v);
        }
    }

    fn oam_write_corrupt(&mut self, row: usize) {
        if !(1..20).contains(&row) {
            return;
        }
        let a = self.oam_w(row, 0);
        let b = self.oam_w(row - 1, 0);
        let c = self.oam_w(row - 1, 2);
        self.oam_glitch_row(row, ((a ^ c) & (b ^ c)) ^ c);
    }

    fn oam_read_corrupt(&mut self, row: usize) {
        if !(1..20).contains(&row) {
            return;
        }
        let a = self.oam_w(row, 0);
        let b = self.oam_w(row - 1, 0);
        let c = self.oam_w(row - 1, 2);
        self.oam_glitch_row(row, b | (a & c));
    }

    fn oam_read_increase_corrupt(&mut self, row: usize) {
        if (4..=18).contains(&row) {
            let a = self.oam_w(row - 2, 0);
            let b = self.oam_w(row - 1, 0);
            let c = self.oam_w(row, 0);
            let d = self.oam_w(row - 1, 2);
            self.oam_set_w(row - 1, 0, (b & (a | c | d)) | (a & c & d));
            self.oam_copy_row(row - 1, row);
            self.oam_copy_row(row - 1, row - 2);
        }
        self.oam_read_corrupt(row);
    }

    pub fn oam_corrupt(&mut self, kind: OamGlitch) {
        let Some(row) = self.oam_scan_row() else {
            return;
        };
        match kind {
            OamGlitch::Increase | OamGlitch::Write => self.oam_write_corrupt(row),
            OamGlitch::Read => self.oam_read_corrupt(row),
            OamGlitch::ReadIncrease => self.oam_read_increase_corrupt(row),
        }
    }
}

impl Memory for PPU {
    fn read(&self, a: u16) -> u8 {
        match a {
            0x8000..=0x9FFF => {
                if self.ppu_mode != PPUMode::Draw {
                    self.read_vram(a, self.vram_bank)
                } else {
                    0xFF
                }
            }
            0xFE00..=0xFE9F => {
                if self.ppu_mode != PPUMode::Draw && self.ppu_mode != PPUMode::OAMScan {
                    self.oam[a as usize - 0xFE00]
                } else {
                    0xFF
                }
            }
            io::LCDC => self.lcdc.bits(),
            io::STAT => {
                // During the first-line mode-0 readback window the PPU is
                // internally in OAM scan but STAT reports mode 0.
                let mode = if self.first_line_after_on {
                    PPUMode::HBlank as u8
                } else {
                    self.ppu_mode as u8
                };
                self.lcds.bits() | mode | 0x80
            }
            io::SCY => self.scy,
            io::SCX => self.scx,
            io::LY => {
                if self.ly == 153 && self.cycle_count >= LY_153_ROLLOVER { 0 } else { self.ly }
            }
            io::LYC => self.lc,
            io::BGP => self.bgp,
            io::OBP0 => self.obp0,
            io::OBP1 => self.obp1,
            io::WY => self.wy,
            io::WX => self.wx,
            io::VBK => {
                if self.mode == GBMode::DMG {
                    0xFF
                } else {
                    0xFE | self.vram_bank as u8
                }
            }
            io::BGPI => {
                if self.mode == GBMode::DMG {
                    0xFF
                } else {
                    self.bcps.read() | 0x40
                }
            }
            io::BGPD => {
                if self.mode == GBMode::DMG || self.ppu_mode == PPUMode::Draw {
                    0xFF
                } else {
                    self.bcpd[self.bcps.address as usize]
                }
            }
            io::OBPI => {
                if self.mode == GBMode::DMG {
                    0xFF
                } else {
                    self.ocps.read() | 0x40
                }
            }
            io::OBPD => {
                if self.mode == GBMode::DMG || self.ppu_mode == PPUMode::Draw {
                    0xFF
                } else {
                    self.ocpd[self.ocps.address as usize]
                }
            }
            io::OPRI => 0xFE | self.opri as u8,
            _ => 0xFF,
        }
    }

    fn write(&mut self, a: u16, v: u8) {
        match a {
            0x8000..=0x9FFF => {
                if self.ppu_mode != PPUMode::Draw {
                    self.write_vram(a, v, self.vram_bank);
                }
            }
            0xFE00..=0xFE9F => {
                if self.ppu_mode != PPUMode::Draw && self.ppu_mode != PPUMode::OAMScan {
                    self.oam[a as usize - 0xFE00] = v
                }
            }
            io::LCDC => {
                let was_on = self.lcdc.contains(LCDC::LCD_ENABLE);
                self.lcdc = LCDC::from_bits(v).unwrap();
                if !self.lcdc.contains(LCDC::LCD_ENABLE) {
                    self.reset_ly();
                    self.ppu_mode = PPUMode::HBlank;

                    match self.mode {
                        GBMode::DMG => {
                            let color = self.ppu_config.palette.off;
                            self.framebuffer.fill(color.r(), color.g(), color.b());
                        }
                        GBMode::CGB => {
                            self.framebuffer.clear();
                        }
                    }
                } else if !was_on {
                    // Enabling the LCD restarts at the beginning of scanline 0.
                    // The seed sets the permanent phase between the PPU's dot
                    // counter and the CPU's M-cycle grid (a line is 456 ≡ 0 mod
                    // 4 dots, so it never re-aligns). The value encodes the
                    // hardware phase pinned by the mealybug m3 suite; the
                    // first-frame enable quirks (short line 0, mode-0 readback)
                    // are not yet modelled.
                    self.reset_ly();
                    self.ppu_mode = PPUMode::OAMScan;
                    self.cycle_count = 5;
                    // First-frame quirk: the freshly-enabled PPU scans OAM for
                    // line 0, but STAT reports mode 0 and the mode-2 source is
                    // suppressed until Mode 3 begins (mooneye stat_lyc_onoff).
                    self.first_line_after_on = true;
                    // The comparison clock restarts: re-evaluate LY(=0)=LYC now
                    // (mooneye stat_lyc_onoff), which may raise or drop the
                    // retained LYC_EQUALS bit and can fire a STAT interrupt.
                    self.check_lyc();
                }
            }
            io::STAT => {
                let sanitised = v & 0b1111_1000 | (self.lcds.bits() & 0b0000_0100);
                self.lcds = LCDS::from_bits_truncate(sanitised);
                self.check_lyc();
            }
            io::SCY => self.scy = v,
            io::SCX => self.scx = v,
            io::LY => {} // LY is read-only; writes are ignored by hardware
            io::LYC => {
                self.lc = v;
                self.check_lyc();
            }
            io::BGP => {
                // DMG: a BGP write landing during Mode 3 leaves `old | new`
                // visible for the single dot the write occupies before the new
                // value takes over — the 1-px slivers the mealybug
                // m3_bgp_change reference pins down. CGB does not glitch.
                if self.mode == GBMode::DMG && self.ppu_mode == PPUMode::Draw {
                    self.bgp_glitch = Some(self.bgp | v);
                }
                self.bgp = v;
            }
            io::OBP0 => self.obp0 = v,
            io::OBP1 => self.obp1 = v,
            io::WY => self.wy = v,
            io::WX => self.wx = v,
            io::KEY0 => {}
            io::VBK => self.vram_bank = (v & 0x01) as usize,
            io::BGPI => self.bcps.write(v),
            io::BGPD => {
                if self.ppu_mode != PPUMode::Draw {
                    self.bcpd[self.bcps.address as usize] = v;
                }

                if self.bcps.auto_increment {
                    self.bcps.address += 1;
                    self.bcps.address &= 0x3F;
                }
            }
            io::OBPI => self.ocps.write(v),
            io::OBPD => {
                if self.ppu_mode != PPUMode::Draw {
                    self.ocpd[self.ocps.address as usize] = v;
                }

                if self.ocps.auto_increment {
                    self.ocps.address += 1;
                    self.ocps.address &= 0x3F;
                }
            }
            io::OPRI => self.opri = v != 0,
            _ => {}
        }
    }
}
