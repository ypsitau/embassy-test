use embedded_hal_async as hal_async;
use embedded_hal_1 as hal;
use embedded_graphics as eg;
use embedded_graphics::prelude::*;
use embedded_graphics::draw_target::DrawTarget;

#[derive(Debug, Clone, Copy)]
pub struct Pos {
    pub x: i32,
    pub y: i32,
}

pub trait SharedPos {
    fn get_pos(&self) -> Option<Pos>;
    fn set_pos(&self, pos: Option<Pos>);
}

#[derive(Debug, Clone, Copy, defmt::Format)]
pub struct Calibration {
    xraw_right: i32,
    xraw_left: i32,
    yraw_top: i32,
    yraw_bottom: i32,
    x_range: i32,
    y_range: i32,
}

impl Calibration {
    fn calc_pos(&self, xraw: u16, yraw: u16) -> (i32, i32) {
        let x = (((xraw as i32) - self.xraw_left) * self.x_range /
            (self.xraw_right - self.xraw_left)).clamp(0, self.x_range);
        let y = (((yraw as i32) - self.yraw_top) * self.y_range /
            (self.yraw_bottom - self.yraw_top)).clamp(0, self.y_range);
        (x, y)
    }
}

impl Default for Calibration {
    fn default() -> Self {
        Self {
            xraw_left: 0x00c8,
            xraw_right: 0x0760,
            yraw_top: 0x00d0,
            yraw_bottom: 0x06d0,
            x_range: 320,
            y_range: 240,
        }
    }
}

pub struct Driver<SpiDevice> {
    spi_device: SpiDevice,
    pub calibration: Calibration,
    swap_xy: bool,
}

