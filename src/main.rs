//! 組込みRustのおまじない
#![no_std] // 必須アトリビュート
#![no_main] // 必須アトリビュート
use panic_halt as _; // 必須クレート
use wio::prelude::*; // ほぼ必須
use wio_terminal as wio; // 必須クレート

use core::fmt::Write;

// ヒープ
use heapless::spsc::Queue;
use heapless::String;

// SD カード
use embedded_sdmmc::{TimeSource, Timestamp, VolumeIdx};
struct Clock;
impl TimeSource for Clock {
    fn get_timestamp(&self) -> Timestamp {
        Timestamp {
            year_since_1970: 0,
            zero_indexed_month: 0,
            zero_indexed_day: 0,
            hours: 0,
            minutes: 0,
            seconds: 0,
        }
    }
}

// 描画
use eg::mono_font::{ascii::FONT_9X15, MonoTextStyle};
use eg::pixelcolor::Rgb565;
use eg::prelude::*;
use eg::text::{Baseline, Text};
use embedded_graphics as eg;

// 文字列変換
use itoa;
use ryu;

// マイク
use wio::hal::adc::InterruptAdc;
use wio::wifi_prelude::interrupt;
type ConversionMode = wio::hal::adc::FreeRunning;
#[interrupt]
fn ADC1_RESRDY() {
    unsafe {
        let ctx = CTX.as_mut().unwrap();
        let mut producer = ctx.samples.split().0;
        if let Some(sample) = ctx.adc.service_interrupt_ready() {
            producer.enqueue_unchecked(sample);
        }
    }
}
struct Ctx {
    adc: InterruptAdc<wio::pac::ADC1, ConversionMode>,
    samples: Queue<u16, 8_usize>,
}
static mut CTX: Option<Ctx> = None;

// ボタン操作
use cortex_m::interrupt::{free as disable_interrupts, CriticalSection};
use wio::{button_interrupt, Button, ButtonController, ButtonEvent};
static mut BUTTON_CTRLR: Option<ButtonController> = None;
static mut Q: Queue<ButtonEvent, 8_usize> = Queue::new();
button_interrupt!(
    BUTTON_CTRLR,
    unsafe fn on_button_event(_cs: &CriticalSection, event: ButtonEvent) {
        let mut q = Q.split().0;
        q.enqueue(event).ok();
    }
);

// Wi-Fi
use wio::hal::clock::GenericClockController;
use wio::hal::delay::Delay;
use wio::wifi_prelude::*;
use wio::wifi_rpcs as rpc;
use wio::wifi_singleton;
// Wi-Fiシングルトンと割り込み処理を生成するマクロ
// WIFI: Option<Wifi> = Some(Wifi::init(略));
wifi_singleton!(WIFI);

