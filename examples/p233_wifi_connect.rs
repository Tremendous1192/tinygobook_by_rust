//! 組込みRustのおまじない
#![no_std] // 必須アトリビュート
#![no_main] // 必須アトリビュート
use panic_halt as _; // 必須クレート
use wio::prelude::*; // ほぼ必須
use wio_terminal as wio; // 必須クレート

// 文字列操作
use core::fmt::Write;
use heapless::String;

// データの送受信
use cortex_m::interrupt::free as disable_interrupts;

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

// SDカードに保存されているファイル名を画面に表示する
fn read_sentences(
    cont: &mut SDCardController<Clock>,
    file_name: &str,
    lcd: &mut wio::LCD,
) -> heapless::String<256_usize> {
    let mut result = String::<256_usize>::new();

    match cont.device().init() {
        // SDカードと通信できている場合
        Ok(_) => {
            // 通信速度を設定する
            cont.set_baud(20.MHz());

            // ルートディレクトリに移動する
            let mut volume: embedded_sdmmc::Volume = cont.get_volume(VolumeIdx(0)).unwrap();
            let dir = cont.open_root_dir(&volume).unwrap();

            // ファイルを開く
            let mut my_file = cont
                .open_file_in_dir(&mut volume, &dir, file_name, embedded_sdmmc::Mode::ReadOnly)
                .unwrap();

            // ファイル内のデータを読み込む
            while !my_file.eof() {
                let mut buffer = [0u8; 128];
                let num_read = cont.read(&volume, &mut my_file, &mut buffer).unwrap();
                for b in &buffer[0..num_read] {
                    write!(result, "{}", *b as char).unwrap();
                }
            }

            // ルートディレクトリの所有権を開放する
            cont.close_file(&volume, my_file).unwrap();
            cont.close_dir(&volume, dir);
        }
        Err(e) => {
            // 通信失敗のエラーメッセージを表示する
            let mut data = String::<128_usize>::new();
            writeln!(data, "Error!: {:?}", e).unwrap();
            draw_text!(data.as_str(), 4, 2, lcd);
        }
    }

    result
}

// Wi-Fi
use wio::hal::clock::GenericClockController;
use wio::hal::delay::Delay;
use wio::wifi_prelude::*;
use wio::wifi_rpcs as rpc;
use wio::wifi_singleton;
use wio::wifi_types::Security;
wifi_singleton!(WIFI);

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
    let (mut sd_controller, _sd_present) = wio::SDCard {
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
    // ここまで 初期化

    // 読み取りまでの待ち時間(?)
    delay.delay_ms(1_000_u16);

    // Wi-Fi接続のためのSSID読み込みテスト
    let file_name = "SSID.TXT";
    let network_name = read_sentences(&mut sd_controller, &file_name, &mut display);
    let mut textbuffer = String::<256_usize>::new();
    writeln!(textbuffer, "NETWORK_NAME = {}", network_name.as_str()).unwrap();
    draw_text!(textbuffer.as_str(), 4, 2, &mut display);

    // パスワード
    let file_name = "PASSW.TXT";
    let password = read_sentences(&mut sd_controller, &file_name, &mut display);
    let mut textbuffer = String::<256_usize>::new();
    writeln!(textbuffer, "NETWORK_NAME = {}", password.as_str()).unwrap();
    draw_text!(textbuffer.as_str(), 4, 2 + 15, &mut display);

    // バージョン番号を表示する
    let version = unsafe {
        WIFI.as_mut()
            .map(|wifi| wifi.blocking_rpc(rpc::GetVersion {}).unwrap())
            .unwrap()
    };
    let mut textbuffer = String::<256_usize>::new();
    writeln!(textbuffer, "Version = {}", version.as_str()).unwrap();
    draw_text!(textbuffer.as_str(), 4, 2 + 15 * 2, &mut display);

    // mac 番号を表示する
    let mac = unsafe {
        WIFI.as_mut()
            .map(|wifi| wifi.blocking_rpc(rpc::GetMacAddress {}).unwrap())
            .unwrap()
    };
    let mut textbuffer = String::<256_usize>::new();
    writeln!(textbuffer, "MAC = {}", mac.as_str()).unwrap();
    draw_text!(textbuffer.as_str(), 4, 2 + 15 * 3, &mut display);

    // Wi-Fi ルーターに接続する
    let ip_info = unsafe {
        WIFI.as_mut()
            .map(|wifi| {
                wifi.connect_to_ap(
                    &mut delay,
                    network_name.as_str(),
                    password.as_str(),
                    Security::WPA2_SECURITY | Security::AES_ENABLED,
                )
                .unwrap()
            })
            .unwrap()
    };
    draw_text!("Connected !!", 4, 2 + 15 * 4, &mut display);

    // Wi-Fi接続の情報を画面に表示する
    let mut textbuffer = String::<256_usize>::new();
    writeln!(textbuffer, "ip = {}", ip_info.ip).unwrap();
    draw_text!(textbuffer.as_str(), 4, 2 + 15 * 5, &mut display);

    let mut textbuffer = String::<256_usize>::new();
    writeln!(textbuffer, "netmask = {}", ip_info.netmask).unwrap();
    draw_text!(textbuffer.as_str(), 4, 2 + 15 * 6, &mut display);

    let mut textbuffer = String::<256_usize>::new();
    writeln!(textbuffer, "gateway = {}", ip_info.gateway).unwrap();
    draw_text!(textbuffer.as_str(), 4, 2 + 15 * 7, &mut display);

    // 組込みはloop必須
    loop {}
    // ここまでloop処理
}
// ここまでmain関数
