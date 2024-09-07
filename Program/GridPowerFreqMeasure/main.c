/*
 * GridPowerFreqMeasure
 * Author : user
 * 
 * # マイコンについて
 * ## MCU
 * AVR ATmega328P
 * 
 * ## Fuse Bit
 * LOW:      0xF7
 * HIGH:     0xD1
 * EXTENDED: 0xFF
 * 
 * ## Frequency
 * XTAL:  20 MHz
 * CPU:   20 MHz
 * 
 * # ソフトウェアについて
 * ## サンプリング
 * ADC連続変換モードを利用してサンプリングしています。
 * サンプリング周波数 f_s = F_CPU / ADC_DIV / ADC_CLOCK = 約 12 kHz
 * 
 * ## 周波数測定
 * 入力信号の周期を測定して周波数に換算するレシプロカル方式で周波数を測定しています。
 * 電源周波数のように低い周波数を測定する場合によく利用されます。
 * ある測定時間のおける入力信号の周波数は、次のように求められます。
 * 
 * 入力信号の周期数 cycles, サンプリング周波数 f_s, サンプリング数 count とすると次のようになります。
 *   入力信号周波数 = cycles / (count / f_s) [Hz]
 * 
 * この方式で周波数測定するときの分解能は、次のように求められます。
 * 
 * 入力信号周期 f_in, サンプリング周期 f_s, 測定時間 T とすると次のようになります。
 *   周波数分解能 = f_in / f_s / T [Hz]
 * 
 * 本プログラムで用いる値を代入し、f_s = 12 [kHz], T = 1 [s] のとき、
 * 次のようになります。
 * 
 *   f_in = 50 [Hz] のとき分解能 = 0.00417 [Hz] (4.17 [mHz])
 *   f_in = 60 [Hz] のとき分解能 = 0.00500 [Hz] (5.00 [mHz])
 * 
 * ## 電圧測定
 * 電圧値は実効値の定義に従い、入力信号1周期分の瞬時値に対して、
 * 2乗 → 総和平均 → 平方根をとることによって求めます。
 * 
 * # 参考資料
 * ## 回路全体
 * http://elm-chan.org/works/lvfm/report_j.html
 * ## レシプロカル方式
 * https://www.keisoku.co.jp/md/support/useful/u_about-aa/u_counter/
 * 
 */

// CPU 動作周波数 (delay.h に必要)
#define F_CPU 20000000UL

#include <stdio.h>
#include <stdint.h>
#include <stdlib.h>
#include <math.h>

#include <avr/io.h>
#include <avr/interrupt.h>
#include <util/delay.h>

// 自作 UART ライブラリ
#include "avr_uart.h"

// GPIO 初期化関数
void port_init(void);

// UART ボーレート周波数
#define UART_BAUD (19200)

// タイマ割り込み周波数
#define TIMER_FREQ (1000)
// タイマ 初期化関数
void timer_init(void);

// AD変換周波数の分周値
#define ADC_DIV (128)
// AD変換周期クロック
#define ADC_CLOCK (13)
// 分圧抵抗による減衰比 1次側を1としたときの2次側比率
#define DAMPLING_RATIO (5.0 / (300.0 + 5.0))

// 波形取得するサンプル数
// WAVEFORM_POINTS >= 最低値,  最低値 = 20MHz / ADC_DIV / ADC_CLOCK / 40Hz = 300
#define WAVEFORM_POINTS (320)
// 波形格納データ型
typedef uint16_t adc_raw_t;
// 波形配列
volatile adc_raw_t waveform[WAVEFORM_POINTS];
// 波形ポイント数
volatile uint16_t waveform_len = 0;

// 電圧2乗値の累計値
volatile uint64_t all_cycle_square_sum = 0;

// 波形補正値
volatile float wave_offset = 512;

volatile float all_cycle_average;

// ADC 初期化関数
void adc_init(void);
// ADC 連続変換開始
void adc_start(void);

