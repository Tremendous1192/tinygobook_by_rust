//! 組込みRustのおまじない
#![no_std] // 必須アトリビュート
#![no_main] // 必須アトリビュート
use panic_halt as _; // 必須クレート
use wio::prelude::*; // ほぼ必須
use wio_terminal as wio; // 必須クレート

// UARTドライバオブジェクト
static mut SERIAL: Option<
    atsamd_hal::sercom::uart::Uart<
        atsamd_hal::sercom::uart::Config<wio::UartPads>,
        atsamd_hal::sercom::uart::Duplex,
    >,
> = None;

// シリアル通信でリテラル文字列を送信する
fn send_message(message: &'static [u8]) {
    unsafe {
        if let Some(serial) = SERIAL.as_mut() {
            for c in message.iter() {
                nb::block!(serial.write(*c)).unwrap();
            }
        }
    }
}

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

    // UARTドライバオブジェクト
    unsafe {
        SERIAL = Some(
            wio::Uart {
            tx:/*UartTxReset*/ pins.uart_tx,
            rx:/*UartRxReset*/ pins.uart_rx,
        }
            .init(
                &mut clocks,
                9600.Hz(),
                peripherals.SERCOM2,
                &mut peripherals.MCLK,
            ),
        );
    }

    // ここまで 初期化

    // Tera Term にメッセージを送信する
    let message = "message-to-uart\n";
    send_message(message.as_bytes());

    // 組込みはloop必須
    loop {}
    // ここまでloop処理
}
// ここまでmain関数
