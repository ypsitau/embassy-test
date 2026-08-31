use embedded_hal_async as hal_async;
use embedded_hal_1 as hal;
use embedded_graphics as eg;
use embedded_graphics::prelude::*;
use embedded_graphics::draw_target::DrawTarget;

#[derive(Debug, Clone, Copy, defmt::Format)]
pub struct Calibration {
    xraw_right: i32,
    xraw_left: i32,
    yraw_top: i32,
    yraw_bottom: i32,
}

impl Default for Calibration {
    fn default() -> Self {
        Self {
            xraw_left: 0x0000,
            xraw_right: 0x07ff,
            yraw_top: 0x0000,
            yraw_bottom: 0x07ff,
        }
    }
}

pub struct Driver<SpiDevice> {
    spi_device: SpiDevice,
    x_range: i32,
    y_range: i32,
    pub calibration: Calibration,
    rotate90: bool,
}

pub struct Builder<SpiDevice> {
    driver: Driver<SpiDevice>,
}

impl<SpiDevice: hal::spi::SpiDevice> Builder<SpiDevice> {
    pub fn new(spi_device: SpiDevice, x_range: i32, y_range: i32) -> Self {
        Self {
            driver: Driver { spi_device, x_range, y_range, calibration: Calibration::default(), rotate90: false },
        }
    }
    pub fn calibration(mut self, calibration: Calibration) -> Self {
        self.driver.calibration = calibration;
        self
    }
    pub fn rotate90(mut self, rotate90: bool) -> Self {
        if rotate90 {
            core::mem::swap(&mut self.driver.x_range, &mut self.driver.y_range);
        }
        self.driver.rotate90 = rotate90;
        self
    }
    pub fn build(self) -> Driver<SpiDevice> {
        self.driver
    }
}

impl<SpiDevice: hal::spi::SpiDevice> Driver<SpiDevice> {
    pub async fn run(&mut self, mut delay: impl hal_async::delay::DelayNs, sampling_delay: u32, mut on_pos_updated: impl FnMut(Option<(i32, i32)>)) {
        const NUM_SAMPLES: usize = 6;
        let mut xraw_hist = heapless::HistoryBuf::<u16, NUM_SAMPLES>::new();
        let mut yraw_hist = heapless::HistoryBuf::<u16, NUM_SAMPLES>::new();
        let mut xraw_sorted = [0u16; NUM_SAMPLES];
        let mut yraw_sorted = [0u16; NUM_SAMPLES];
        loop {
            if let Some((xraw, yraw)) = self.read_pos_raw() {
                xraw_hist.write(xraw);
                yraw_hist.write(yraw);
                if xraw_hist.is_full() {
                    xraw_sorted.copy_from_slice(xraw_hist.as_slice());
                    yraw_sorted.copy_from_slice(yraw_hist.as_slice());
                    xraw_sorted.sort_unstable();
                    yraw_sorted.sort_unstable();
                    let idx = xraw_sorted.len() / 2;
                    let xraw_avg = (xraw_sorted[idx] + xraw_sorted[idx - 1] + xraw_sorted[idx + 1]) / 3;
                    let yraw_avg = (yraw_sorted[idx] + yraw_sorted[idx - 1] + yraw_sorted[idx + 1]) / 3;
                    let (x, y) = self.calc_pos(xraw_avg, yraw_avg);
                    on_pos_updated(Some((x, y)));
                }
            } else if !xraw_hist.is_empty() {
                xraw_hist.clear();
                yraw_hist.clear();
                on_pos_updated(None);
            }
            delay.delay_ms(sampling_delay).await;
        }
    }
    fn calc_pos(&self, xraw: u16, yraw: u16) -> (i32, i32) {
        let &c = &self.calibration;
        let x = (((xraw as i32) - c.xraw_left) * self.x_range / (c.xraw_right - c.xraw_left)).clamp(0, self.x_range);
        let y = (((yraw as i32) - c.yraw_top) * self.y_range / (c.yraw_bottom - c.yraw_top)).clamp(0, self.y_range);
        (x, y)
    }
    pub fn read_pos_raw(&mut self) -> Option<(u16, u16)> {
        let (xraw, yraw) = self.read_pos_adc()?;
        if self.rotate90 {
            Some((yraw, xraw))
        } else {
            Some((xraw, yraw))
        }
    }
    pub fn read_pos_adc(&mut self) -> Option<(u16, u16)> {
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
        } else {
            Some((xraw, yraw))
        }
    }
    const fn compose_cmd(adc: u8, mode: u8, reference: u8, power_down_mode: u8) -> u8 {
        (0b1 << 7) | (adc << 4) | (mode << 3) | (reference << 2) | (power_down_mode << 0)
    }
}

