
/* 
## 参考サイト
https://tomoyuki-nakabayashi.github.io/embedded-rust-techniques/03-bare-metal/print.html
 */

// 自作
use crate::my_queue;
pub use my_queue::RingBufferIndex8bit;
const SIZE: usize = 256;
static mut BUFFER: RingBufferIndex8bit<u8, SIZE> = RingBufferIndex8bit::new(SIZE);

// UART 初期化関数
pub fn init(f_cpu: u32, baud: u32) {
    let dp = unsafe {avr_device::atmega328p::Peripherals::steal()};

    // USART 初期化
    let ubrr = (f_cpu / baud / 16) - 1;
    dp.USART0.ubrr0.write(|w| w.bits(ubrr as u16));

    // 8ビット非パリティ・ストップビット1
    dp.USART0.ucsr0c.write(|w| w.ucsz0().chr8());
    dp.USART0.ucsr0b.write(|w| w.txen0().set_bit());
}

// 1バイトのデータを送信する関数
pub fn send_data(data: u8) {
    let dp = unsafe { avr_device::atmega328p::Peripherals::steal() };
    
    // バッファの空き待ち
    unsafe {
        while BUFFER.is_full() {
            avr_device::asm::nop();
        }
        BUFFER.enqueue(data);
    }
    
    // 割り込み 有効
    dp.USART0.ucsr0b.modify(|_, w| w.udrie0().set_bit());
}

// 文字列を送信する関数
pub fn send_str(s: &str) {
    for &byte in s.as_bytes() {
        send_data(byte);
    }
}

// 改行コード付きで文字列を送信する関数
pub fn send_str_line(s: &str) {
    send_str(&s);
    send_str("\n");
}

// 割り込みハンドラ
#[avr_device::interrupt(atmega328p)]
fn USART_UDRE() {
    let dp = unsafe {avr_device::atmega328p::Peripherals::steal()};

    unsafe {
        let data = BUFFER.dequeue();
        dp.USART0.udr0.write(|w| w.bits(data));
        
        if BUFFER.is_empty() {
            dp.USART0.ucsr0b.modify(|_, w| w.udrie0().clear_bit());
        }
    }
}
