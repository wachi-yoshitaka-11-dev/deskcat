//! `DISP-01`（MSP2807、controller ILI9341）のSPI driver。
//!
//! [Issue #13](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/13) の受け入れ条件
//! （controller識別・単色fill・color order・四隅とorientation・更新timing）に対応する。
//! 実機での結果は`docs/hardware/experiment-log.md`の`EXP-016`〜`EXP-018`にある。
//!
//! # 配線の根拠
//!
//! pin割り当ては [`docs/hardware/gpio-assignment.md`](../../../docs/hardware/gpio-assignment.md)
//! の`信号inventory`が正本である。ここでは`LCD-SCLK`(GPIO18)／`LCD-MOSI`(GPIO23)／
//! `LCD-MISO`(GPIO19)／`LCD-CS`(GPIO22)／`LCD-DC`(GPIO17)／`LCD-RST`(GPIO16)の6本だけを
//! 扱う。`TOUCH-CS`(GPIO21)は同一SPI busを共有するが、touch driverは別Issue
//! （[#15](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/15)）の範囲であり、
//! このmoduleは触れない。`LCD-BL`(GPIO4)はbacklightの点灯／消灯だけを扱う
//! （PWM調光は将来検討、`信号inventory`の同行）。
//!
//! # commandとtimingの根拠
//!
//! 一次資料は**ILI9341 Datasheet V1.11**
//! （<https://cdn-shop.adafruit.com/datasheets/ILI9341.pdf>、
//! sha256 `a9bbfdf6d078f54a6aca7a56cba91246905358d3a4ed738817bfd3f582b5741c`、2026-09-17取得。
//! 2026-09-28に取り直して同じsha256であることを確かめた。以前はここに末尾の`c`が抜けた
//! 63文字を書いていた。
//! `docs/hardware/gpio-assignment.md`が§18.2.1・§12.1／§12.2を既に引用しているものと同一文書）。
//!
//! - Software Reset (01h、§8.2.2 p.90)。**書込み後5msec待つ**（同節`Restriction`）。
//! - Sleep Out (11h、§8.2.12 p.101)。**発行後120msecでSleep Out状態になる**
//!   （同節flow chart「It takes 120msec to become Sleep Out mode after SLPOUT command issued.」）。
//! - Hardware reset（`RESX`）: パルス幅`tRW`最小10µs、reset cancel`tRT`最小5ms
//!   （Sleep In状態からの解除。§15.4 Reset Timing、p.229）。
//! - Memory Access Control (36h、MADCTL、§8.2.29 p.127)。Reset時default `00h`
//!   （MY=MX=MV=ML=BGR=MH=0）。bit配置はparameter byteの`D7..D0`が
//!   `MY MX MV ML BGR MH 0 0`（同節のbit表）である。**このfirmwareは`0x28`
//!   （`MV`=`D5`と`BGR`=`D3`だけを立てる）で初期化する**（[`MADCTL_LANDSCAPE`]）。
//!   - `BGR=1`: reset時default（`BGR=0`）では赤と青が入れ替わって表示された（`EXP-016`）。
//!     `BGR=1`で入れ替わりが解消した（`EXP-017`の目視、`EXP-018`の写真）。
//!   - `MV=1`、`MY=0`、`MX=0`: 横向き（320×240）で、論理座標の原点が左上、`x`が右、`y`が
//!     下へ増える。§9.3 MCU to memory write/read direction（p.208）の表と、`EXP-018`の
//!     写真（`MV=0`で描いた生の座標の向きから決め、`0x28`で四隅が一致した）による。
//!     実際の物理orientationはmoduleの現物に依存し、一次資料だけからは決まらない
//!     （[AGENTS.md](../../../AGENTS.md) 推測禁止）。
//!   - **判定の向きは「横向き、14 pinの`J2`のheaderを右の辺にして見た向き」である**
//!     （[#13の判断(2)](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/13#issuecomment-5860773100)）。
//!     [#34](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/34)の仮筐体で
//!     頭へのLCDの載せ方が決まったら見直す。向きを変えるときは[`MADCTL_LANDSCAPE`]と
//!     [`WIDTH`]／[`HEIGHT`]だけを変える。
//! - Pixel Format Set (3Ah、COLMOD、§8.2.33 p.134)。`DPI`/`DBI` = `101`/`101`で16bit/pixel
//!   （byte値`0x55`）。
//! - **このdriverはcontrollerのID（Read ID4＝D3h、Read ID1〜3＝DAh〜DCh）を読まない。**
//!   D3hは`EXP-016`〜`EXP-018`で、DAh〜DChは`EXP-018`で、期待する値を返さなかった
//!   （`EXP-018`ではすべて`00`で、値の範囲が80h〜FFhと決まっているID2も範囲外）。読み出しのdataが
//!   ESP32の`LCD-MISO`へ届いていない見込みがある。原因の候補は`EXP-018`にあり、特定していない。
//!   その候補の一つとして、§7.1.10（p.38）の4-wire serialの図は、Interface Iでは
//!   読み出しdataを`SDA`（この配線では`MOSI`の線）へ出すと描いており、その場合は読み出しの
//!   たびにcontrollerとESP32が同じ線を駆動しうる。moduleをどちらに設定しているかは、
//!   moduleを改変しない方法では確かめられない。**読んでも識別に使えず、線の衝突の
//!   おそれだけが残るため、読み出しのcodeを削除した。**読み出しのcodeは commit `213a107`
//!   に残る。controllerの識別は、moduleのsilkと資料、およびこの節が挙げるcommandが実機で
//!   期待どおりに効いたこと（`EXP-018`）で記録する。
//! - Display Serial Interface Timing（4-line SPI system、§18.3.4 p.242）。
//!   `twc`（write clock cycle）min 100ns → 最大10 MHz、`trc`（read clock cycle）min 150ns
//!   → 最大約6.67 MHz。**このfirmwareは読み書き共通で1本のSPI clockを使うため、
//!   厳しい方（read）を基準に余裕を持たせた値を`SPI_CLOCK_HZ`に採る**（下記）。
//!   `tas`/`tah`（D/CX setup/hold）は最小10nsであり、GPIO制御とSPI呼び出しの間に
//!   自然に生じる遅延で満たす（同節）。
//! - SPI mode: `SDA`/`SDI`はSCL立ち上がりで取り込まれ、`SDO`はSCL立ち下がりで出力される
//!   （§4 Pin Descriptions、`Interface Logic Signals`表、p.10。
//!   「The data is applied on the rising edge of the SCL signal.」／
//!   「The data is outputted on the falling edge of the SCL signal.」）。
//!   これは`CPOL=0, CPHA=0`（Mode 0）に一致する。
//!   `docs/hardware/gpio-assignment.md`の`LCD-SCLK`行が「要確認」としていたSPI modeを
//!   Mode 0で確定する根拠はここにある。