pub async fn calibrate<Color: eg::pixelcolor::PixelColor>(
        touch: &mut Driver<impl hal::spi::SpiDevice>, display: &mut impl DrawTarget<Color = Color>,
        mut delay: impl hal_async::delay::DelayNs,
        color: Color, color_bg: Color) -> Option<Calibration> {
    const DISTANCE_FROM_EDGE: i32 = 20;
    let pts = [
        (DISTANCE_FROM_EDGE, DISTANCE_FROM_EDGE),
        (touch.x_range - DISTANCE_FROM_EDGE, touch.y_range - DISTANCE_FROM_EDGE),
    ];
    let mut ptraws: heapless::Vec::<(i32, i32), 2> = heapless::Vec::new();
    for (x, y) in &pts {
        display.clear(color_bg).ok();
        draw_cross(display, *x, *y, color);
        ptraws.push(read_pos_raw_for_calibration(touch, &mut delay).await).ok();
    }
    display.clear(color_bg).ok();
    while let Some(_) = touch.read_pos_adc() {
        delay.delay_ms(100).await;
    }
    let (xraw1, yraw1) = ptraws[0];
    let (xraw2, yraw2) = ptraws[1];
    let (x_left, y_top) = pts[0];
    let (x_right, y_bottom) = pts[1];
    let xraw_left = xraw1 + (xraw2 - xraw1) * (-x_left) / (x_right - x_left);
    let xraw_right = xraw1 + (xraw2 - xraw1) * (touch.x_range - x_left) / (x_right - x_left);
    let yraw_top = yraw1 + (yraw2 - yraw1) * (-y_top) / (y_bottom - y_top);
    let yraw_bottom = yraw1 + (yraw2 - yraw1) * (touch.y_range - y_top) / (y_bottom - y_top);
    if xraw_left == xraw_right || yraw_top == yraw_bottom {
        defmt::warn!("Calibration failed: xraw_left == xraw_right or yraw_top == yraw_bottom");
        return None;
    }
    Some(Calibration { xraw_left, xraw_right, yraw_top, yraw_bottom, })
}

fn draw_cross<Color: eg::pixelcolor::PixelColor>(display: &mut impl DrawTarget<Color = Color>, x: i32, y: i32, color: Color) {
    const CROSS_SIZE: i32 = 10;
    const CROSS_THICKNESS: u32 = 4;
    eg::primitives::Line::new(Point::new(x - CROSS_SIZE, y), Point::new(x + CROSS_SIZE, y))
        .into_styled(eg::primitives::PrimitiveStyle::with_stroke(color, CROSS_THICKNESS))
        .draw(display).ok();
    eg::primitives::Line::new(Point::new(x, y - CROSS_SIZE), Point::new(x, y + CROSS_SIZE))
        .into_styled(eg::primitives::PrimitiveStyle::with_stroke(color, CROSS_THICKNESS))
        .draw(display).ok();
}

async fn read_pos_raw_for_calibration(touch: &mut Driver<impl hal::spi::SpiDevice>,
        delay: &mut impl hal_async::delay::DelayNs) -> (i32, i32) {
    const NUM_SAMPLES: usize = 10;
    let mut xraws = heapless::Vec::<u16, NUM_SAMPLES>::new();
    let mut yraws = heapless::Vec::<u16, NUM_SAMPLES>::new();
    loop {
        if let Some((xraw, yraw)) = touch.read_pos_raw() {
            xraws.push(xraw).unwrap();
            yraws.push(yraw).unwrap();
            if xraws.is_full() { break; }
        } else {
            xraws.clear();
            yraws.clear();
        }
        delay.delay_ms(100).await;
    }
    xraws.sort_unstable();
    yraws.sort_unstable();
    let idx = xraws.len() / 2;
    let xraw_avg = (xraws[idx] + xraws[idx - 1] + xraws[idx + 1]) / 3;
    let yraw_avg = (yraws[idx] + yraws[idx - 1] + yraws[idx + 1]) / 3;
    (xraw_avg as i32, yraw_avg as i32)
}
