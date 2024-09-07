#ifndef __AVR_UART_H__
#define __AVR_UART_H__

extern void uart_init(uint32_t f_cpu, uint32_t baud);

extern void uart_send_data(const char data);
extern void uart_send_str(const char * str);
extern void uart_send_int(const int * value);

#endif
