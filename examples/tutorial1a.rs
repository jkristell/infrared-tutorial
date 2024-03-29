#![no_std]
#![no_main]

use cortex_m_rt::entry;
use cortex_m_semihosting::hprintln;
use panic_semihosting as _;
use stm32f1xx_hal::{
    gpio::{gpiob::PB8, Floating, Input},
    pac,
    prelude::*,
    stm32::{interrupt, TIM2},
    timer::{Event, Timer, CountDownTimer},
};

use infrared::{
    Command,
    InfraredReceiver,
    rc6::*
};

// Sample rate
const TIMER_FREQ: u32 = 20_000;

// Our receivertype
type Receiver = InfraredReceiver<PB8<Input<Floating>>, Rc6>;

// Globals
static mut TIMER: Option<CountDownTimer<TIM2>> = None;
static mut RECEIVER: Option<Receiver> = None;

#[entry]
fn main() -> ! {
    let _core = cortex_m::Peripherals::take().unwrap();
    let device = pac::Peripherals::take().unwrap();

    let mut flash = device.FLASH.constrain();
    let mut rcc = device.RCC.constrain();

    let clocks = rcc
        .cfgr
        .use_hse(8.mhz())
        .sysclk(48.mhz())
        .pclk1(24.mhz())
        .freeze(&mut flash.acr);

    let mut gpiob = device.GPIOB.split(&mut rcc.apb2);
    let pin = gpiob.pb8.into_floating_input(&mut gpiob.crh);

    let mut timer = Timer::tim2(device.TIM2, &clocks, &mut rcc.apb1)
        .start_count_down(TIMER_FREQ.hz());

    timer.listen(Event::Update);

    let receiver = InfraredReceiver::new(pin, TIMER_FREQ);

    // Safe because the devices are only used in the interrupt handler
    unsafe {
        TIMER.replace(timer);
        RECEIVER.replace(receiver);
    }

    unsafe {
        // Enable the timer interrupt
        pac::NVIC::unmask(pac::Interrupt::TIM2);
    }

    hprintln!("Ready!").unwrap();

    loop {
        continue;
    }
}

#[interrupt]
fn TIM2() {
    static mut SAMPLECOUNTER: u32 = 0;

    let receiver = unsafe { RECEIVER.as_mut().unwrap() };

    if let Ok(Some(cmd)) = receiver.sample(*SAMPLECOUNTER) {
        let _ = hprintln!("Cmd: {} {}", cmd.address(), cmd.command());
    }

    // Clear the interrupt
    let timer = unsafe { TIMER.as_mut().unwrap() };
    timer.clear_update_interrupt_flag();

    *SAMPLECOUNTER = SAMPLECOUNTER.wrapping_add(1);
}
