//! This example shows powerful PIO module in the RP2040 chip.

#![no_std]
#![no_main]
use defmt::info;
use embassy_executor::Spawner;
use embassy_rp as rp;
use fixed::traits::ToFixed as _;
use fixed_macro::types::U56F8;
use {defmt_rtt as _, panic_probe as _};

rp::bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => rp::pio::InterruptHandler<rp::peripherals::PIO0>;
});

fn setup_pio_task_sm0<'d>(pio: &mut rp::pio::Common<'d, rp::peripherals::PIO0>, sm: &mut rp::pio::StateMachine<'d, rp::peripherals::PIO0, 0>, pin: rp::Peri<'d, impl rp::pio::PioPin>) {
    // Setup sm0

    // Send data serially to pin
    let prg = rp::pio::program::pio_asm!(
        ".origin 16",
        "set pindirs, 1",
        ".wrap_target",
        "out pins,1 [19]",
        ".wrap",
    );

    let mut cfg = rp::pio::Config::default();
    cfg.use_program(&pio.load_program(&prg.program), &[]);
    let out_pin = pio.make_pio_pin(pin);
    cfg.set_out_pins(&[&out_pin]);
    cfg.set_set_pins(&[&out_pin]);
    cfg.clock_divider = (U56F8!(125_000_000) / 20 / 200).to_fixed();
    cfg.shift_out.auto_fill = true;
    sm.set_config(&cfg);
}

#[embassy_executor::task]
async fn pio_task_sm0(mut sm: rp::pio::StateMachine<'static, rp::peripherals::PIO0, 0>) {
    sm.set_enable(true);

    let mut v = 0x0f0caffa;
    loop {
        sm.tx().wait_push(v).await;
        v ^= 0xffff;
        info!("Pushed {:032b} to FIFO", v);
    }
}

fn setup_pio_task_sm1<'d>(pio: &mut rp::pio::Common<'d, rp::peripherals::PIO0>, sm: &mut rp::pio::StateMachine<'d, rp::peripherals::PIO0, 1>) {
    // Setupm sm1

    // Read 0b10101 repeatedly until ISR is full
    let prg = rp::pio::program::pio_asm!(
        //
        ".origin 8",
        "set x, 0x15",
        ".wrap_target",
        "in x, 5 [31]",
        ".wrap",
    );

    let mut cfg = rp::pio::Config::default();
    cfg.use_program(&pio.load_program(&prg.program), &[]);
    cfg.clock_divider = (U56F8!(125_000_000) / 2000).to_fixed();
    cfg.shift_in.auto_fill = true;
    cfg.shift_in.direction = rp::pio::ShiftDirection::Right;
    sm.set_config(&cfg);
}

#[embassy_executor::task]
async fn pio_task_sm1(mut sm: rp::pio::StateMachine<'static, rp::peripherals::PIO0, 1>) {
    sm.set_enable(true);
    loop {
        let rx = sm.rx().wait_pull().await;
        info!("Pulled {:032b} from FIFO", rx);
    }
}

fn setup_pio_task_sm2<'d>(pio: &mut rp::pio::Common<'d, rp::peripherals::PIO0>, sm: &mut rp::pio::StateMachine<'d, rp::peripherals::PIO0, 2>) {
    // Setup sm2

    // Repeatedly trigger IRQ 3
    let prg = rp::pio::program::pio_asm!(
        ".origin 0",
        ".wrap_target",
        "set x,10",
        "delay:",
        "jmp x-- delay [15]",
        "irq 3 [15]",
        ".wrap",
    );
    let mut cfg = rp::pio::Config::default();
    cfg.use_program(&pio.load_program(&prg.program), &[]);
    cfg.clock_divider = (U56F8!(125_000_000) / 2000).to_fixed();
    sm.set_config(&cfg);
}

#[embassy_executor::task]
async fn pio_task_sm2(mut irq: rp::pio::Irq<'static, rp::peripherals::PIO0, 3>, mut sm: rp::pio::StateMachine<'static, rp::peripherals::PIO0, 2>) {
    sm.set_enable(true);
    loop {
        irq.wait().await;
        info!("IRQ trigged");
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());
    let mut pio0 = rp::pio::Pio::new(p.PIO0, Irqs);
    setup_pio_task_sm0(&mut pio0.common, &mut pio0.sm0, p.PIN_0);
    setup_pio_task_sm1(&mut pio0.common, &mut pio0.sm1);
    setup_pio_task_sm2(&mut pio0.common, &mut pio0.sm2);
    spawner.spawn(pio_task_sm0(pio0.sm0).unwrap());
    spawner.spawn(pio_task_sm1(pio0.sm1).unwrap());
    spawner.spawn(pio_task_sm2(pio0.irq3, pio0.sm2).unwrap());
}
