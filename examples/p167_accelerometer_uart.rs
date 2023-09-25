//! 組込みRustのおまじない
#![no_std] // 必須アトリビュート
#![no_main] // 必須アトリビュート
use panic_halt as _; // 必須クレート
use wio::prelude::*; // ほぼ必須
use wio_terminal as wio; // 必須クレート

// 文字列操作
use core::fmt::Write;
use heapless::String;

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

    // UARTドライバオブジェクト
    let mut serial = wio::Uart {
        tx:/*UartTxReset*/ pins.uart_tx,
        rx:/*UartRxReset*/ pins.uart_rx,
    }
    .init(
        &mut clocks,
        9600.Hz(),
        peripherals.SERCOM2,
        &mut peripherals.MCLK,
    );

    // 浮動小数型を文字列に変換する
    let mut buffer = ryu::Buffer::new();
    // ここまで 初期化

    // 組込みはloop必須
    // 1秒ごとの内蔵加速度センサの値をUARTで送信する
    loop {
        // 加速度センサの出力値
        let accelerometer::vector::F32x3 { x, y, z } = lis3dh.accel_norm().unwrap();

        // 加速度を表示用文字列に変換する
        let mut acceleration = heapless::String::<128_usize>::new();
        write!(acceleration, "x: {:4}", x).unwrap();
        write!(acceleration, ". y: {:4}", y).unwrap();
        writeln!(acceleration, ". z: {:4}", z).unwrap();
        /*
        let _ = acceleration.push_str("x: ");
        let _ = acceleration.push_str(buffer.format((x * 1_000_f32).round() / 1_000_f32));
        let _ = acceleration.push_str(". y: ");
        let _ = acceleration.push_str(buffer.format((y * 1_000_f32).round() / 1_000_f32));
        let _ = acceleration.push_str(". z: ");
        let _ = acceleration.push_str(buffer.format((z * 1_000_f32).round() / 1_000_f32));
        let _ = acceleration.push_str("\n");
        */

        // Tera Term に加速度を送信する
        for c in acceleration.as_bytes().iter() {
            nb::block!(serial.write(*c)).unwrap();
        }

        delay.delay_ms(1_000_u16);
    }
    // ここまでloop処理
}
// ここまでmain関数