impl<SpiDevice: hal::spi::SpiDevice> Driver<SpiDevice> {
    pub fn new(spi_device: SpiDevice, calibration: Calibration, swap_xy: bool) -> Self {
        Self { spi_device, calibration, swap_xy }
    }
    pub async fn calibrate(&mut self, display: &mut impl DrawTarget<Color = eg::pixelcolor::Rgb565>, mut delay: impl hal_async::delay::DelayNs) {
        const DISTANCE_FROM_EDGE: i32 = 20;
        let (x_left, y_top) = (DISTANCE_FROM_EDGE, DISTANCE_FROM_EDGE);
        let (x_right, y_bottom) = (
            self.calibration.x_range - DISTANCE_FROM_EDGE,
            self.calibration.y_range - DISTANCE_FROM_EDGE);
        display.clear(eg::pixelcolor::Rgb565::BLACK).ok();
        Self::draw_cross(display, x_left, y_top);
        let (xraw1, yraw1) = self.read_pos_raw_for_calibration(&mut delay).await;
        display.clear(eg::pixelcolor::Rgb565::BLACK).ok();
        Self::draw_cross(display, x_right, y_bottom);
        let (xraw2, yraw2) = self.read_pos_raw_for_calibration(&mut delay).await;
        display.clear(eg::pixelcolor::Rgb565::BLACK).ok();
        while let Some(_) = self.read_pos_raw() {
            delay.delay_ms(100).await;
        }
        self.calibration.xraw_left = xraw1 + (xraw2 - xraw1) *
                            (-x_left) / (x_right - x_left);
        self.calibration.xraw_right = xraw1 + (xraw2 - xraw1) *
                            (self.calibration.x_range - x_left) / (x_right - x_left);
        self.calibration.yraw_top = yraw1 + (yraw2 - yraw1) *
                            (-y_top) / (y_bottom - y_top);
        self.calibration.yraw_bottom = yraw1 + (yraw2 - yraw1) *
                            (self.calibration.y_range - y_top) / (y_bottom - y_top);
    }
    fn draw_cross(display: &mut impl DrawTarget<Color = eg::pixelcolor::Rgb565>, x: i32, y: i32) {
        const CROSS_SIZE: i32 = 10;
        const CROSS_THICKNESS: u32 = 4;
        eg::primitives::Line::new(Point::new(x - CROSS_SIZE, y), Point::new(x + CROSS_SIZE, y))
            .into_styled(eg::primitives::PrimitiveStyle::with_stroke(
                eg::pixelcolor::Rgb565::WHITE, CROSS_THICKNESS))
            .draw(display).ok();
        eg::primitives::Line::new(Point::new(x, y - CROSS_SIZE), Point::new(x, y + CROSS_SIZE))
            .into_styled(eg::primitives::PrimitiveStyle::with_stroke(
                eg::pixelcolor::Rgb565::WHITE, CROSS_THICKNESS))
            .draw(display).ok();
    }
    pub async fn run(&mut self, shared_pos: &impl SharedPos, mut delay: impl hal_async::delay::DelayNs, sampling_delay: u32) {
        const NUM_SAMPLES: usize = 6;
        let mut xraw_hist = heapless::HistoryBuf::<u16, NUM_SAMPLES>::new();
        let mut yraw_hist = heapless::HistoryBuf::<u16, NUM_SAMPLES>::new();
        let mut xraw_sorted = heapless::Vec::<u16, NUM_SAMPLES>::new();
        let mut yraw_sorted = heapless::Vec::<u16, NUM_SAMPLES>::new();
        loop {
            if let Some((x, y)) = self.read_pos_raw() {
                xraw_hist.write(x);
                yraw_hist.write(y);
                if xraw_hist.is_full() {
                    xraw_sorted.clear();
                    yraw_sorted.clear();
                    xraw_hist.iter().for_each(|&x| xraw_sorted.push(x).unwrap());
                    yraw_hist.iter().for_each(|&y| yraw_sorted.push(y).unwrap());
                    xraw_sorted.sort_unstable();
                    yraw_sorted.sort_unstable();
                    let idx = xraw_sorted.len() / 2;
                    let xraw_avg = (xraw_sorted[idx] + xraw_sorted[idx - 1] + xraw_sorted[idx + 1]) / 3;
                    let yraw_avg = (yraw_sorted[idx] + yraw_sorted[idx - 1] + yraw_sorted[idx + 1]) / 3;
                    let (x, y) = self.calibration.calc_pos(xraw_avg, yraw_avg);
                    shared_pos.set_pos(Some(Pos { x, y }));
                } else {
                    shared_pos.set_pos(None);
                }
            } else {
                xraw_hist.clear();
                yraw_hist.clear();
                shared_pos.set_pos(None);
            }
            delay.delay_ms(sampling_delay).await;
        }
    }
    pub fn read_pos_raw(&mut self) -> Option<(u16, u16)> {
        let mut xbytes = [0u8; 2];
        let mut ybytes = [0u8; 2];
        let mut zbytes = [0u8; 1];
        self.spi_device.transaction(&mut [
            hal::spi::Operation::Write(&[Self::compose_cmd(0b101, 0b0, 0b1, 0b01)]), // X
            hal::spi::Operation::Read(&mut xbytes),
            hal::spi::Operation::Write(&[Self::compose_cmd(0b001, 0b0, 0b1, 0b01)]), // Y
            hal::spi::Operation::Read(&mut ybytes),
            hal::spi::Operation::Write(&[Self::compose_cmd(0b011, 0b1, 0b1, 0b01)]), // Z1
            hal::spi::Operation::Read(&mut zbytes),
        ]).ok()?;
        //defmt::info!("xbytes: {:02x}, ybytes: {:02x}, zbytes: {:02x}", xbytes, ybytes, zbytes);
        let xraw = (((xbytes[0] as u16) << 4) | (xbytes[1] as u16 >> 4)) as u16;
        let yraw = (((ybytes[0] as u16) << 4) | (ybytes[1] as u16 >> 4)) as u16;
        if zbytes[0] < 3 {
            None
        } else if self.swap_xy {
            Some((yraw, xraw))
        } else {
            Some((xraw, yraw))
        }
    }
    async fn read_pos_raw_for_calibration(&mut self, delay: &mut impl hal_async::delay::DelayNs) -> (i32, i32) {
        const NUM_SAMPLES: usize = 10;
        let mut xraw_hist = heapless::HistoryBuf::<u16, NUM_SAMPLES>::new();
        let mut yraw_hist = heapless::HistoryBuf::<u16, NUM_SAMPLES>::new();
        let mut xraw_sorted = heapless::Vec::<u16, NUM_SAMPLES>::new();
        let mut yraw_sorted = heapless::Vec::<u16, NUM_SAMPLES>::new();
        loop {
            if let Some((x, y)) = self.read_pos_raw() {
                xraw_hist.write(x);
                yraw_hist.write(y);
                if xraw_hist.is_full() { break; }
            } else {
                xraw_hist.clear();
                yraw_hist.clear();
            }
            delay.delay_ms(100).await;
        }
        xraw_sorted.clear();
        yraw_sorted.clear();
        xraw_hist.iter().for_each(|&x| xraw_sorted.push(x).unwrap());
        yraw_hist.iter().for_each(|&y| yraw_sorted.push(y).unwrap());
        xraw_sorted.sort_unstable();
        yraw_sorted.sort_unstable();
        let idx = xraw_sorted.len() / 2;
        let xraw_avg = (xraw_sorted[idx] + xraw_sorted[idx - 1] + xraw_sorted[idx + 1]) / 3;
        let yraw_avg = (yraw_sorted[idx] + yraw_sorted[idx - 1] + yraw_sorted[idx + 1]) / 3;
        (xraw_avg as i32, yraw_avg as i32)
    }
    const fn compose_cmd(adc: u8, mode: u8, reference: u8, power_down_mode: u8) -> u8 {
        (0b1 << 7) | (adc << 4) | (mode << 3) | (reference << 2) | (power_down_mode << 0)
    }
}
