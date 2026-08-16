#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_rp as rp;
use embassy_rp::pwm::SetDutyCycle as _; // for set_duty_cycle_fraction()
use fixed::traits::ToFixed as _;        // for to_fixed()
use {defmt_rtt as _, panic_probe as _};

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = rp::init(Default::default());
    let mut config = rp::pwm::Config::default();
    let (divider, top) = pwm_calc_divider_and_top(rp::clocks::clk_sys_freq(), 1000);
    config.divider = divider.to_fixed();
    config.top = top;
    assign_pwm_a_first_half(p, config.clone());
    //assign_pwm_a_second_half(p, config.clone());
    //assign_pwm_b_first_half(p, config.clone());
    //assign_pwm_b_second_half(p, config.clone());
    //assign_pwm_ab_first_half(p, config.clone());
    //assign_pwm_ab_second_half(p, config.clone());
} 

// a function that can take rp::pwm::Pwm and rp:pwm::PwmOutput
fn test_func(mut pwm: impl rp::pwm::SetDutyCycle) {
    pwm.set_duty_cycle_fraction(50, 100).unwrap();
}

fn assign_pwm_a_first_half(p: rp::Peripherals, config: rp::pwm::Config) {
    // create rp::pwm::Pwm instances
    let mut pwm_0 = rp::pwm::Pwm::new_output_a(p.PWM_SLICE0, p.PIN_0, config.clone());
    let mut pwm_2 = rp::pwm::Pwm::new_output_a(p.PWM_SLICE1, p.PIN_2, config.clone());
    let mut pwm_4 = rp::pwm::Pwm::new_output_a(p.PWM_SLICE2, p.PIN_4, config.clone());
    let mut pwm_6 = rp::pwm::Pwm::new_output_a(p.PWM_SLICE3, p.PIN_6, config.clone());
    let mut pwm_8 = rp::pwm::Pwm::new_output_a(p.PWM_SLICE4, p.PIN_8, config.clone());
    let mut pwm_10 = rp::pwm::Pwm::new_output_a(p.PWM_SLICE5, p.PIN_10, config.clone());
    let mut pwm_12 = rp::pwm::Pwm::new_output_a(p.PWM_SLICE6, p.PIN_12, config.clone());
    let mut pwm_14 = rp::pwm::Pwm::new_output_a(p.PWM_SLICE7, p.PIN_14, config.clone());
    test_func(pwm_0);
    test_func(pwm_2);
    test_func(pwm_4);
    test_func(pwm_6);
    test_func(pwm_8);
    test_func(pwm_10);
    test_func(pwm_12);
    test_func(pwm_14);
}

fn assign_pwm_a_second_half(p: rp::Peripherals, config: rp::pwm::Config) {
    // create rp::pwm::Pwm instances
    let mut pwm_16 = rp::pwm::Pwm::new_output_a(p.PWM_SLICE0, p.PIN_16, config.clone());
    let mut pwm_18 = rp::pwm::Pwm::new_output_a(p.PWM_SLICE1, p.PIN_18, config.clone());
    let mut pwm_20 = rp::pwm::Pwm::new_output_a(p.PWM_SLICE2, p.PIN_20, config.clone());
    let mut pwm_22 = rp::pwm::Pwm::new_output_a(p.PWM_SLICE3, p.PIN_22, config.clone());
    let mut pwm_24 = rp::pwm::Pwm::new_output_a(p.PWM_SLICE4, p.PIN_24, config.clone());
    let mut pwm_26 = rp::pwm::Pwm::new_output_a(p.PWM_SLICE5, p.PIN_26, config.clone());
    let mut pwm_28 = rp::pwm::Pwm::new_output_a(p.PWM_SLICE6, p.PIN_28, config.clone());
    test_func(pwm_16);
    test_func(pwm_18);
    test_func(pwm_20);
    test_func(pwm_22);
    test_func(pwm_24);
    test_func(pwm_26);
    test_func(pwm_28);
}

