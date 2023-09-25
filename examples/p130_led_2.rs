#![no_std]
#![no_main]
use panic_halt as _;
use wio::prelude::*;
use wio_terminal as wio;

// 内蔵LEDのグローバル変数
static mut USER_LED: Option<
    atsamd_hal::gpio::Pin<
        atsamd_hal::gpio::PA15,
        atsamd_hal::gpio::Output<atsamd_hal::gpio::PushPull>,
    >,
> = None;

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
    unsafe {
        USER_LED = Some(pins.user_led.into_push_pull_output());
    }

    loop {
        // LEDを1秒点灯して0.1秒消灯する
        unsafe {
            if let Some(user_led) = USER_LED.as_mut() {
                user_led.set_high().ok();
                delay.delay_ms(1_000_u16);
                user_led.set_low().ok();
                delay.delay_ms(100_u16);
            }
        }
    }
}