// 測定中の波形立上り回数  1s ごとに -1 リセット, -1 でないときは測定中
volatile int8_t   rised_count = -1;
// 測定中の計数値         1s ごとに 0 リセット
volatile uint16_t elapsed_count = 0;
// 測定中の計数確定値     1s ごとに 0 リセット
volatile uint16_t measured_count = 0;

volatile uint8_t  timer_60s_counter = 0;

// 波形CSV形式出力関数
uint8_t waveform_csv(const uint16_t start, const uint8_t count) {
    /*
     * Arguments:
     *   start: 出力添字番号 (0 - *)
     *   count: 出力個数
     * Return:
     *   最後の値まで出力し終えたときは 1
     *   まだ出力できる値があるときは 0 
     */
    uint16_t end = start + count;
    uint16_t i = start;
    
    while (i < end && i < WAVEFORM_POINTS) {
        if (i >= waveform_len) {
            return 1;
        }
        // CSV line 2 start
        uart_send_str("csv-2");

        // index
        uart_send_str(",");
        uart_send_int(i);

        // waveform
        uart_send_str(",");
        uart_send_int(waveform[i]);
        uart_send_str("\n");
        
        i++;
    }
    return 0;
}

void clear_waveform(void) {
    for (int i = 0; i < WAVEFORM_POINTS; i++) {
        waveform[i] = 0;
    }
}

int main(void) {
    // 各種初期化
    port_init();
    timer_init();
    adc_init();
    adc_start();
    uart_init(F_CPU, UART_BAUD);
    
    // 全割り込み有効化
    sei();

    // CSV ヘッダ情報出力
    uart_send_str("\nmode, data1, data2\n");

    const int block_size = 8;
    const int block_count = WAVEFORM_POINTS / block_size;
    while (1) {
        int counter = 2;
		
		for (int i = 0; i < block_count; i++) {
            while (timer_60s_counter != counter);
            if (waveform_csv(block_size * i, block_size))  break;
            counter++;
		}

        clear_waveform();
    }
}

ISR(TIMER1_COMPA_vect) {
    /**
     * タイマ1 COMPA タイマ割り込みベクタ
     * 
     * 1 ms ごとに実行する
     */
    static uint16_t timer_1s_counter = 0;
    timer_1s_counter++;
    
    if (timer_1s_counter / 1000) {
        // 1 sec 周期
        timer_1s_counter = 0;

        // CSV line 1 start
        uart_send_str("csv-1");

        // // rised_count
        // uart_send_int(rised_count);

        // // measured_count
        // uart_send_str(",");
        // uart_send_int(measured_count);

        // 周波数測定
        float freq = 0;
        if (measured_count != 0) {
            freq = (float)rised_count / (measured_count / ((float)F_CPU / ADC_DIV / ADC_CLOCK));
        }

        // 電圧測定
        float rms = 0;
        if (measured_count != 0) {
            const float temp_rms = sqrt((float)all_cycle_square_sum / measured_count);
            rms = (temp_rms / 1024.0 * 5.0) / DAMPLING_RATIO;
        }
        
        char buf_str[32];

        // 周波数出力
        uart_send_str(",");
        dtostrf(freq, 0, 3, buf_str);
        uart_send_str(buf_str);

        // 電圧値出力
        uart_send_str(",");
        dtostrf(rms, 0, 2, buf_str);
        uart_send_str(buf_str);

        // 平均電圧値出力
        uart_send_str(",");
        dtostrf(wave_offset, 4, 2, buf_str);
        uart_send_str(buf_str);

        uart_send_str("\n");

        // オフセット値の算出
        wave_offset = wave_offset * 0.95 + all_cycle_average * 0.05;

        // 測定リセット、次回の立ち上がりから測定開始
        rised_count = -1;

        timer_60s_counter++;
        if (timer_60s_counter / 60) {
            timer_60s_counter = 0;
        }
    }
}

