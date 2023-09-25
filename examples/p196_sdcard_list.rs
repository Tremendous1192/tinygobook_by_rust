//! 組込みRustのおまじない
#![no_std] // 必須アトリビュート
#![no_main] // 必須アトリビュート
use panic_halt as _; // 必須クレート
use wio::prelude::*; // ほぼ必須
use wio_terminal as wio; // 必須クレート

// 文字列操作
use core::fmt::Write;
use heapless::String;

// 描画
use eg::mono_font::{ascii::FONT_9X15, MonoTextStyle};
use eg::pixelcolor::Rgb565;
use eg::prelude::*;
use eg::text::{Baseline, Text};
use embedded_graphics as eg;

// 文字列を画面に表示する
macro_rules! draw_text {
    ($text_str:expr, $x:expr, $y:expr, $display:expr) => {
        Text::with_baseline(
            $text_str,
            Point::new($x, $y),
            MonoTextStyle::new(&FONT_9X15, Rgb565::WHITE),
            Baseline::Top,
        )
        .draw($display)
        .ok()
        .unwrap();
    };
}

// SD カード
use embedded_sdmmc::{TimeSource, Timestamp, VolumeIdx};
use wio::SDCardController;
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
    // ここまで 初期化

    // 読み取りまでの待ち時間(?)
    delay.delay_ms(1000_u16);

    // SDカード内のファイル名を画面に表示する
    let style = MonoTextStyle::new(&FONT_9X15, Rgb565::WHITE);
    // SDカードと通信する
    match cont.device().init() {
        Ok(_) => {
            // 通信速度を設定する
            cont.set_baud(20.MHz());

            // SDカードとの通信に成功したことと、SDカードの容量を画面に表示する
            let mut data = String::<128_usize>::new();
            write!(data, "OK! ").unwrap();
            match cont.device().card_size_bytes() {
                Ok(size) => writeln!(data, "{}Mb", size / 1024 / 1024).unwrap(),
                Err(e) => writeln!(data, "Err: {:?}", e).unwrap(),
            }
            draw_text!(data.as_str(), 4, 2, &mut display);

            // SDカードに保存されているファイル名を画面に表示する関数を実行する
            if let Err(e) = print_contents(&mut cont, &mut display) {
                let mut data = String::<128_usize>::new();
                writeln!(data, "Err: {:?}", e).unwrap();
                draw_text!(data.as_str(), 4, 20, &mut display);
            }
        }
        Err(e) => {
            // 通信失敗のエラーメッセージを表示する
            let mut data = String::<128_usize>::new();
            writeln!(data, "Error!: {:?}", e).unwrap();
            draw_text!(data.as_str(), 4, 2, &mut display);
        }
    }

    // 組込みはloop必須
    loop {}
    // ここまでloop処理
}
// ここまでmain関数

// SDカードに保存されているファイル名を画面に表示する
fn print_contents(
    cont: &mut SDCardController<Clock>,
    lcd: &mut wio::LCD,
) -> Result<(), embedded_sdmmc::Error<embedded_sdmmc::SdMmcError>> {
    // ルートディレクトリに移動する
    let volume: embedded_sdmmc::Volume = cont.get_volume(VolumeIdx(0)).unwrap();
    let dir = cont.open_root_dir(&volume).unwrap();

    // 序数(ファイル数を計数する)
    let mut count = 0;

    // 戻り値
    let out = cont.iterate_dir(&volume, &dir, |ent| {
        // ファイル名を取得する
        let mut data = String::<128_usize>::new();
        writeln!(data, "{} - {:?}", ent.name, ent.attributes).unwrap();

        // ファイル名を画面に表示する
        draw_text!(data.as_str(), 4, 20 + count * 16, lcd);

        count += 1;
    });

    // フォルダの所有権を手放す
    cont.close_dir(&volume, dir);

    out
}
