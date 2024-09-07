#include <stdint.h>
#include <stdlib.h>

#include <avr/io.h>
#include <avr/interrupt.h>

#define _BV(bit) (1 << (bit))

#define UART_SEND_ENABLE()  ( UCSR0B |=  _BV(UDRIE0) )
#define UART_SEND_DISABLE() ( UCSR0B &= ~_BV(UDRIE0) )

#define FIFO_BIT_SIZE 8
#define FIFO_DATA_SIZE 256
#define FIFO_BIT_MASK ((1 << FIFO_BIT_SIZE) - 1)

typedef struct ring_FIFO_t {
    uint8_t index_write;
    uint8_t index_read;
    uint8_t buff[FIFO_DATA_SIZE];
} FIFO;

volatile FIFO send_fifo;

void fifo_init(void) {
    send_fifo.index_write = 0;
    send_fifo.index_read = 0;
}

void uart_init(uint32_t f_cpu, uint32_t baud) {
    // USART initialize
    UBRR0 = (uint16_t)(f_cpu / baud / 16) - 1;
    // 8bit non-parity stop1bit
    UCSR0C = (3 << UCSZ00);
    UCSR0B = (1 << TXEN0);

    fifo_init();
}

// FIFO にデータを入れる
// 溢れようがノーチェック
inline void fifo_enqueue(uint8_t data) {
    send_fifo.buff[send_fifo.index_write++] = data;
    send_fifo.index_write &= FIFO_BIT_MASK;
}

// FIFO のデータを出す
// 積まれてなくてもノーチェック
inline uint8_t fifo_dequeue(void) {
    uint8_t ret = send_fifo.buff[send_fifo.index_read++];
    send_fifo.index_read &= FIFO_BIT_MASK;
    return ret;
}

// FIFO に積まれているデータは 0 か？
inline uint8_t is_fifo_empty(void) {
    return (send_fifo.index_write == send_fifo.index_read);
}

// 1 Byte
void uart_send_data(const char data) {
    // バッファの空き待ち
    while (((send_fifo.index_write + 1) & FIFO_BIT_MASK) == send_fifo.index_read);
    UART_SEND_DISABLE();
    
    // バッファに積む
    fifo_enqueue(data);
    
    // データレジスタ空き割り込み許可で連続送信開始
    UART_SEND_ENABLE();
}

void uart_send_str(const char * str) {
    while (*str != '\0') {
        uart_send_data(*str++);
    }
}

void uart_send_int(const int value) {
    char str_buf[10];
    uart_send_str(itoa(value, str_buf, 10));
}

ISR(USART_UDRE_vect) {
    UDR0 = fifo_dequeue();

    // 送信したいデータがなければ
    if (is_fifo_empty()) {
        // データレジスタ空き割り込み解除
        UART_SEND_DISABLE();
    }
}
