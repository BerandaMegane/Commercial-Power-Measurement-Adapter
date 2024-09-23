#![no_std]
#![no_main]
#![feature(abi_avr_interrupt)]

// UART 自作
mod uart_interrupt;

// 動作クロック周波数
type CoreClock = atmega_hal::clock::MHz20;
// タイマ割り込み周波数
const TIMER1_FREQ: u32 = 1000;
// UART 通信速度
const UART_BAUDRATE: u32 = 19200;
// AD変換周期クロック
const ADC_CLOCK: f32 = 13.0;
const ADC_DIV: f32 = 128.0;
const ADC_FREQ: f32 = CoreClock::FREQ as f32 / ADC_CLOCK / ADC_DIV;

// ディレイ関係
// use atmega_hal::{clock::Clock, pac, prelude::_embedded_hal_blocking_delay_DelayMs, usart::{Baudrate, Usart}, Atmega};
use atmega_hal::clock::Clock;
use embedded_hal::delay::DelayNs;
type Delay = atmega_hal::delay::Delay<crate::CoreClock>;
#[allow(dead_code)]
fn delay_ms(ms: u16) {
    Delay::new().delay_ms(u32::from(ms))
    // embedded_hal::delay::DelayNs::delay_ms(&mut Delay::new(), u32::from(ms))
}
#[allow(dead_code)]
fn delay_us(us: u32) {
    Delay::new().delay_us(us)
}

// 測定範囲 10 - 100 Hz
// 測定中の波形立上り回数  1s ごとに -1 リセット, -1 でないときは測定中
// 測定を開始してから1秒以内にタイマ割り込みによって、測定が終了するはず
// よって、数値は電源周波数以下となるはず
static mut RISED_COUNT: i8 = -1;  // 60Hz なら 60 以下
// 測定中の計数値  1s ごとに 0 リセット
static mut ELAPSED_COUNT: u16 = 0;  // 60Hz: ADC_FREQ / 60 = 200, 10Hz: 1200
// 測定中の計数確定値  1s ごとに 0 リセット
static mut MEASURED_COUNT: u16 = 0;  // 60Hz: RISED * ELAPSED = 12000 (ほぼ ADC_FREQ)
// 1秒計測用カウンタ
static mut TIMER_1S_COUNTER: u16 = 0;  // 0..1000
// 60秒計測用カウンタ
static mut TIMER_60S_COUNTER: u8 = 0;  // 0..60

// 分圧抵抗値
const DAMP_A: f32 = 5.0;
const DAMP_B: f32 = 300.0;
// 分圧抵抗による減衰比 1次側を1としたときの2次側比率
const DAMPLING_RATIO: f32 = DAMP_A / (DAMP_B + DAMP_A);

// 波形取得するサンプル数 >= ADC_FREQ / 40Hz = 300
mod my_queue;
pub use my_queue::RingBufferIndex16bit;
const WAVEFORM_MAX: usize = 512;
static mut WAVEFORM_BUF: RingBufferIndex16bit<u16, WAVEFORM_MAX> = RingBufferIndex16bit::new(WAVEFORM_MAX);

// 波形補正値
static mut WAVE_OFFSET: f32 = 512.0;
// 2乗した電圧値の累計値 (実効値)
static mut ALL_CYCLE_SQUARE_SUM: f32 = 0.0;
// 電圧値の累計値 (平均値)
static mut ALL_CYCLE_AVERAGE: f32 = 0.0;

