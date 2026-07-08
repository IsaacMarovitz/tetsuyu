use std::fs;
use std::path::Path;
use std::sync::OnceLock;
use tetsuyu::components::prelude::ppu::{SCREEN_H, SCREEN_W};
use tetsuyu::components::prelude::*;
use tetsuyu::config::{Color, Config};
use tetsuyu::framebuffer::{FramebufferReader, create_framebuffer_pair};
use tetsuyu::hw::motherboard::Motherboard;
use tetsuyu::mbc::header::{CGBFlag, Header};

/// Approx T-cycles per frame
pub const FC: u64 = 70_224;

#[macro_export]
macro_rules! test_suite {
    ($runner:expr, $($(#[ $attr:meta ])* $id:ident => ($($arg:expr),*)),* $(,)?) => {
        $(
            $(#[$attr])*
            #[test]
            fn $id() {
                $runner($($arg),*);
            }
        )*
    };
}

fn test_config() -> Option<&'static Config> {
    static CONFIG: OnceLock<Option<Config>> = OnceLock::new();
    CONFIG
        .get_or_init(|| {
            let s = fs::read_to_string("./config.toml").ok()?;
            let mut config: Config = toml::from_str(&s).ok()?;
            config.headless = true;
            config.print_serial = false;

            // Mealybug palette
            config.ppu_config.palette.dark = Color::new(0x000000);
            config.ppu_config.palette.dark_gray = Color::new(0x555555);
            config.ppu_config.palette.light_gray = Color::new(0xAAAAAA);
            config.ppu_config.palette.light = Color::new(0xFFFFFF);
            config.ppu_config.palette.off = Color::new(0xFF0000);

            Some(config)
        })
        .as_ref()
}

pub fn setup_harness(rom_path: &str, mode: GBMode) -> Option<Harness> {
    let base_config = test_config().or_else(|| {
        static WARN: std::sync::Once = std::sync::Once::new();
        WARN.call_once(|| eprintln!("skipping: no ./config.toml with boot-rom paths"));
        None
    })?;

    let mut config = base_config.clone();
    config.mode = mode;
    Harness::new(rom_path, config).ok()
}

/// Condition that ends a [`Harness::run_until`] run.
#[derive(Debug, Clone, Copy)]
pub enum StopCondition {
    /// The CPU executed `LD B,B` (0x40), the test-ROM magic breakpoint.
    MagicBreak,
    /// Stop when Blargg's status byte at $A000 changes from $80 (running) to a result code.
    BlarggStatus,
    /// At least this many frames (VBlanks) have been produced.
    Frames(u64),
    /// At least this many T-cycles have elapsed.
    Cycles(u64),
    /// Stop only when the absolute end of the trimmed serial buffer matches an expected string.
    SerialEndsWithAny(&'static [&'static str]),
}

/// Why a run ended.
#[derive(Debug, PartialEq, Eq)]
pub enum RunOutcome {
    /// The stop condition was met.
    Met,
    /// The cycle budget elapsed before the stop condition was met.
    TimedOut,
}

pub struct Harness {
    mb: Motherboard,
    fb: FramebufferReader,
    cycles: u64,
    /// Latch to prevent BlarggStatus from exiting early during ROM initialization.
    blargg_started: bool,
}

impl Harness {
    /// Load `rom_path` and build a headless machine using `config` (including
    /// its boot-rom paths and mode).
    pub fn new(rom_path: &str, config: Config) -> Result<Self, String> {
        let rom = fs::read(rom_path).map_err(|e| format!("open ROM \"{rom_path}\": {e}"))?;
        let header = Header::new(rom.clone());
        let rom_is_cgb = matches!(
            header.cgb_flag,
            CGBFlag::CGBOnly | CGBFlag::BackwardsCompatible
        );

        let boot_path = match config.mode {
            GBMode::DMG => config.dmg_boot_rom.clone(),
            GBMode::CGB => config.cgb_boot_rom.clone(),
        };
        let boot_vec =
            fs::read(&boot_path).map_err(|e| format!("open boot ROM \"{boot_path}\": {e}"))?;
        let mut boot_rom = [0u8; 0x900];
        let end = boot_vec.len().min(boot_rom.len());
        boot_rom[..end].copy_from_slice(&boot_vec[..end]);

        let (writer, fb) = create_framebuffer_pair();
        let mb = Motherboard::new(rom, header, config, boot_rom, writer, rom_is_cgb);

        Ok(Self {
            mb,
            fb,
            cycles: 0,
            blargg_started: false,
        })
    }

    /// Step one instruction at a time until `stop` is met or `max_cycles`
    /// T-cycles have elapsed, whichever comes first.
    pub fn run_until(&mut self, stop: StopCondition, max_cycles: u64) -> RunOutcome {
        loop {
            self.cycles += self.mb.step() as u64;
            let frames = self.fb.poll();

            if self.check_stop_condition(stop, frames) {
                return RunOutcome::Met;
            }
            if self.cycles >= max_cycles {
                return RunOutcome::TimedOut;
            }
        }
    }

    /// Step one instruction at a time indefinitely until `stop` is met.
    /// Has no cycle constraints or execution timeout limits.
    pub fn run_until_unbounded(&mut self, stop: StopCondition) {
        loop {
            self.cycles += self.mb.step() as u64;
            let frames = self.fb.poll();

            if self.check_stop_condition(stop, frames) {
                break;
            }
        }
    }

    /// Centralized stop-condition evaluation logic shared by execution runners.
    fn check_stop_condition(&mut self, stop: StopCondition, frames: u64) -> bool {
        match stop {
            StopCondition::MagicBreak => self.mb.magic_break(),
            StopCondition::Frames(n) => frames >= n,
            StopCondition::Cycles(n) => self.cycles >= n,
            StopCondition::BlarggStatus => {
                let status = self.mb.peek(0xA000);
                let has_sig = self.mb.peek(0xA001) == 0xDE
                    && self.mb.peek(0xA002) == 0xB0
                    && self.mb.peek(0xA003) == 0x61;

                // Latch true only when the test suite officially flags it has started running
                if has_sig && status == 0x80 {
                    self.blargg_started = true;
                }

                // Only exit when it has safely initialized and status changes away from 0x80
                has_sig && self.blargg_started && status != 0x80
            }
            StopCondition::SerialEndsWithAny(substrings) => {
                let out = self.serial();

                // Trim trailing whitespaces, carriage returns, and newlines from the end edge
                let mut len = out.len();
                while len > 0
                    && (out[len - 1] == b'\n'
                        || out[len - 1] == b'\r'
                        || out[len - 1] == b' '
                        || out[len - 1] == b'\t')
                {
                    len -= 1;
                }
                let trimmed = &out[..len];

                // Verify if the strict end of the current buffer matches our termination goals
                substrings
                    .iter()
                    .any(|&sub| trimmed.ends_with(sub.as_bytes()))
            }
        }
    }

    /// Read one byte of CPU-addressable memory (no side effects).
    pub fn peek(&self, addr: u16) -> u8 {
        self.mb.peek(addr)
    }

    /// Bytes transmitted over the serial port so far.
    pub fn serial(&self) -> &[u8] {
        self.mb.serial_output()
    }

    /// CPU registers.
    pub fn cpu_regs(&self) -> Registers {
        self.mb.cpu_regs()
    }

    /// True once the CPU has hit the `LD B,B` magic breakpoint.
    pub fn magic_break(&self) -> bool {
        self.mb.magic_break()
    }

    /// The most recent complete frame as RGBA (`4 * 160 * 144` bytes).
    pub fn framebuffer(&mut self) -> &[u8] {
        self.fb.get_latest_frame()
    }
}

/// A decoded reference image, expanded to 8-bit RGBA.
pub struct RefImage {
    pub width: usize,
    pub height: usize,
    pub rgba: Vec<u8>,
}

impl RefImage {
    /// Decode the PNG at `path` into 8-bit RGBA. Supports the 8-bit grayscale,
    /// RGB, RGBA, and palette-indexed encodings the mealybug / acid2 references
    /// ship as.
    pub fn load_png(path: impl AsRef<Path>) -> Result<Self, String> {
        let path = path.as_ref();
        let file = fs::File::open(path)
            .map_err(|e| format!("open reference \"{}\": {e}", path.display()))?;
        // The mealybug references are 1-/2-bit indexed or grayscale PNGs; expand
        // paletted + low-bit-depth pixels to straight 8-bit channels (and strip
        // any 16-bit down to 8) so the byte handling below is uniform.
        let mut decoder = png::Decoder::new(std::io::BufReader::new(file));
        decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
        let mut reader = decoder
            .read_info()
            .map_err(|e| format!("decode \"{}\": {e}", path.display()))?;

        let buf_size = reader
            .output_buffer_size()
            .ok_or_else(|| format!("\"{}\": image too large", path.display()))?;
        let mut raw = vec![0u8; buf_size];
        let info = reader
            .next_frame(&mut raw)
            .map_err(|e| format!("read \"{}\": {e}", path.display()))?;
        raw.truncate(info.buffer_size());

        if info.bit_depth != png::BitDepth::Eight {
            return Err(format!(
                "\"{}\": unsupported bit depth {:?} (expected 8-bit)",
                path.display(),
                info.bit_depth
            ));
        }

        let (w, h) = (info.width as usize, info.height as usize);
        let px = w * h;
        let mut rgba = vec![0u8; px * 4];

        match info.color_type {
            png::ColorType::Grayscale => {
                for (i, &g) in raw.iter().take(px).enumerate() {
                    rgba[i * 4..i * 4 + 4].copy_from_slice(&[g, g, g, 0xFF]);
                }
            }
            png::ColorType::GrayscaleAlpha => {
                for i in 0..px {
                    let g = raw[i * 2];
                    rgba[i * 4..i * 4 + 4].copy_from_slice(&[g, g, g, raw[i * 2 + 1]]);
                }
            }
            png::ColorType::Rgb => {
                for i in 0..px {
                    rgba[i * 4..i * 4 + 3].copy_from_slice(&raw[i * 3..i * 3 + 3]);
                    rgba[i * 4 + 3] = 0xFF;
                }
            }
            png::ColorType::Rgba => rgba.copy_from_slice(&raw[..px * 4]),
            png::ColorType::Indexed => {
                let palette =
                    reader.info().palette.as_ref().ok_or_else(|| {
                        format!("\"{}\": indexed PNG has no palette", path.display())
                    })?;
                for i in 0..px {
                    let idx = raw[i] as usize;
                    rgba[i * 4] = palette.get(idx * 3).copied().unwrap_or(0);
                    rgba[i * 4 + 1] = palette.get(idx * 3 + 1).copied().unwrap_or(0);
                    rgba[i * 4 + 2] = palette.get(idx * 3 + 2).copied().unwrap_or(0);
                    rgba[i * 4 + 3] = 0xFF;
                }
            }
        }

        Ok(Self {
            width: w,
            height: h,
            rgba,
        })
    }
}

/// Result of comparing a rendered frame to a reference image.
#[derive(Debug, Clone)]
pub struct DiffReport {
    pub total: usize,
    pub matched: usize,
    /// First (x, y) that differs, scanning row-major; `None` when exact.
    pub first_diff: Option<(usize, usize)>,
}

impl DiffReport {
    pub fn match_pct(&self) -> f64 {
        if self.total == 0 {
            0.0
        } else {
            self.matched as f64 * 100.0 / self.total as f64
        }
    }

    pub fn format_diff(&self) -> String {
        self.first_diff
            .map(|(x, y)| format!("({x},{y})"))
            .unwrap_or_else(|| "exact".into())
    }
}

pub fn compare_frame(frame: &[u8], reference: &RefImage) -> Result<DiffReport, String> {
    if reference.width != SCREEN_W || reference.height != SCREEN_H {
        return Err(format!(
            "reference is {}×{}, expected {SCREEN_W}×{SCREEN_H}",
            reference.width, reference.height
        ));
    }
    if frame.len() < SCREEN_W * SCREEN_H * 4 {
        return Err(format!("frame is {} bytes, too small", frame.len()));
    }

    let mut matched = 0;
    let mut first_diff = None;

    for (i, (f_px, r_px)) in frame
        .chunks_exact(4)
        .zip(reference.rgba.chunks_exact(4))
        .enumerate()
    {
        if f_px[..3] == r_px[..3] {
            matched += 1;
        } else if first_diff.is_none() {
            first_diff = Some((i % SCREEN_W, i / SCREEN_W));
        }
    }

    Ok(DiffReport {
        total: SCREEN_W * SCREEN_H,
        matched,
        first_diff,
    })
}

pub fn run_and_compare(
    rom_path: &str,
    expected_png: &str,
    frames: u64,
) -> Option<Result<DiffReport, String>> {
    let mut h = setup_harness(rom_path, GBMode::DMG)?;
    let reference = match RefImage::load_png(expected_png) {
        Ok(img) => img,
        Err(e) => return Some(Err(e)),
    };

    if h.run_until(StopCondition::Frames(frames), (frames + 20) * FC) == RunOutcome::TimedOut {
        return Some(Err(format!(
            "{rom_path}: did not reach {frames} frames in budget"
        )));
    }
    Some(compare_frame(h.framebuffer(), &reference))
}
