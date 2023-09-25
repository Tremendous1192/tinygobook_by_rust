//! 組込みRustのおまじない
#![no_std] // 必須アトリビュート
#![no_main] // 必須アトリビュート
use panic_halt as _; // 必須クレート
use wio::prelude::*; // ほぼ必須
use wio_terminal as wio; // 必須クレート

#[wio::entry] // 必須アトリビュート
fn main() -> ! {
    // 初期化
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

    // GPIO
    let pins = wio::Pins::new(peripherals.PORT);

    // 光度センサー
    let (mut adc1, mut light_sensor_adc) = wio::LightSensor {
            pd1:/*LightSensorAdcReset*/ pins.fpc_d13_a13,
        }
    .init(peripherals.ADC1, &mut clocks, &mut peripherals.MCLK);

    // 内蔵LED
    let mut user_led = pins.user_led.into_push_pull_output();
    // ここまで 初期化

    // 組込みはloop必須
    // 光センサの入力値が50より小さいときに内蔵LEDを点灯する
    loop {
        // 光センサの入力値
        let value: u16 = nb::block!(adc1.read(&mut light_sensor_adc)).unwrap();
        if value < 50_u16 {
            let _ = user_led.set_high();
        } else {
            let _ = user_led.set_low();
        }
    }
    // ここまでloop処理
}
// ここまでmain関数