// メイン関数 エントリポイント
#[atmega_hal::entry]
fn main() -> ! {
    // ペリフェラルの取得
    let dp = unsafe {atmega_hal::Peripherals::steal()};

    // 各種初期化
    port_init();
    timer1_init();
    adc_init();
    adc_start();
    uart_interrupt::init(CoreClock::FREQ, UART_BAUDRATE);
    
    // 全割り込み許可
    unsafe {avr_device::interrupt::enable();}  // sei();

    uart_interrupt::send_str_line("mode,data1,data2");
    
    unsafe {
        let mut buf: [u8; 16] = [0; 16];

        loop {
            // sync 同期
            const CSV2_BEGIN: u8 = 3;
            while TIMER_60S_COUNTER < CSV2_BEGIN {
                avr_device::asm::nop();
            }

            // while ! WAVEFORM_VEC.is_empty() {
            while ! WAVEFORM_BUF.is_empty() {
                // CSV line 2 start
                uart_interrupt::send_str("csv-2");
                
                // index
                uart_interrupt::send_str(",");
                // uart_interrupt::send_str(u32_to_str(WAVEFORM_VEC.len() as u32, &mut buf));
                uart_interrupt::send_str(u32_to_str(WAVEFORM_BUF.len() as u32, &mut buf));
                
                // waveform
                uart_interrupt::send_str(",");
                // uart_interrupt::send_str(u32_to_str(WAVEFORM_VEC.pop().unwrap() as u32, &mut buf));
                uart_interrupt::send_str(u32_to_str(WAVEFORM_BUF.dequeue() as u32, &mut buf));
                
                // 改行
                uart_interrupt::send_str("\n");
        
                delay_ms(20);
            }

            // sync
            while TIMER_60S_COUNTER != 0 {
                avr_device::asm::nop();
            }
        }
    }
}


// タイマ1 比較A 割り込みハンドラ
// note: VSCode の rust-analyzer が赤線を出すため、別関数に分割
#[avr_device::interrupt(atmega328p)]
fn TIMER1_COMPA() {
    timer1_compa_proc();
}

fn timer1_compa_proc() {
    unsafe {
        // 1 sec 周期
        if TIMER_1S_COUNTER < 1000 {
            TIMER_1S_COUNTER += 1;
            return;
        } else {
            TIMER_1S_COUNTER = 0;
        }

        // 60 sec 周期
        if TIMER_60S_COUNTER < 60 {
            TIMER_60S_COUNTER += 1;
        } else {
            TIMER_60S_COUNTER = 0;
        }
    }

    timer_interrupt_1s();
}

fn timer_interrupt_1s() {
    // ペリフェラル取得
    let dp = unsafe {atmega_hal::Peripherals::steal()};

    unsafe {
        // LED 反転
        dp.PORTB.portb.modify(|r, w| w.pb5().bit(! r.pb5().bit()));

        // 周波数測定
        let mut freq: f32 = 0.0;
        if MEASURED_COUNT > 0 {
            freq = RISED_COUNT as f32 * ADC_FREQ / MEASURED_COUNT as f32;
        }
        
        // 電圧測定
        let mut rms: f32 = 0.0;
        if MEASURED_COUNT > 0 {
            let temp_rms = sqrt_newton(ALL_CYCLE_SQUARE_SUM as f32 / MEASURED_COUNT as f32);
            rms = (temp_rms / 1024.0 * 5.0) / DAMPLING_RATIO;
        }

        // 割り込み許可
        avr_device::interrupt::enable();  // sei();
        
        // 出力
        uart_interrupt::send_str("csv-1,");
        let mut buf: [u8; 32] = [0; 32];

        // 周波数
        uart_interrupt::send_str(f32_to_str(freq, 4, &mut buf));
        uart_interrupt::send_str(",");
        // 電圧
        uart_interrupt::send_str(f32_to_str(rms, 3, &mut buf));

        // // rised_count
        // uart_interrupt::send_str(i32_to_str(RISED_COUNT as i32, &mut buf));
        // uart_interrupt::send_str(",");
        // // measured_count
        // uart_interrupt::send_str(u32_to_str(MEASURED_COUNT as u32, &mut buf));
        // uart_interrupt::send_str(",");
        // // WAVE_OFFSET
        // uart_interrupt::send_str(f32_to_str(WAVE_OFFSET, 3, &mut buf));
        // uart_interrupt::send_str(",");

        // // ALL_CYCLE_SQUARE_SUM
        // uart_interrupt::send_str(f32_to_str(ALL_CYCLE_SQUARE_SUM, 1, &mut buf));
        // uart_interrupt::send_str(",");

        // 改行
        uart_interrupt::send_str("\n");

        // オフセット値の算出
        WAVE_OFFSET = WAVE_OFFSET * 0.95 + ALL_CYCLE_AVERAGE * 0.05;

        // 測定リセット、次回の立ち上がりから測定開始
        RISED_COUNT = -1;
    }
}