fn assign_pwm_b_first_half(p: rp::Peripherals, config: rp::pwm::Config) {
    // create rp::pwm::Pwm instances
    let mut pwm_1 = rp::pwm::Pwm::new_output_b(p.PWM_SLICE0, p.PIN_1, config.clone());
    let mut pwm_3 = rp::pwm::Pwm::new_output_b(p.PWM_SLICE1, p.PIN_3, config.clone());
    let mut pwm_5 = rp::pwm::Pwm::new_output_b(p.PWM_SLICE2, p.PIN_5, config.clone());
    let mut pwm_7 = rp::pwm::Pwm::new_output_b(p.PWM_SLICE3, p.PIN_7, config.clone());
    let mut pwm_9 = rp::pwm::Pwm::new_output_b(p.PWM_SLICE4, p.PIN_9, config.clone());
    let mut pwm_11 = rp::pwm::Pwm::new_output_b(p.PWM_SLICE5, p.PIN_11, config.clone());
    let mut pwm_13 = rp::pwm::Pwm::new_output_b(p.PWM_SLICE6, p.PIN_13, config.clone());
    let mut pwm_15 = rp::pwm::Pwm::new_output_b(p.PWM_SLICE7, p.PIN_15, config.clone());
    test_func(pwm_1);
    test_func(pwm_3);
    test_func(pwm_5);
    test_func(pwm_7);
    test_func(pwm_9);
    test_func(pwm_11);
    test_func(pwm_13);
    test_func(pwm_15);
}

fn assign_pwm_b_second_half(p: rp::Peripherals, config: rp::pwm::Config) {
    // create rp::pwm::Pwm instances
    let mut pwm_17 = rp::pwm::Pwm::new_output_b(p.PWM_SLICE0, p.PIN_17, config.clone());
    let mut pwm_19 = rp::pwm::Pwm::new_output_b(p.PWM_SLICE1, p.PIN_19, config.clone());
    let mut pwm_21 = rp::pwm::Pwm::new_output_b(p.PWM_SLICE2, p.PIN_21, config.clone());
    let mut pwm_23 = rp::pwm::Pwm::new_output_b(p.PWM_SLICE3, p.PIN_23, config.clone());
    let mut pwm_25 = rp::pwm::Pwm::new_output_b(p.PWM_SLICE4, p.PIN_25, config.clone());
    let mut pwm_27 = rp::pwm::Pwm::new_output_b(p.PWM_SLICE5, p.PIN_27, config.clone());
    let mut pwm_29 = rp::pwm::Pwm::new_output_b(p.PWM_SLICE6, p.PIN_29, config.clone());
    test_func(pwm_17);
    test_func(pwm_19);
    test_func(pwm_21);
    test_func(pwm_23);
    test_func(pwm_25);
    test_func(pwm_27);
    test_func(pwm_29);
}

fn assign_pwm_ab_first_half(p: rp::Peripherals, config: rp::pwm::Config) {
    // create rp::pwm::PwmOutput instances
    let (Some(mut pwm_0), Some(mut pwm_1)) = rp::pwm::Pwm::new_output_ab(
        p.PWM_SLICE0, p.PIN_0, p.PIN_1, config.clone()).split() else { panic!(); };
    let (Some(mut pwm_2), Some(mut pwm_3)) = rp::pwm::Pwm::new_output_ab(
        p.PWM_SLICE1, p.PIN_2, p.PIN_3, config.clone()).split() else { panic!(); };
    let (Some(mut pwm_4), Some(mut pwm_5)) = rp::pwm::Pwm::new_output_ab(
        p.PWM_SLICE2, p.PIN_4, p.PIN_5, config.clone()).split() else { panic!(); };
    let (Some(mut pwm_6), Some(mut pwm_7)) = rp::pwm::Pwm::new_output_ab(
        p.PWM_SLICE3, p.PIN_6, p.PIN_7, config.clone()).split() else { panic!(); };
    let (Some(mut pwm_8), Some(mut pwm_9)) = rp::pwm::Pwm::new_output_ab(
        p.PWM_SLICE4, p.PIN_8, p.PIN_9, config.clone()).split() else { panic!(); };
    let (Some(mut pwm_10), Some(mut pwm_11)) = rp::pwm::Pwm::new_output_ab(
        p.PWM_SLICE5, p.PIN_10, p.PIN_11, config.clone()).split() else { panic!(); };
    let (Some(mut pwm_12), Some(mut pwm_13)) = rp::pwm::Pwm::new_output_ab(
        p.PWM_SLICE6, p.PIN_12, p.PIN_13, config.clone()).split() else { panic!(); };
    let (Some(mut pwm_14), Some(mut pwm_15)) = rp::pwm::Pwm::new_output_ab(
        p.PWM_SLICE7, p.PIN_14, p.PIN_15, config.clone()).split() else { panic!(); };
    test_func(pwm_0);
    test_func(pwm_1);
    test_func(pwm_2);
    test_func(pwm_3);
    test_func(pwm_4);
    test_func(pwm_5);
    test_func(pwm_6);
    test_func(pwm_7);
    test_func(pwm_8);
    test_func(pwm_9);
    test_func(pwm_10);
    test_func(pwm_11);
    test_func(pwm_12);
    test_func(pwm_13);
    test_func(pwm_14);
    test_func(pwm_15);
}

