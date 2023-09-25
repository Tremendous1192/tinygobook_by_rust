//! 組込みRustのおまじない
#![no_std] // 必須アトリビュート
#![no_main] // 必須アトリビュート
use panic_halt as _; // 必須クレート
use wio::prelude::*; // ほぼ必須
use wio_terminal as wio; // 必須クレート

use micromath::F32Ext; // 組込み向け数学処理

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

    // 内蔵LED
    let mut user_led = pins.user_led.into_push_pull_output();

    // 加速度計
    let mut lis3dh = wio::Accelerometer{
        scl:/*I2c0SclReset*/ pins.i2c0_scl,
        sda:/*I2c0SdaReset*/ pins.i2c0_sda,
    }
    .init(&mut clocks, peripherals.SERCOM4, &mut peripherals.MCLK);
    // ここまで 初期化

    // 組込みはloop必須
    // 加速度の大きさが1以上のときに内蔵LEDを点灯する
    loop {
        // 加速度センサの値を読み取る
        let accelerometer::vector::F32x3 { x, y, z } = lis3dh.accel_norm().unwrap();
        let speed = ((x * x + y * y + z * z) as f32).sqrt();

        // LED点滅
        if speed >= 1_f32 {
            user_led.set_high().ok();
        } else {
            user_led.set_low().ok();
        }
        delay.delay_ms(10_u16);
    }
    // ここまでloop処理
}
// ここまでmain関数