// 非同期処理
//use cortex_m::interrupt::free as disable_interrupts;

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

    // 便利コンポーネント
    //let sets = pins.split();

    // 加速度計
    // lis3dh::Lis3dh
    let mut lis3dh = wio::Accelerometer{
        scl:/*I2c0SclReset*/ pins.i2c0_scl,
        sda:/*I2c0SdaReset*/ pins.i2c0_sda,
    }
    .init(&mut clocks, peripherals.SERCOM4, &mut peripherals.MCLK);

    // ブザー
    let mut buzzer = wio::Buzzer {
      ctr:/*BuzzerCtrlReset*/pins.buzzer_ctr,
    }
    .init(&mut clocks, peripherals.TCC0, &mut peripherals.MCLK);

    // LCDディスプレイ
    let (mut display, _backlight) = wio::Display {
         miso:/*LcdMisoReset*/ pins.lcd_miso,
         mosi:/*LcdMosiReset*/ pins.lcd_mosi,
         sck:/*LcdSckReset*/ pins.lcd_sck,
         cs:/*LcdCsReset*/ pins.lcd_cs,
         dc:/*LcdDcReset*/ pins.lcd_dc,
         reset:/*LcdResetReset*/ pins.lcd_reset,
         backlight:/*LcdBacklightReset*/ pins.lcd_backlight,
    }
    .init(
        &mut clocks,
        peripherals.SERCOM7,
        &mut peripherals.MCLK,
        58.MHz(),
        &mut delay,
    )
    .unwrap();
    eg::primitives::Rectangle::with_corners(Point::new(0, 0), Point::new(320, 240))
        .into_styled(
            eg::primitives::PrimitiveStyleBuilder::new()
                .fill_color(Rgb565::BLACK)
                .build(),
        )
        .draw(&mut display)
        .ok()
        .unwrap();

    // 光度センサー
    /*
    let (mut adc1, mut light_sensor_adc) = wio::LightSensor {
        pd1:/*LightSensorAdcReset*/ pins.fpc_d13_a13,
    }
    .init(peripherals.ADC1, &mut clocks, &mut peripherals.MCLK);
    */

    // 内臓マイク
    let (mut microphone_adc, mut microphone_pin) = {
        let (adc, pin) = wio::Microphone{
            mic:/*MicOutputReset*/ pins.mic_output,

          }
        .init(peripherals.ADC1, &mut clocks, &mut peripherals.MCLK);
        let interrupt_adc: InterruptAdc<_, ConversionMode> = InterruptAdc::from(adc);
        (interrupt_adc, pin)
    };
    microphone_adc.start_conversion(&mut microphone_pin);
    unsafe {
        CTX = Some(Ctx {
            adc: microphone_adc,
            samples: Queue::new(),
        });
    }
    let mut consumer = unsafe { CTX.as_mut().unwrap().samples.split().1 };
    unsafe {
        cortex_m::peripheral::NVIC::unmask(interrupt::ADC1_RESRDY);
    }

    // SDカード制御
    let (mut cont, _sd_present) = wio::SDCard {
        cs:/*SdCsReset*/ pins.sd_cs,
        mosi:/*SdMosiReset*/ pins.sd_mosi,
        sck:/*SdSckReset*/ pins.sd_sck,
        miso:/*SdMisoReset*/ pins.sd_miso,
        det:/*SdDetReset*/ pins.sd_det,
    }
    .init(
        &mut clocks,
        peripherals.SERCOM6,
        &mut peripherals.MCLK,
        Clock, // TimeSource トレイト
    )
    .unwrap();

    // UARTドライバオブジェクト
    let mut serial: wio::HalUart = wio::Uart {
        tx:/*UartTxReset*/ pins.uart_tx,
        rx:/*UartRxReset*/ pins.uart_rx,
    }
    .init(
        &mut clocks,
        9600.Hz(),
        peripherals.SERCOM2,
        &mut peripherals.MCLK,
    );

    // 内蔵LED
    let mut user_led = pins.user_led.into_push_pull_output();
    user_led.set_low().unwrap();

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
    // データ送受信の格納先
    let mut consumer = unsafe { Q.split().1 };

    // wifi ペリフェラル
    let nvic = &mut core.NVIC;
    disable_interrupts(|cs| unsafe {
        wifi_init(
            cs,
            wio::WifiPins {
                pwr:/*WifiPwrReset*/ pins.rtl8720d_chip_pu,
                rxd:/*WifiRxdReset*/ pins.rtl8720d_rxd,
                txd:/*WifiTxdReset*/ pins.rtl8720d_txd,
                mosi:/*WifiTxReset*/ pins.rtl8720d_hspi_mosi,
                clk:/*WifiClkReset*/ pins.rtl8720d_hspi_clk,
                miso:/*WifiRxReset*/ pins.rtl8720d_hspi_miso,
                cs:/*WifiCsReset*/ pins.rtl8720d_hspi_cs,
                ready:/*WifiReadyReset*/ pins.rtl8720d_data_ready,
                dir:/*WifiDirReset*/ pins.rtl8720d_dir,
            },
            peripherals.SERCOM0,
            &mut clocks,
            &mut peripherals.MCLK,
            &mut delay,
        );

        if let Some(wifi) = WIFI.as_mut() {
            wifi.enable(cs, nvic);
        }
    });

    // ヘッダーピン
    let headerpins = wio::HeaderPins{
        a0_d0:/*A0D0Reset*/ pins.a0_d0,
        a1_d1:/*A1D1Reset*/ pins.a1_d1,
        a2_d2:/*A2D2Reset*/ pins.a2_d2,
        a3_d3:/*A3D3Reset*/ pins.a3_d3,
        a4_d4:/*A4D4Reset*/ pins.a4_d4,
        a5_d5:/*A5D5Reset*/ pins.a5_d5,
        a6_d6:/*A6D6Reset*/ pins.a6_d6,
        a7_d7:/*A7D7Reset*/ pins.a7_d7,
        a8_d8:/*A8D8Reset*/ pins.a8_d8,
    };

    // ここまで 初期化

    // 組込みはloop必須
    loop {
        // LED点滅
        user_led.toggle().ok();
        delay.delay_ms(200_u16);
    }
    // ここまでloop処理
}
// ここまでmain関数
