use crate::components::ppu::ppu::FRAMEBUFFER_SIZE;
use std::sync::mpsc::{Receiver, Sender, channel};

pub struct Frame {
    pub data: Box<[u8; FRAMEBUFFER_SIZE]>,
}

impl Frame {
    pub fn new() -> Self {
        Self {
            data: Box::new([0xFF; FRAMEBUFFER_SIZE]),
        }
    }
}

pub struct FramebufferWriter {
    back_buffer: Box<[u8; FRAMEBUFFER_SIZE]>,
    frame_sender: Sender<Frame>,
}

impl FramebufferWriter {
    pub fn new(frame_sender: Sender<Frame>) -> Self {
        Self {
            back_buffer: Box::new([0xFF; FRAMEBUFFER_SIZE]),
            frame_sender,
        }
    }

    #[inline]
    pub fn set_pixel(&mut self, r: u8, g: u8, b: u8, x: usize, y: usize) {
        const BYTES_PER_PIXEL: usize = 4;
        const BYTES_PER_ROW: usize = BYTES_PER_PIXEL * 160;

        let vertical_offset = y * BYTES_PER_ROW;
        let horizontal_offset = x * BYTES_PER_PIXEL;
        let total_offset = vertical_offset + horizontal_offset;

        self.back_buffer[total_offset] = r;
        self.back_buffer[total_offset + 1] = g;
        self.back_buffer[total_offset + 2] = b;
        self.back_buffer[total_offset + 3] = 0xFF;
    }

    pub fn submit_frame(&mut self) {
        let mut new_back = Box::new([0xFF; FRAMEBUFFER_SIZE]);
        std::mem::swap(&mut self.back_buffer, &mut new_back);

        let frame = Frame { data: new_back };

        let _ = self.frame_sender.send(frame);
    }

    pub fn clear(&mut self) {
        self.back_buffer.fill(0xFF);
    }

    pub fn fill(&mut self, r: u8, g: u8, b: u8) {
        for y in 0..144 {
            for x in 0..160 {
                self.set_pixel(r, g, b, x, y);
            }
        }
    }
}

pub struct FramebufferReader {
    frame_receiver: Receiver<Frame>,
    current_frame: Frame,
}

impl FramebufferReader {
    pub fn new(frame_receiver: Receiver<Frame>) -> Self {
        Self {
            frame_receiver,
            current_frame: Frame::new(),
        }
    }

    pub fn get_latest_frame(&mut self) -> &[u8] {
        while let Ok(frame) = self.frame_receiver.try_recv() {
            self.current_frame = frame;
        }

        &self.current_frame.data[..]
    }
}

pub fn create_framebuffer_pair() -> (FramebufferWriter, FramebufferReader) {
    let (sender, receiver) = channel();
    (
        FramebufferWriter::new(sender),
        FramebufferReader::new(receiver),
    )
}