// LCDを使わない`bringup-i2c-15-16`などの試験buildでも、このmoduleをcompileする。
// `main()`から呼ばれない試験buildではdead_codeになるが、cross-compileを続けるため
// moduleごと`#[cfg]`で落とさない。製品buildではLCDを使う。
#![allow(dead_code)]

use esp_idf_svc::hal::delay::{Ets, FreeRtos};
use esp_idf_svc::hal::gpio::{InputPin, Level, Output, OutputPin, PinDriver};
use esp_idf_svc::hal::spi::config::{Config as SpiConfig, DriverConfig, MODE_0};
use esp_idf_svc::hal::spi::{SpiAnyPins, SpiBusDriver, SpiDriver};
use esp_idf_svc::hal::units::Hertz;
use esp_idf_svc::sys::{EspError, ESP_ERR_INVALID_ARG};

/// SPI clockの上限。read timing（`trc`最小150ns→最大約6.67 MHz）を基準に、
/// write（`twc`最小100ns→最大10 MHz）にも共通で使えるよう余裕を持たせた値。
/// **測定や一般値ではなく、上記一次資料の上限から導いた値である。**
/// 今は読み出しをしない（module doc）が、`LCD-MISO`はbusに残るため値は変えない。
const SPI_CLOCK_HZ: u32 = 6_000_000;

