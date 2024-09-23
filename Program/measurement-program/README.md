# measurement-program

元々 C で書かれていた AVR マイコンプログラムを Rust で書き直しました。  
Rust (nightly) コンパイラ等が必要です。

## Build Instructions ビルド手順
1. Install prerequisites as described in the [`avr-hal` README] (`avr-gcc`, `avr-libc`).

2. Run `cargo build --relase` to build the firmware.  
The binary files will be generated in `target/avr-atmega328p/release/***/elf`.

[`avr-hal` README]: https://github.com/Rahix/avr-hal#readme

## License
This project is licensed under the MIT License, see the LICENSE.txt file for details.
