#![no_std]
#![no_main]
use panic_halt as _;
use wio::prelude::*;
use wio_terminal as wio;

// 内蔵LEDのグローバル変数
static mut USER_LED: Option<UserLED> = None;

// 内蔵LEDをグローバル変数で制御するための構造体
struct UserLED {
    user_led: atsamd_hal::gpio::Pin<
        atsamd_hal::gpio::PA15,
        atsamd_hal::gpio::Output<atsamd_hal::gpio::PushPull>,
    >,
}
impl UserLED {
    fn toggle_led(&mut self) {
        self.user_led.toggle().ok();
    }
    fn high_led(&mut self) {
        self.user_led.set_high().ok();
    }
    fn low_led(&mut self) {
        self.user_led.set_low().ok();
    }
}

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
        USER_LED = Some(UserLED {
            user_led: pins.user_led.into_push_pull_output(),
        });
    }

    loop {
        // LEDを1秒点灯して0.1秒消灯する
        unsafe {
            if let Some(led) = USER_LED.as_mut() {
                led.high_led();
                delay.delay_ms(1_000_u16);
                led.low_led();
                delay.delay_ms(100_u16);
            }
        }
    }
}
