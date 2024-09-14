# Commercial-Power-Measurement-Adapter

部屋のコンセントから出ている商用電源（AC100V）の周波数と電圧を測定するアダプタをマイコンで製作しました。  
Windows PC や Raspberry Pi へアダプタを USB 接続すると、仮想 COM ポートとして認識され、測定値を取得することができます。

![筐体に収められた基板](./img/P5040030.JPG)
![回路基板](./img/P5040013.JPG)

回路・マイコンプログラムは次のサイトを参考にしています。~~というかアレンジして作ってみた結果、劣化コピーになりました。~~  
* ChaN氏 [ELM - 電圧周波数測定アダプタ](http://elm-chan.org/works/lvfm/report_j.html)

Qiita にて解説しています。
* Qiita - [電源周波数測定アダプタを真似して作ってみた](https://qiita.com/BerandaMegane/items/4fd927695e5ca32714c5)

## Circuit design 回路設計 

回路図や LTSpice シミュレーションファイル (.asc) など、アダプタの回路設計に関するファイルを置いています。  
作図ソフトウェアは BSch3V です。

* BSch3V - https://www.suigyodo.com/online/schsoft.htm
* LTSpice - https://www.analog.com/jp/resources/design-tools-and-calculators/ltspice-simulator.html

回路設計のバージョンは2つあります。

* [ver1](https://github.com/BerandaMegane/Commercial-Power-Measurement-Adapter/tree/main/CircuitDesign/ver1)
  * 製作したバージョンですが、修正したい点がいくつかあります。
* [ver2](https://github.com/BerandaMegane/Commercial-Power-Measurement-Adapter/tree/main/CircuitDesign/ver2)
  * 修正バージョンですが、製作していません。

## Board design 基板設計

ユニバーサル基板（秋月電子通商 C 基板）で製作しました。部品の配置に関するファイルを置いています。画像に載っていない部品も多いので、あんまり参考にはなりません。  
作図ソフトウェアは marmelo です。

* marmelo - https://motchy99.blog.fc2.com/blog-entry-70.html

## Program ATmega328P マイコンプログラム

制御マイコンには AVR ATmega328P を使用しており、動作クロック 20MHz で動作している前提でプログラミングされています。  
Atmel Studio (Microchip Studio) による AVR GCC プロジェクトで製作しました。

ビルド済みバイナリ (.elf, .hex) を使ってプログラムを書き込むときは、次を参考にしてください。

1. [Releases](https://github.com/BerandaMegane/Commercial-Power-Measurement-Adapter/releases) からバイナリをダウンロードします。
1. AVR ライターを使用し、マイコンにバイナリを書き込みます。  
AVR ライターは Arduino があれば自作することもできます。

* Atmel Studio - https://www.microchip.com/en-us/tools-resources/develop/microchip-studio

## STL 3Dプリンタ 筐体モデル

3Dプリンタで作った筐体です。

* [top.stl](./STL/top.stl)
* [bottom.stl](./STL/bottom.stl)