fn assign_pwm_ab_second_half(p: rp::Peripherals, config: rp::pwm::Config) {
    // create rp::pwm::PwmOutput instances
    let (Some(mut pwm_16), Some(mut pwm_17)) = rp::pwm::Pwm::new_output_ab(
        p.PWM_SLICE0, p.PIN_16, p.PIN_17, config.clone()).split() else { panic!(); };
    let (Some(mut pwm_18), Some(mut pwm_19)) = rp::pwm::Pwm::new_output_ab(
        p.PWM_SLICE1, p.PIN_18, p.PIN_19, config.clone()).split() else { panic!(); };
    let (Some(mut pwm_20), Some(mut pwm_21)) = rp::pwm::Pwm::new_output_ab(
        p.PWM_SLICE2, p.PIN_20, p.PIN_21, config.clone()).split() else { panic!(); };
    let (Some(mut pwm_22), Some(mut pwm_23)) = rp::pwm::Pwm::new_output_ab(
        p.PWM_SLICE3, p.PIN_22, p.PIN_23, config.clone()).split() else { panic!(); };
    let (Some(mut pwm_24), Some(mut pwm_25)) = rp::pwm::Pwm::new_output_ab(
        p.PWM_SLICE4, p.PIN_24, p.PIN_25, config.clone()).split() else { panic!(); };
    let (Some(mut pwm_26), Some(mut pwm_27)) = rp::pwm::Pwm::new_output_ab(
        p.PWM_SLICE5, p.PIN_26, p.PIN_27, config.clone()).split() else { panic!(); };
    let (Some(mut pwm_28), Some(mut pwm_29)) = rp::pwm::Pwm::new_output_ab(
        p.PWM_SLICE6, p.PIN_28, p.PIN_29, config.clone()).split() else { panic!(); };
    test_func(pwm_16);
    test_func(pwm_17);
    test_func(pwm_18);
    test_func(pwm_19);
    test_func(pwm_20);
    test_func(pwm_21);
    test_func(pwm_22);
    test_func(pwm_23);
    test_func(pwm_24);
    test_func(pwm_25);
    test_func(pwm_26);
    test_func(pwm_27);
    test_func(pwm_28);
    test_func(pwm_29);
}

fn pwm_calc_divider_and_top(clk_sys_freq: u32, freq: u32) -> (f32, u16) {
    let mut divider_rough: u32 = 1;
    let mut top_rtn: u16 = u16::MAX;
    if freq == 0 { return (divider_rough as f32, top_rtn); }
    let freqdiv: f32 = clk_sys_freq as f32 / freq as f32;
    let mut freqdiff_min: u32 = u32::MAX;
    for divider in 1..=256_u32 {
        let top_plus_1: u32 = (freqdiv / divider as f32) as u32;
        if !(1..=65_536).contains(&top_plus_1) { continue; }
        let top: u16 = (top_plus_1 - 1) as u16;
        let freq_actual: u32 = clk_sys_freq / (divider * top_plus_1);
        let freqdiff: u32 = freq_actual.abs_diff(freq);
        if freqdiff_min > freqdiff {
            freqdiff_min = freqdiff;
            divider_rough = divider;
            top_rtn = top;
        }
    }
    let mut divider_rtn: f32 = divider_rough as f32;
    for divider_frac in 0..=15_u32 {
        let divider: f32 = divider_rough as f32 + divider_frac as f32 / 16.0;
        let top_plus_1: u32 = (freqdiv / divider) as u32;
        if !(1..=65_536).contains(&top_plus_1) { continue; }
        let top: u16 = (top_plus_1 - 1) as u16;
        let freq_actual: u32 = (clk_sys_freq as f32 / (divider * top_plus_1 as f32)) as u32;
        let freqdiff: u32 = freq_actual.abs_diff(freq);
        if freqdiff_min > freqdiff {
            freqdiff_min = freqdiff;
            divider_rtn = divider;
            top_rtn = top;
        }
    }
    (divider_rtn, top_rtn)
}

fn pwm_calc_freq(clk_sys_freq: u32, divider: f32, top: u16) -> u32 {
    let divisor = divider * (u32::from(top) + 1) as f32;
    (clk_sys_freq as f32 / divisor) as u32
}
