use embedded_hal_async as hal_async;
use embedded_hal_1 as hal;

#[derive(Debug, Clone, Copy)]
pub struct Pos {
    pub x: i32,
    pub y: i32,
}

pub trait SharedPos {
    fn get_pos(&self) -> Option<Pos>;
    fn set_pos(&self, pos: Option<Pos>);
}

struct SortedArray<const N: usize> {
    data: [i32; N],
    len: usize,
}

impl<const N: usize> SortedArray<N> {
    fn new() -> Self {
        Self { data: [0; N], len: 0 }
    }
    fn clear(&mut self) {
        self.len = 0;
    }
    fn push(&mut self, elem: i32){
        let mut idx = 0;
        while idx < self.len && self.data[idx] < elem {
            idx += 1;
        }
        if idx < N {
            for j in (idx..self.len).rev() {
                self.data[j + 1] = self.data[j];
            }
            self.data[idx] = elem;
            if self.len < N {
                self.len += 1;
            }
        }
    }
    fn median(&self) -> Option<i32> {
        let idx_mid = self.len / 2;
        if self.len >= 3 {
            Some((self.data[idx_mid] + self.data[idx_mid - 1] + self.data[idx_mid + 1]) / 3)
        } else if self.len == 2 {
            Some((self.data[0] + self.data[1]) / 2)
        } else if self.len == 1 {
            Some(self.data[0])
        } else {
            None
        }
    }
}

struct Calibration {
    xraw_max: i32,
    xraw_min: i32,
    yraw_min: i32,
    yraw_max: i32,
    x_range: i32,
    y_range: i32,
}

impl Calibration {
    fn calc_pos(&self, xraw: i32, yraw: i32) -> (i32, i32) {
        let x = ((xraw - self.xraw_min) * self.x_range /
            (self.xraw_max - self.xraw_min)).clamp(0, self.x_range);
        let y = ((yraw - self.yraw_min) * self.y_range /
            (self.yraw_max - self.yraw_min)).clamp(0, self.y_range);
        (x, y)
    }
}

const CALIBRATION: Calibration = Calibration {
    xraw_min: 0x00c8,
    xraw_max: 0x0760,
    yraw_min: 0x00d0,
    yraw_max: 0x06d0,
    x_range: 320,
    y_range: 240,
};

pub struct Driver<SpiDevice> {
    spi_device: SpiDevice,
}

impl<SpiDevice: hal::spi::SpiDevice> Driver<SpiDevice> {
    pub fn new(spi_device: SpiDevice) -> Self {
        Self { spi_device }
    }
    pub async fn run(&mut self, shared_pos: &impl SharedPos, mut delay: impl hal_async::delay::DelayNs) {
        let mut idx_write = 0;
        let mut idx_read = 0;
        let mut pos_buf: [Pos; 8] = [Pos { x: 0, y: 0 }; 8];
        let mut x_sorted = SortedArray::<8>::new();
        let mut y_sorted = SortedArray::<8>::new();
        loop {
            if let Some((x, y)) = self.read_pos_raw() {
                pos_buf[idx_write] = Pos { x, y };
                idx_write = (idx_write + 1) % pos_buf.len();
                if idx_read == idx_write {
                    idx_read = (idx_read + 1) % pos_buf.len();
                }
                x_sorted.clear();
                y_sorted.clear();
                let mut idx = idx_read;
                while idx != idx_write {
                    x_sorted.push(pos_buf[idx].x);
                    y_sorted.push(pos_buf[idx].y);
                    idx = (idx + 1) % pos_buf.len();
                }
                let (x, y) = CALIBRATION.calc_pos(
                    x_sorted.median().unwrap_or(0), y_sorted.median().unwrap_or(0));
                shared_pos.set_pos(Some(Pos { x, y }));
            } else {
                idx_write = 0;
                idx_read = 0;
                shared_pos.set_pos(None);
            }
            delay.delay_ms(10).await;
        }
    }
    pub fn read_pos_raw(&mut self) -> Option<(i32, i32)> {
        let mut xbytes = [0u8; 2];
        let mut ybytes = [0u8; 2];
        let mut zbytes = [0u8; 1];
        self.spi_device.transaction(&mut [
            hal::spi::Operation::Write(&[Self::compose_cmd(0b001, 0b0, 0b1, 0b01)]), // Y
            hal::spi::Operation::Read(&mut xbytes),
            hal::spi::Operation::Write(&[Self::compose_cmd(0b101, 0b0, 0b1, 0b01)]), // X
            hal::spi::Operation::Read(&mut ybytes),
            hal::spi::Operation::Write(&[Self::compose_cmd(0b011, 0b1, 0b1, 0b01)]), // Z1
            hal::spi::Operation::Read(&mut zbytes),
        ]).ok()?;
        //defmt::info!("xbytes: {:02x}, ybytes: {:02x}, zbytes: {:02x}", xbytes, ybytes, zbytes);
        let x = (((xbytes[0] as i32) << 4) | (xbytes[1] as i32 >> 4)) as i32;
        let y = (((ybytes[0] as i32) << 4) | (ybytes[1] as i32 >> 4)) as i32;
        if zbytes[0] < 3 {
            None
        } else {
            //defmt::info!("xraw: {:04x}, yraw: {:04x}, zbytes: {:02x}", xraw, yraw, zbytes);
            Some((x, y))
        }
    }
    const fn compose_cmd(adc: u8, mode: u8, reference: u8, power_down_mode: u8) -> u8 {
        (0b1 << 7) | (adc << 4) | (mode << 3) | (reference << 2) | (power_down_mode << 0)
    }
}