#[avr_device::interrupt(atmega328p)]
fn ADC() {
    /* ADC 割り込みベクタ
     * 
     * ADC 終了後、実行される割り込み処理
     */

    let dp = unsafe {atmega_hal::Peripherals::steal()};
    
    unsafe {
        // ADC値
        let adc_raw_val = dp.ADC.adc.read().bits();
        // 波形正負極性  true: 正, false: 負
        let threshold = WAVE_OFFSET as u16;
        let current_polarity = adc_raw_val > threshold;
        // 前回計測時の正負極性
        static mut PRE_POLARITY: bool = false;
        // 波形立ち上がりフラグ  1: 立ち上がり
        let is_wave_rising = 
            ! PRE_POLARITY
            && current_polarity 
            && adc_raw_val < (threshold + 25);  // 初回測定誤差を減らすため
        PRE_POLARITY = current_polarity;
        
        // 計数値の処理
        if is_wave_rising {
            // 波形立上りを検出したら計数値リセットし、測定開始
            if RISED_COUNT == -1 {
                RISED_COUNT = 0;
                ELAPSED_COUNT = 0;
                MEASURED_COUNT = 0;
            } else {
                RISED_COUNT += 1;
                MEASURED_COUNT = ELAPSED_COUNT;
            }
        }
        ELAPSED_COUNT += 1;
        
        // 波形取得
        if TIMER_60S_COUNTER == 1 {
            // 60 sec カウンタが 1 のとき (0-indexed)
            
            if RISED_COUNT == 0 && ! WAVEFORM_BUF.is_full() {
                // 1周期目の波形取得
                WAVEFORM_BUF.enqueue(adc_raw_val);
            }
        }
        
        // 電圧の 実効値 and 平均 測定
        // 1周期分の2乗値の総和
        static mut CYCLE_SQUARE_SUM: f32 = 0.0;
        CYCLE_SQUARE_SUM += (adc_raw_val as f32 - WAVE_OFFSET) * (adc_raw_val as f32 - WAVE_OFFSET);
        
        // 1周期分の総和
        static mut CYCLE_VAL_SUM: u32 = 0;
        CYCLE_VAL_SUM += adc_raw_val as u32;

        if is_wave_rising {
            // 波形立ち上がり時
            
            if RISED_COUNT == 0 {
                // 1周期目
                
                // 2乗値の総和
                CYCLE_SQUARE_SUM = 0.0;
                ALL_CYCLE_SQUARE_SUM = 0.0;
                
                // 総和
                CYCLE_VAL_SUM = 0;
            } else {
                // 2周期目以降
                
                // 2乗値の総和
                ALL_CYCLE_SQUARE_SUM += CYCLE_SQUARE_SUM;
                CYCLE_SQUARE_SUM = 0.0;

                ALL_CYCLE_AVERAGE = if MEASURED_COUNT != 0 {
                    CYCLE_VAL_SUM as f32 / MEASURED_COUNT as f32
                } else {
                    512.0
                }
            }
        }
    }
}

// パニックハンドラ
#[cfg(not(doc))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    // 割り込み無効
    avr_device::interrupt::disable();
    let dp = unsafe {atmega_hal::Peripherals::steal()};
    loop {
        const BLINK_HZ: u32 = 10;
        avr_device::asm::delay_cycles(CoreClock::FREQ / BLINK_HZ / 2);
        dp.PORTB.portb.modify(|_, w| w.pb4().set_bit());
        avr_device::asm::delay_cycles(CoreClock::FREQ / BLINK_HZ / 2);
        dp.PORTB.portb.modify(|_, w| w.pb4().clear_bit());
    }
}

