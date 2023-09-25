#![no_std]
#![no_main]
use panic_halt as _;
use wio::prelude::*;
use wio_terminal as wio;

#[wio::entry]
fn main() -> ! {
    // 必須インスタンス
    let mut peripherals = wio::pac::Peripherals::take().unwrap();
    let mut core = wio::pac::CorePeripherals::take().unwrap();
    let mut clocks = wio::hal::clock::GenericClockController::with_external_32kosc(
        peripherals.GCLK,
        &mut peripherals.MCLK,
        &mut peripherals.OSC32KCTRL,
        &mut peripherals.OSCCTRL,
        &mut peripherals.NVMCTRL,
    );
    let mut delay = wio::hal::delay::Delay::new(core.SYST, &mut clocks);
    let pins = wio::Pins::new(peripherals.PORT);

    // 内蔵LED
    let mut user_led = pins.user_led.into_push_pull_output();

    loop {
        // LEDを1秒点灯して0.1秒消灯する
        user_led.set_high().ok();
        delay.delay_ms(1_000_u16);
        user_led.set_low().ok();
        delay.delay_ms(100_u16);
    }
}
