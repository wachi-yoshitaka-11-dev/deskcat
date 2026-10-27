# 独り言用 k8x12 glyph

## 出所と許諾

- 原作者・出所: Num Kadoma、[k8x12 公式配布ページ](https://littlelimit.net/k8x12.htm)。
- 採用版: 2021-05-05、公式 X11 BDF archive の k8x12.bdf（通常版）。
- 元 archive: [k8x12_bdf_2021-05-05.zip](https://littlelimit.net/arc/k8x12/k8x12_bdf_2021-05-05.zip)。
- SHA-256: 305ab94915ec3c8e1ada08f493ee5bfdd21e2710a97d7c4d4272444d8e18ff40。
- Copyright (C) 2015-2021 Num Kadoma。
- 許諾: 同梱の k8x12.txt と[作者の license](https://littlelimit.net/font.htm)は、商用を含む利用・複製・改変・再配布を無制限に許可し、無保証とする。生成した部分集合もこの条件で再配布できる。
- 保守状況: 採用 archive は 2021-05-05 版に固定する。#23 の固定文に必要な文字と、今回の拡張集合を生成時と host test で確認する。将来の字形更新は自動追従しない。

比較した候補は #23 の[調査コメント](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/23#issuecomment-6115319288)に記録した。8×12 は 8×8 の美咲ゴシックより仮名の縦方向に余裕がある。実 LCD での可読性は未確認である。

## 生成

repository の root で、公式 archive を手元の任意の場所に保存してから実行する。

```bash
python3 scripts/generate_speech_glyphs.py /path/to/k8x12_bdf_2021-05-05.zip
python3 scripts/generate_speech_glyphs.py /path/to/k8x12_bdf_2021-05-05.zip --check
```

script は archive の SHA-256 を固定してから k8x12.bdf を読む。入力文字は
`crates/deskcat-domain/src/local_speech.rs` の `LOCAL_SPEECH_LINES` の3文、
表示可能な ASCII（U+0020–U+007E）、JIS X 0208 の非漢字領域（1–8区。記号、
ひらがな、カタカナ、全角英数字、ギリシャ文字、キリル文字、罫線を含む）、
第一水準漢字（16–47区）である。Python 標準の `iso2022_jp` codec から割当済みの
文字を取得し、重複を除いて Unicode scalar 順に並べる。文字集合の SHA-256 を
script に固定し、codec の対応が変われば生成を止める。全3,584字に公式 archive の
字形があることを検査し、#23 の既存22字も残す。
生成先は `crates/deskcat-face/src/speech_glyphs.rs` である。
各字形は8×12 cell の行ごとの1 byte として記録し、bit 7 が左端に対応する。
元 archive と全文字 font は repository に置かない。新しい package や実行時依存は無い。
bitmap 本体は3,584×12 = 43,008 byte、Unicode scalar と bitmap を組にした
生成 table は型の上で3,584×16 = 57,344 byteである。`TextLayout` は
最大20字の表 index を `u16` の固定長配列に保持し、Unicode 順の表を二分探索する。
1文字あたりの検索は表全体の線形走査をせず、入力は20字で打ち切る。
1 cell の RGB565 展開 buffer は16×24×2 = 768 byteを stack に置く。

## ESP32 製品 build の容量

2026-10-14 に VM の ESP32 Build profile で、既定 feature（製品経路）の
`cargo build --locked`を変更前後に実行した。profile は `dev`（`opt-level = "z"`、
debug 情報あり）である。ESP-IDF v5.5.3 の既存 SDK を外部 `IDF_PATH`で参照するため、
build guard が要求する明示的な override を用いた。SDK は tag `v5.5.3`、
commit `2c211b236707889e8400c4dc5644dd5c4ee071e0` と確認した。
詳細な条件と失敗した最初の環境設定は
[Version Record の #522 追記](../toolchains/version-records/2026-08-06-esp32-build-linux.md)に記録した。

| 指標 | 22字（変更前） | 3,584字（変更後） | 差 |
|---|---:|---:|---:|
| `elf2image` の app image | 850,912 byte | 908,112 byte | +57,200 byte |
| `.flash.rodata` | 264,036 byte | 321,108 byte | +57,072 byte |
| `.flash.text` | 509,820 byte | 509,948 byte | +128 byte |
| `.iram0.vectors` | 1,028 byte | 1,028 byte | 0 |
| `.iram0.text` | 62,103 byte | 62,103 byte | 0 |
| `.dram0.data` | 13,522 byte | 13,522 byte | 0 |
| `.dram0.bss` | 3,448 byte | 3,448 byte | 0 |
| `.noinit` | 2 byte | 2 byte | 0 |

生成した partition table の factory app partition は1,048,576 byteであり、
変更後の image はそこに収まり、残りは140,464 byteである。表は flash の
`.flash.rodata`に入り、静的 IRAM と DRAM の上記 section は増えていない。
これは build artifact の静的比較であり、実行時の peak RAM 使用量ではない。

## MVP の範囲と未確定事項

AGENTS.md は初期 MVP に画像アセットを含めない。ここでの glyph は独り言の**文字**を
表示するための文字データであり、表情の画像や一般画像を追加しない。収録外は
表示前に分類して拒否する。全画面に中央1行、各 glyph を2倍の16×24 pixel で描く値と
表示時間上限は **暫定** であり、Protocol の PROTO-TBD-007 は未解決である
（実 LCD での計測は EXP-046 で取れたが、上限値の妥当性は未判定）。
ESP32 build で得られる静的使用量と、実 LCD での字形・描画時間・memory の実測を分けて報告する。

## 表情との優先順位

文字表示を受理した時点で、進行中の表情描画を中断する。画面を帯単位で黒くしてから
glyph を1文字ずつ描き、指定された表示時間の間は文字画面を優先する。その間に
`set_expression` を受理した場合は最新の表情名を保持し、文字を消す期限に達したら
その表情を全画面で描き直す。文字の描画が失敗した場合も、文字表示を中止して
最新の表情の描き直しを試みる。表示時間は**受理時刻から**数える暫定動作である。
短い時間では描画完了前に期限に達する可能性があるため、実 LCD の所要時間は EXP-046 で測った
（`accept_to_draw_ms` は 612〜709 ms で、字数による差は小さい。事実の記録であり、上限の根拠にはしない）。
PROTO-TBD-007 の最終値と起算点は、まだ決めていない。表情の3種類と矩形は変更しない。

段階 (a) では firmware の描画 adapter と main loop の期限監視を追加した。
段階 (b)（#587）で、`pi_link.rs` が受けた `show_text` を `FaceDrawer::show_text` へ渡し、
受理と拒否の ACK を返すようになった。実 LCD での表示は、EXP-037 と EXP-046 で観察した（目視は利用者の申告。字形の判定ではない）。