const CMD_SWRESET: u8 = 0x01;
const CMD_SLPOUT: u8 = 0x11;
const CMD_DISPON: u8 = 0x29;
const CMD_CASET: u8 = 0x2A;
const CMD_PASET: u8 = 0x2B;
const CMD_RAMWR: u8 = 0x2C;
const CMD_MADCTL: u8 = 0x36;
const CMD_COLMOD: u8 = 0x3A;

/// Reset時のMADCTL既定値（`00h`）。**一次資料の`Default Value`欄そのものであり、
/// このfirmwareが独自に選んだ値ではない。**module doc参照。
/// **このfirmwareは初期化時にこの値を書き込まない（下記[`MADCTL_LANDSCAPE`]を書き込む）。**
/// IC自体のreset時defaultを示す値として残す。
const MADCTL_RESET_DEFAULT: u8 = 0x00;

/// 初期化で書き込むMADCTL値。`MY=0 MX=0 MV=1 ML=0 BGR=1 MH=0`（`0x28`）。
///
/// bitの意味はILI9341 Datasheet V1.11 §8.2.29 p.127のbit表、`MV`・`MY`・`MX`と
/// CASET／PASETの行き先は§9.3 p.208の表による。値は`docs/hardware/experiment-log.md`の
/// `EXP-018`の写真で決め、この値で横向き（`J2`を右）の四隅が一致することを確かめた。
/// **向きの値はここと[`WIDTH`]／[`HEIGHT`]だけに置く**（開発ガイド§9.5「LCDの寸法と向き」）。
pub const MADCTL_LANDSCAPE: u8 = 0x28;

/// 16 bit/pixel（RGB565相当）。`DPI[2:0]=101, DBI[2:0]=101`（module doc参照）。
const COLMOD_16BPP: u8 = 0x55;

/// 論理座標での画面の幅と高さ（[`MADCTL_LANDSCAPE`]の横向き）。
///
/// Panelは`hardware-bom.md` DISP-01行、現物silk `2.8 TFT SPI 240X320 V1.2`のとおり
/// 物理的に240列×320行である。`MV=1`ではCASET（`x`）が物理の行（0〜319）へ、
/// PASET（`y`）が物理の列（0〜239）へ向かう（§9.3 p.208）ため、幅320・高さ240になる。
/// **[`MADCTL_LANDSCAPE`]の`MV`を変えるときは、ここも入れ替える。**
pub const WIDTH: u16 = 320;
pub const HEIGHT: u16 = 240;

/// RGB565の代表色。**一般値ではなく、色空間の定義そのもの**
/// （赤・緑・青・白の各chをfull scaleにしただけ）。
pub mod color {
    pub const BLACK: u16 = 0x0000;
    pub const WHITE: u16 = 0xFFFF;
    pub const RED: u16 = 0xF800;
    pub const GREEN: u16 = 0x07E0;
    pub const BLUE: u16 = 0x001F;
}

/// `DISP-01`のILI9341 driver。
///
/// **CSは常にこのdriverが自前のGPIOで管理する**（`SpiBusDriver`はhardware CSを
/// 持たない。moduleのCS制御table「Managed CS: N」の構成）。ILI9341の各commandは
/// 「command byte → (D/CXをHighへ) → parameter byte列」を**1回のCSX assertionの中で**
/// 送る設計であり（datasheetの各command図がS...Pの1区間で示す）、途中でCSを
/// 上げ下げしない。
pub struct Ili9341<'d> {
    bus: SpiBusDriver<'d, SpiDriver<'d>>,
    cs: PinDriver<'d, Output>,
    dc: PinDriver<'d, Output>,
    rst: PinDriver<'d, Output>,
    bl: PinDriver<'d, Output>,
}

