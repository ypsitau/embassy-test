#![no_std]
#![no_main]
use defmt::info;
use embassy_futures as futures;
use embassy_executor::Spawner;
use embassy_rp as rp;
use fixed::traits::ToFixed as _;
use fixed_macro::types::U56F8;
use {defmt_rtt as _, panic_probe as _};

rp::bind_interrupts!(struct Irqs {
    PIO1_IRQ_0 => rp::pio::InterruptHandler<rp::peripherals::PIO1>;
});

fn setup_pio_task0<'d, PIO: rp::pio::Instance, const SM: usize> (
    pio: &mut rp::pio::Common<'d, PIO>,
    sm: &mut rp::pio::StateMachine<'d, PIO, SM>,
    pin: rp::Peri<'d, impl rp::pio::PioPin>
) {
    // Send data serially to pin
    let program = rp::pio::program::pio_asm!(
        ".origin 16",
        "set pindirs, 1",
        ".wrap_target",
        "out pins,1 [19]",
        ".wrap",
    );
    let mut config = rp::pio::Config::default();
    config.use_program(&pio.load_program(&program.program), &[]);
    let out_pin = pio.make_pio_pin(pin);
    config.set_out_pins(&[&out_pin]);
    config.set_set_pins(&[&out_pin]);
    config.clock_divider = (U56F8!(125_000_000) / 20 / 200).to_fixed();
    config.shift_out.auto_fill = true;
    sm.set_config(&config);
}

async fn pio_task0<PIO: rp::pio::Instance, const SM: usize>(
    mut sm: rp::pio::StateMachine<'static, PIO, SM>
) {
    sm.set_enable(true);
    let mut v = 0x0f0caffa;
    loop {
        sm.tx().wait_push(v).await;
        v ^= 0xffff;
        info!("Pushed {:032b} to FIFO", v);
    }
}

fn setup_pio_task1<'d, PIO: rp::pio::Instance, const SM: usize>(
    pio: &mut rp::pio::Common<'d, PIO>,
    sm: &mut rp::pio::StateMachine<'d, PIO, SM>,
) {
    // Read 0b10101 repeatedly until ISR is full
    let program = rp::pio::program::pio_asm!(
        //
        ".origin 8",
        "set x, 0x15",
        ".wrap_target",
        "in x, 5 [31]",
        ".wrap",
    );
    let mut config = rp::pio::Config::default();
    config.use_program(&pio.load_program(&program.program), &[]);
    config.clock_divider = (U56F8!(125_000_000) / 2000).to_fixed();
    config.shift_in.auto_fill = true;
    config.shift_in.direction = rp::pio::ShiftDirection::Right;
    sm.set_config(&config);
}

async fn pio_task1<PIO: rp::pio::Instance, const SM: usize>(
    mut sm: rp::pio::StateMachine<'static, PIO, SM>
) {
    sm.set_enable(true);
    loop {
        let rx = sm.rx().wait_pull().await;
        info!("Pulled {:032b} from FIFO", rx);
    }
}

fn setup_pio_task2<'d, PIO: rp::pio::Instance, const SM: usize>(
    pio: &mut rp::pio::Common<'d, PIO>,
    sm: &mut rp::pio::StateMachine<'d, PIO, SM>,
) {
    // Repeatedly trigger IRQ
    let program = rp::pio::program::pio_asm!(
        ".origin 0",
        ".wrap_target",
        "set x,10",
        "delay:",
        "jmp x-- delay [15]",
        "irq 3 [15]",
        ".wrap",
    );
    let mut config = rp::pio::Config::default();
    config.use_program(&pio.load_program(&program.program), &[]);
    config.clock_divider = (U56F8!(125_000_000) / 2000).to_fixed();
    sm.set_config(&config);
}

async fn pio_task2<PIO: rp::pio::Instance, const SM: usize, const IRQ: usize>(
    mut irq: rp::pio::Irq<'static, PIO, IRQ>,
    mut sm: rp::pio::StateMachine<'static, PIO, SM>
) {
    sm.set_enable(true);
    loop {
        irq.wait().await;
        info!("IRQ trigged");
    }
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_rp::init(Default::default());
    let mut pio = rp::pio::Pio::new(p.PIO1, Irqs);
    setup_pio_task0(&mut pio.common, &mut pio.sm3, p.PIN_0);
    setup_pio_task1(&mut pio.common, &mut pio.sm1);
    setup_pio_task2(&mut pio.common, &mut pio.sm2);
    futures::join::join3(
        pio_task0(pio.sm3),
        pio_task1(pio.sm1),
        pio_task2(pio.irq3, pio.sm2)
    ).await;
}