// GPIO 初期化
fn port_init() {
    let dp = unsafe {atmega_hal::Peripherals::steal()};
    let pins = atmega_hal::pins!(dp);
    
    /* 
     * Port B
     *   PB0 - 3: No Connect
     *   PB4: Debug (Panic)
     *   PB5: Debug
     *   PB6: No Connect
     */
    pins.pb5.into_output();
    pins.pb4.into_output();
    
    /* 
     * Port C
     *   PC0: ADC0 (AC wave input)
     *   PC1 - 5: No Connect
     */
    pins.pc0.into_floating_input();

    /* 
     * Port D
     *   PD0: No Connect
     *   PD1: TxD (output)
     *   PD2 - 7: No Connect
     */
    pins.pd1.into_output();
}

// Timer1 初期設定
fn timer1_init() {
    let dp = unsafe {atmega_hal::Peripherals::steal()};

    // カウント値の算出
    let ticks = (CoreClock::FREQ / TIMER1_FREQ - 1) as u16;

    // カウント値クリア
    dp.TC1.tcnt1.write(|w| w.bits(0));
    // 比較値
    dp.TC1.ocr1a.write(|w| w.bits(ticks));
    // CTCモード 分周なし 直接カウント
    dp.TC1.tccr1a.write(|w| w.wgm1().bits(0));
    dp.TC1.tccr1b.write(|w| w.wgm1().bits(1));
    // タイマ比較A 割り込み許可
    dp.TC1.timsk1.write(|w| w.ocie1a().set_bit());
    // タイマカウントスタート
    dp.TC1.tccr1b.modify(|_, w| w.cs1().direct());
}


// ADC 初期化関数
fn adc_init() {
    let dp = unsafe {atmega_hal::Peripherals::steal()};

    // 基準電圧 AVCC, 右寄せ(ADLAR = 0), ADC0
    dp.ADC.admux.write(|w| 
        w
        .refs().avcc()  // 基準電圧 AVCC
        .adlar().clear_bit()  // 右寄せ
        .mux().adc0()  // ADC
    );

    // AD許可, AD連続変換, AD変換完了割り込み許可, 128分周
    dp.ADC.adcsra.write(|w| 
        w
        .aden().set_bit()  // AD許可
        .adie().set_bit()  // AD変換完了割り込み許可
        .adate().set_bit()  // AD連続変換
        .adps().prescaler_128()  // 128分周
    );

    dp.ADC.adcsrb.write(|w| 
        w.adts().val_0x00()
    );
}

// ADC 変換開始
fn adc_start() {
    let dp = unsafe {atmega_hal::Peripherals::steal()};
    dp.ADC.adcsra.modify(|_, w| {
        w
        .adsc().set_bit() // AD変換開始
    });
}

// 平方根を求める関数  by ChatGPT
fn sqrt_newton(x: f32) -> f32 {
    if x < 0.0 {
        return f32::NAN; // 負の数の場合は NaN を返す
    }
    
    let mut guess = x / 2.0;
    let mut prev_guess = 0.0;

    // 収束するまで繰り返す
    while abs_f32(guess - prev_guess) > 0.00001 {
        prev_guess = guess;
        guess = (guess + x / guess) / 2.0;
    }

    guess
}


