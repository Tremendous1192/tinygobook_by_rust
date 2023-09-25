//! 組込みRustのおまじない
#![no_std] // 必須アトリビュート
#![no_main] // 必須アトリビュート
use panic_halt as _; // 必須クレート
use wio::prelude::*; // ほぼ必須
use wio_terminal as wio; // 必須クレート

// ボタン操作
use cortex_m::interrupt::{free as disable_interrupts, CriticalSection};
use heapless::spsc::Queue;
use wio::wifi_prelude::interrupt;
use wio::{button_interrupt, Button, ButtonController, ButtonEvent};
static mut BUTTON_CTRLR: Option<ButtonController> = None;
static mut Q: Queue<ButtonEvent, 8_usize> = Queue::new();
button_interrupt!(
    BUTTON_CTRLR,
    unsafe fn on_button_event(_cs: &CriticalSection, event: ButtonEvent) {
        let mut q = Q.split().0;
        q.enqueue(event).ok();

        // mainループの重い処理中にも内蔵LEDの点灯処理を割り込ませることができる
        let mut consumer = unsafe { Q.split().1 };
        if let Some(press) = consumer.dequeue() {
            unsafe {
                if let Some(user_led) = USER_LED.as_mut() {
                    match press.button {
                        Button::TopLeft => {}
                        Button::TopMiddle => {}
                        Button::Left => {
                            user_led.set_high().ok();
                        }
                        Button::Right => {
                            user_led.set_low().ok();
                        }
                        Button::Down => {
                            user_led.set_low().ok();
                        }
                        Button::Up => {
                            user_led.set_high().ok();
                        }
                        Button::Click => {}
                    }
                }
            }
        }
        // ここまでボタン割り込み処理
    }
);

// 内蔵LED
use atsamd_hal::gpio::{Output, Pin, PushPull, PA15};
static mut USER_LED: Option<Pin<PA15, Output<PushPull>>> = None;

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
    unsafe {
        USER_LED = Some(pins.user_led.into_push_pull_output());
    }

    // ボタン
    let button_ctrlr = wio::ButtonPins {
        button1:/*Button1Reset*/ pins.button1,
        button2:/*Button2Reset*/ pins.button2,
        button3:/*Button3Reset*/ pins.button3,
        switch_x:/*SwitchXReset*/ pins.switch_x,
        switch_y:/*SwitchYReset*/ pins.switch_y,
        switch_z:/*SwitchZReset*/ pins.switch_z,
        switch_u:/*SwitchUReset*/ pins.switch_u,
        switch_b:/*SwitchBReset*/ pins.switch_b,
    }
    .init(peripherals.EIC, &mut clocks, &mut peripherals.MCLK);
    let nvic = &mut core.NVIC;
    disable_interrupts(|_| unsafe {
        button_ctrlr.enable(nvic);
        BUTTON_CTRLR = Some(button_ctrlr);
    });
    // ここまで 初期化

    // 組込みはloop必須
    // 疑似的な重い処理
    loop {
        delay.delay_ms(10_000_u16);
    }
    // ここまでloop処理
}
// ここまでmain関数