impl<'d> Ili9341<'d> {
    /// SPI busとGPIOを結線し、driverを作る。**まだ初期化commandは送らない。**
    #[allow(clippy::too_many_arguments)]
    pub fn new<SPI: SpiAnyPins + 'd>(
        spi: SPI,
        sclk: impl OutputPin + 'd,
        mosi: impl OutputPin + 'd,
        miso: impl InputPin + 'd,
        cs: impl OutputPin + 'd,
        dc: impl OutputPin + 'd,
        rst: impl OutputPin + 'd,
        bl: impl OutputPin + 'd,
    ) -> Result<Self, EspError> {
        let driver = SpiDriver::new(spi, sclk, mosi, Some(miso), &DriverConfig::new())?;
        let bus = SpiBusDriver::new(
            driver,
            &SpiConfig::new()
                .baudrate(Hertz(SPI_CLOCK_HZ))
                .data_mode(MODE_0),
        )?;

        let mut cs = PinDriver::output(cs)?;
        let mut dc = PinDriver::output(dc)?;
        let mut rst = PinDriver::output(rst)?;
        let mut bl = PinDriver::output(bl)?;

        // Idle状態。CSはactive-lowのためHighでinactive。RSTはHighで通常動作。
        // BLはactive-high（`sensor-datasheet-notes.md`の`Backlight回路／電流／polarity`行）
        // であり、初期化完了までLowに保つ（`gpio-assignment.md`の`LCD-BL`行と同じ理由）。
        cs.set_high()?;
        dc.set_low()?;
        rst.set_high()?;
        bl.set_low()?;

        Ok(Self {
            bus,
            cs,
            dc,
            rst,
            bl,
        })
    }

    /// Hardware reset。`tRW`（reset pulse）最小10µs、`tRT`（reset cancel、Sleep In時）
    /// 最小5msに余裕を持たせた値を使う（`module doc`の`§15.4`引用）。
    fn hardware_reset(&mut self) -> Result<(), EspError> {
        self.rst.set_high()?;
        Ets::delay_us(20);
        self.rst.set_low()?;
        Ets::delay_us(50);
        self.rst.set_high()?;
        FreeRtos::delay_ms(10);
        Ok(())
    }

    /// commandを送り、CSXをlowにする。`body`をその中で実行し、`body`の成否に
    /// 関わらずCSXを解放する。
    ///
    /// CodeRabbit review（[#415](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/415)）
    /// の指摘: 以前は`command()`／`read_id()`／`fill_rect()`がそれぞれ独立に
    /// `?`でSPIエラーを伝播しており、途中の`self.bus.write`／`self.bus.read`が
    /// 失敗するとCSXがLow（assert状態）のまま関数を抜けていた。次のcommandは
    /// `cs.set_low()`を呼ぶが既にLowであり、controllerからは前のtransactionの
    /// 続きに見えかねない。**この関数はCSXの解放をbodyの結果と切り離して保証する。**
    /// `body`が失敗した場合はそのerrorを返す（CSX解放自体が別途失敗しても、
    /// 根本原因である`body`側のerrorを優先し、解放側のerrorは`let _`で無視する）。
    /// `body`が成功した場合に限り、CSX解放（`cs.set_high()`）の結果をそのまま返す。
    fn with_transaction(
        &mut self,
        cmd: u8,
        body: impl FnOnce(&mut Self) -> Result<(), EspError>,
    ) -> Result<(), EspError> {
        self.cs.set_low()?;
        let result = self.write_command_byte(cmd).and_then(|()| body(self));
        match result {
            Ok(()) => self.cs.set_high(),
            Err(err) => {
                let _ = self.cs.set_high();
                Err(err)
            }
        }
    }

    /// command byteを送り、D/CXをHighへ戻す。`with_transaction`の内側専用。
    fn write_command_byte(&mut self, cmd: u8) -> Result<(), EspError> {
        self.dc.set_low()?;
        self.bus.write(&[cmd])?;
        self.dc.set_high()
    }

    /// commandを送り、parameterを続けて書く。**1回のCSX assertion内で完結する。**
    fn command(&mut self, cmd: u8, params: &[u8]) -> Result<(), EspError> {
        self.with_transaction(cmd, |this| {
            if params.is_empty() {
                Ok(())
            } else {
                this.bus.write(params)
            }
        })
    }

    /// 初期化sequence。**hardware reset→software reset→sleep out→pixel
    /// format→MADCTL（[`MADCTL_LANDSCAPE`]）→display on、の順で一次資料の待ち時間を守る。**
    /// controllerのIDは読まない（module doc）。
    pub fn init(&mut self) -> Result<(), EspError> {
        self.hardware_reset()?;

        self.command(CMD_SWRESET, &[])?;
        FreeRtos::delay_ms(5);

        self.command(CMD_SLPOUT, &[])?;
        FreeRtos::delay_ms(120);

        self.command(CMD_COLMOD, &[COLMOD_16BPP])?;
        self.command(CMD_MADCTL, &[MADCTL_LANDSCAPE])?;
        self.command(CMD_DISPON, &[])?;

        Ok(())
    }

    /// Backlightを点灯する。**controller初期化（`init`）が完了した後に呼ぶこと。**
    pub fn backlight_on(&mut self) -> Result<(), EspError> {
        self.bl.set_level(Level::High)
    }

    /// 描画windowを設定する（CASET／PASET）。座標は両端を含む（inclusive）。
    fn set_window(&mut self, x0: u16, y0: u16, x1: u16, y1: u16) -> Result<(), EspError> {
        self.command(
            CMD_CASET,
            &[(x0 >> 8) as u8, x0 as u8, (x1 >> 8) as u8, x1 as u8],
        )?;
        self.command(
            CMD_PASET,
            &[(y0 >> 8) as u8, y0 as u8, (y1 >> 8) as u8, y1 as u8],
        )
    }

    /// 矩形を単色で塗る。**RAMWRの間はCSXを一度も上げない**（1回の描画を1つの
    /// SPI transactionとして扱う。datasheetのcommand図と同じ形）。
    ///
    /// # Errors
    ///
    /// `x1 < x0`、`y1 < y0`、`x1 >= WIDTH`、`y1 >= HEIGHT`のいずれかを満たす場合、
    /// SPIへは何も送らず`ESP_ERR_INVALID_ARG`を返す。CodeRabbit review
    /// （[#415](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/415)）の指摘:
    /// 以前は`debug_assert!`のみで、release buildでは前提を検査しなかった
    /// （`overflow-checks`既定offのため、満たさない場合`x1 - x0 + 1`等が無音でwrapし、
    /// 無関係なpixel数とCASET/PASET値を送りかねなかった）。CASET/PASETの一次資料の
    /// 制約「SC[15:0] always must be equal to or less than EC[15:0]」と同じ前提を、
    /// ここで実行時に検証する。
    pub fn fill_rect(
        &mut self,
        x0: u16,
        y0: u16,
        x1: u16,
        y1: u16,
        color: u16,
    ) -> Result<(), EspError> {
        if x1 < x0 || y1 < y0 || x1 >= WIDTH || y1 >= HEIGHT {
            return Err(EspError::from_infallible::<{ ESP_ERR_INVALID_ARG }>());
        }
        self.set_window(x0, y0, x1, y1)?;

        let pixel_count = u32::from(x1 - x0 + 1) * u32::from(y1 - y0 + 1);
        let [hi, lo] = color.to_be_bytes();

        // 16bit/pixel(RGB565)をbig-endianで送る（module doc `16bpp Frame Memory Write`表）。
        // chunkにまとめてbus呼び出し回数を抑える。**heap allocationはしない**
        // （ISR文脈ではないが、firmwareのallocation方針を`main`側の関数と揃える）。
        const CHUNK_PIXELS: usize = 64;
        let mut chunk = [0u8; CHUNK_PIXELS * 2];
        for pair in chunk.chunks_exact_mut(2) {
            pair[0] = hi;
            pair[1] = lo;
        }

        self.with_transaction(CMD_RAMWR, |this| {
            let mut remaining = pixel_count;
            while remaining > 0 {
                let this_chunk = remaining.min(CHUNK_PIXELS as u32) as usize;
                this.bus.write(&chunk[..this_chunk * 2])?;
                remaining -= this_chunk as u32;
            }
            Ok(())
        })
    }

    /// 画面全体を単色で塗る。
    pub fn fill_screen(&mut self, color: u16) -> Result<(), EspError> {
        self.fill_rect(0, 0, WIDTH - 1, HEIGHT - 1, color)
    }
}