// 浮動小数点数を10進数文字列に変換する関数
fn f32_to_str<'a>(value: f32, after: u8, buf: &'a mut [u8]) -> &'a str {
    // ポインタ
    let mut ptr: u8 = 0;
    
    
    for i in 0..buf.len() {
        buf[i] = 0;
    }
    
    if value < 0.0 {
        buf[ptr as usize] = "-".as_bytes()[0];
        ptr += 1;
    }
    
    let mut n = abs_f32(value);
    
    // 整数部分の変換
    let mut integer = n as u32;
    for _i in 0..buf.len()-1 {
        let r = integer % 10;
        let c: u8 = "0".as_bytes()[0] + r as u8;

        buf[ptr as usize] = c;
        ptr += 1;

        integer /= 10;
        if integer == 0 {
            break;
        }
    }
    reverse(buf, 0, ptr.saturating_sub(1));

    // 小数点
    buf[ptr as usize] = ".".as_bytes()[0];
    ptr += 1;

    // 小数の変換
    let mut decimal: f32 = n % 1.0;
    for _i in 0..after {
        decimal *= 10.0;
        let r = decimal as u8;
        let c: u8 = "0".as_bytes()[0] + r as u8;
        buf[ptr as usize] = c;
        ptr += 1;

        decimal %= 1.0;
    }

    return match byte_array_to_str(buf) {
        Ok(s) => { s }
        Err(_s) => { "" }
    };
}

// 符号なし整数を10進数文字列に変換する関数
#[allow(dead_code)]
fn u32_to_str<'a>(value: u32, buf: &'a mut [u8]) -> &'a str {
    let mut ptr: u8 = 0;
    
    // 初期化
    for i in 0..buf.len() {
        buf[i] = 0;
    }
    // 変換
    let mut n = value;
    for _i in 0..buf.len()-1 {
        let r = n % 10;
        let c: u8 = 0x30 + r as u8;  // 0x30 = '0'

        buf[ptr as usize] = c;
        ptr += 1;

        n /= 10;
        if n == 0 {
            break;
        }
    }
    reverse(buf, 0, ptr-1);
    buf[ptr as usize] = 0x00;

    return match byte_array_to_str(buf) {
        Ok(s) => { s }
        Err(_s) => { "" }
    };
}

// 整数を10進数文字列に変換する関数
#[allow(dead_code)]
fn i32_to_str<'a>(value: i32, buf: &'a mut [u8]) -> &'a str {
    let mut ptr: u8 = 0;
    
    // 初期化
    for i in 0..buf.len() {
        buf[i] = 0;
    }
    
    // 符号
    let is_minus = value < 0;
    if is_minus {
        buf[ptr as usize] = "-".as_bytes()[0];
        ptr += 1;
    }

    // 変換
    let mut n: u32 = if ! is_minus { value as u32 } else { (-value) as u32 };
    for _i in 0..buf.len()-1 {
        let r = n % 10;
        let c: u8 = 0x30 + r as u8;  // 0x30 = '0'

        buf[ptr as usize] = c;
        ptr += 1;

        n /= 10;
        if n == 0 {
            break;
        }
    }
    
    if is_minus {
        reverse(buf, 1, ptr-1);
    } else {
        reverse(buf, 0, ptr-1);
    }
    buf[ptr as usize] = 0x00;

    return match byte_array_to_str(buf) {
        Ok(s) => { s }
        Err(_s) => { "" }
    };
}

pub fn byte_array_to_str(bytes: &[u8]) -> Result<&str, &'static str> {
    // ヌル終端 (0) の位置を探す
    let nul_pos = bytes.iter().position(|&b| b == 0).ok_or("No null terminator found")?;

    // ヌル終端までのバイト列を &str に変換
    core::str::from_utf8(&bytes[..nul_pos]).map_err(|_| "Invalid UTF-8 sequence")
}

fn reverse(buf: &mut [u8], mut i: u8, mut j: u8) {
    while i < j {
        // swap(&mut buf[i], &mut buf[j]);
        buf.swap(i as usize, j as usize);
        i += 1;
        j = j.saturating_sub(1);
    }
}

// 絶対値を求める関数  by ChatGPT
fn abs_f32(x: f32) -> f32 {
    if x > 0.0 {
        return x;
    } else {
        return -x;
    }
}

