#![no_std]
#![no_main]


//use defmt::*;
use embassy_executor::Spawner;
use embassy_rp as rp;
use embassy_rp::pwm::SetDutyCycle as _;
use fixed::traits::ToFixed as _;
use {defmt_rtt as _, panic_probe as _};

pub struct PwmSlice<'d> {
    pub pwm: rp::pwm::Pwm<'d>,
    pub config: rp::pwm::Config,
}

impl<'d> PwmSlice<'d> {
    pub fn new<T: rp::pwm::Slice>(slice: rp::Peri<'d, T>, a: rp::Peri<'d, impl rp::pwm::ChannelAPin<T>>, b: rp::Peri<'d, impl rp::pwm::ChannelBPin<T>>, config: rp::pwm::Config) -> Self {
        let pwm = rp::pwm::Pwm::new_output_ab(slice, a, b, config.clone());
        Self { pwm, config }
    }
    pub fn set_duty_cycle_a(&mut self, duty: u16) {
        self.config.compare_a = duty;
        self.pwm.set_config(&self.config);
    }
    pub fn set_duty_cycle_b(&mut self, duty: u16) {
        self.config.compare_b = duty;
        self.pwm.set_config(&self.config);
    }
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = rp::init(Default::default());
    let mut config = rp::pwm::Config::default();
    let (divider, top) = pwm_calc_divider_and_top(1000);
    config.divider = divider.to_fixed();
    config.top = top;
    let mut pwm_slice0 = PwmSlice::new(p.PWM_SLICE0, p.PIN_0, p.PIN_1, config.clone());
    pwm_slice0.set_duty_cycle_a(top / 4);
    pwm_slice0.set_duty_cycle_b(top / 2);
    /*
    let mut pwm_drivers = [
        pwm::Pwm::new_output_ab(p.PWM_SLICE1, p.PIN_2, p.PIN_3, config.clone()),
        pwm::Pwm::new_output_ab(p.PWM_SLICE2, p.PIN_4, p.PIN_5, config.clone()),
        pwm::Pwm::new_output_ab(p.PWM_SLICE3, p.PIN_6, p.PIN_7, config.clone()),
        pwm::Pwm::new_output_ab(p.PWM_SLICE4, p.PIN_8, p.PIN_9, config.clone()),
        pwm::Pwm::new_output_ab(p.PWM_SLICE5, p.PIN_10, p.PIN_11, config.clone()),
        pwm::Pwm::new_output_ab(p.PWM_SLICE6, p.PIN_12, p.PIN_13, config.clone()),
        pwm::Pwm::new_output_ab(p.PWM_SLICE7, p.PIN_14, p.PIN_15, config.clone()),
    ];
    */
    //for (i, pwm_driver) in pwm_drivers.iter_mut().enumerate() {
    //    let duty_cycle = (top as u32 * (i as u32 + 1) / pwm_drivers.len() as u32) as u16;
    //    pwm_driver.set_duty_cycle(duty_cycle).unwrap();
    //}
    //pwm_driver.set_duty_cycle_percent(50).unwrap();
    /*
    loop {
        // 100% duty cycle, fully on
        pwm_driver.set_duty_cycle_fully_on().unwrap();
        Timer::after_secs(1).await;

        // 66% duty cycle. Expressed as simple percentage.
        pwm_driver.set_duty_cycle_percent(66).unwrap();
        Timer::after_secs(1).await;

        // 25% duty cycle. Expressed as 32768/4 = 8192.
        pwm_driver.set_duty_cycle(c.top / 4).unwrap();
        Timer::after_secs(1).await;

        // 0% duty cycle, fully off.
        pwm_driver.set_duty_cycle_fully_off().unwrap();
        Timer::after_secs(1).await;
    }
    */
}

fn pwm_calc_divider_and_top(freq: u32) -> (f32, u16) {
    let clk_sys_freq = rp::clocks::clk_sys_freq();
    let mut best_divider = 1.0_f32;
    let mut best_top = u16::MAX;
    if freq == 0 {
        return (best_divider, best_top);
    }
    let target_divisor = clk_sys_freq / freq;
    let mut best_error = u32::MAX;
    for divider_int in 1..=256_u32 {
        let top_plus_1 = target_divisor / divider_int;
        if !(1..=65_536).contains(&top_plus_1) {
            continue;
        }
        let top = (top_plus_1 - 1) as u16;
        let actual_divisor = divider_int * (u32::from(top) + 1);
        let actual_freq = clk_sys_freq / actual_divisor;
        let error = actual_freq.abs_diff(freq);
        if error < best_error {
            best_error = error;
            best_divider = divider_int as f32;
            best_top = top;
        }
    }
    for divider_int in 1..=255_u32 {
        for frac in 1..=15_u32 {
            let divider_frac = divider_int as f32 + frac as f32 / 16.0;
            let top_plus_1 = (target_divisor as f32 / divider_frac) as u32;
            if !(1..=65_536).contains(&top_plus_1) {
                continue;
            }
            let top = (top_plus_1 - 1) as u16;
            let actual_freq = (clk_sys_freq as f32
                / (divider_frac * (u32::from(top) + 1) as f32)) as u32;
            let error = actual_freq.abs_diff(freq);
            if error < best_error {
                best_error = error;
                best_divider = divider_frac;
                best_top = top;
            }
        }
    }
    (best_divider, best_top)
}
