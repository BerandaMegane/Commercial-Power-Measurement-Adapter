# Commercial-Power-Measurement-Adapter

部屋のコンセントから出ている商用電源（AC100V）の周波数と電圧を測定するアダプタをマイコンで製作しました。  
Windows PC や Raspberry Pi へアダプタを USB 接続すると、仮想 COM ポートとして認識され、測定値を取得することができます。

![筐体に収められた基板](./img/P5040030.JPG)
![回路基板](./img/P5040013.JPG)

回路・マイコンプログラムは次のサイトを参考にしています。~~というかアレンジして作ってみた結果、劣化コピーになりました。~~  
* ChaN氏 [ELM - 電圧周波数測定アダプタ](http://elm-chan.org/works/lvfm/report_j.html)

回路設計のバージョンは2つあります。

* ver1
  * 製作したバージョンですが、修正したい点がいくつかあります。
* ver2
  * 修正バージョンですが、製作していません。

## Circuit design 回路設計 

* [CircuitDesign](./CircuitDesign/)

回路図やLTSpice シミュレーションファイル (.asc) など、アダプタの回路設計に関するファイルを置いています。  
作図ソフトウェアは BSch3V です。

BSch3V - https://www.suigyodo.com/online/schsoft.htm

## Board design 基板設計

* [BoardDesign](./BoardDesign/)

ユニバーサル基板（秋月電子通商 C 基板）で製作しました。部品の配置に関するファイルを置いています。画像に載っていない部品も多いので、あんまり参考にはなりません。
作図ソフトウェアは marmelo です。

marmelo - https://motchy99.blog.fc2.com/blog-entry-70.html

## Program ATmega328P マイコンプログラム

* [Program](./Program/)

Atmel Studio (Microchip Studio) による AVR GCC プロジェクトです。

Atmel Studio - https://www.microchip.com/en-us/tools-resources/develop/microchip-studio

## STL 3Dプリンタ 筐体モデル

* [STL](./STL/)
  * [top.stl](./STL/top.stl)
  * [bottom.stl](./STL/bottom.stl)

3Dプリンタで作った筐体です。