ISR(ADC_vect) {
    /** ADC 割り込みベクタ
     * 
     * ADC 終了後、実行される割り込み処理
     */

    // ADC値
    const adc_raw_t adc_raw_val = (ADCH << 8) | ADCL;
    // 波形正負極性  1: 正, 0: 負
    const uint8_t current_polarity = (ADCH & 0x02) ? 1 : 0;
    // 前回計測時の正負極性
    static uint8_t pre_polarity = 0;
    // 波形立ち上がりフラグ  1: 立ち上がり
    const uint8_t is_wave_rising = (!pre_polarity && current_polarity);
    pre_polarity = current_polarity;
    
    // 計数値の処理
	if (is_wave_rising) {
        // 波形立上りを検出したら計数値リセットし、測定開始
        if (rised_count == -1) {
            rised_count = 0;
            elapsed_count = 0;
            measured_count = 0;
        } else {
            rised_count++;
            measured_count = elapsed_count + 1;
        }
    }
    elapsed_count++;
    
    // 波形取得
    if (timer_60s_counter == 1) {
        // 60 sec カウンタが 1 秒目のとき

        if (rised_count == 0 && elapsed_count <= WAVEFORM_POINTS) {
            // 1周期目の波形取得
            waveform[elapsed_count - 1] = adc_raw_val;
            waveform_len = elapsed_count;
        }
    }

    // 電圧の 実効値 and 平均 測定
    
    // 1周期分の2乗値の総和
    static uint64_t cycle_square_sum = 0;
    cycle_square_sum += (adc_raw_val - wave_offset) * (adc_raw_val - wave_offset);

    // 1周期分の総和
    static uint64_t cycle_val_sum = 0;
    cycle_val_sum += adc_raw_val;

    if (is_wave_rising) {
        // 波形立ち上がり時

        if (rised_count == 0) {
            // 1周期目

            // 2乗値の総和
            cycle_square_sum = 0;
            all_cycle_square_sum = 0;
            
            // 総和
            cycle_val_sum = 0;
        } else if (rised_count > 0) {
            // 2周期目以降
            
            // 2乗値の総和
            all_cycle_square_sum += cycle_square_sum;
            cycle_square_sum = 0;

            all_cycle_average = (float)cycle_val_sum / measured_count;
        }
    }

}

// GPIO 初期化
void port_init(void) {
    /** 
     * Port B
     *   PB0 - 6: No Connect
     */
    DDRB  = 0x00;
    PORTB = 0x3F;

    /** 
     * Port C
     *   PC0: ADC0 (信号入力)
     *   PC1 - 5: No Connect
     */
    DDRC  = 0b00000000;
    PORTC = 0b00111110;

    /** 
     * Port D
     *   PD0: No Connect
     *   PD1: TxD
     *   PD2 - 7: No Connect
     */
    DDRD  = 0b00000010;
    PORTD = 0b11111101;
}

// タイマ 初期化関数
void timer_init(void) {
    TCCR1A = 0;
    // CTC動作（OCR1Aでクリア）
    TCCR1B = (1 << WGM12);
    TCNT1 = 0;
    OCR1A = F_CPU / TIMER_FREQ - 1;
    // タイマ比較A 割り込み許可
    TIMSK1 = (1 << OCIE1A);

    // タイマカウントスタート（分周なし）
    TCCR1B |= (1 << CS10);
}

// ADC 初期化関数
void adc_init(void) {
    // 基準電圧 AVCC, 右寄せ(ADLAR = 0), ADC0
    ADMUX = (1 << REFS0) | (0 << ADLAR) | (0 << MUX0);
    // AD許可, AD連続変換, AD変換完了割り込み許可, 128分周
    ADCSRA = (1 << ADEN) | (1 << ADATE) | (1 << ADIE) | (7 << ADPS0);
    // 連続変換動作
    ADCSRB = (0 << ADTS0);
}

// ADC 変換開始
void adc_start(void) {
    ADCSRA |= (1 << ADSC);
}
