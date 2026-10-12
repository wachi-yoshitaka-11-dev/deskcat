# #23 独り言用 k8x12 glyph

## 出所と許諾

- 原作者・出所: Num Kadoma、[k8x12 公式配布ページ](https://littlelimit.net/k8x12.htm)。
- 採用版: 2021-05-05、公式 X11 BDF archive の k8x12.bdf（通常版）。
- 元 archive: [k8x12_bdf_2021-05-05.zip](https://littlelimit.net/arc/k8x12/k8x12_bdf_2021-05-05.zip)。
- SHA-256: 305ab94915ec3c8e1ada08f493ee5bfdd21e2710a97d7c4d4272444d8e18ff40。
- Copyright (C) 2015-2021 Num Kadoma。
- 許諾: 同梱の k8x12.txt と[作者の license](https://littlelimit.net/font.htm)は、商用を含む利用・複製・改変・再配布を無制限に許可し、無保証とする。生成した部分集合もこの条件で再配布できる。
- 保守状況: 作者の公開履歴では 2021-05-05 が最新。今回の固定文候補に必要な文字があることを生成時と host test で確認する。将来の字形更新は自動追従しない。

比較した候補は #23 の[調査コメント](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/23#issuecomment-6115319288)に記録した。8×12 は 8×8 の美咲ゴシックより仮名の縦方向に余裕がある。実 LCD での可読性は未確認である。

## 生成

repository の root で、公式 archive を手元の任意の場所に保存してから実行する。

```bash
python3 scripts/generate_speech_glyphs.py /path/to/k8x12_bdf_2021-05-05.zip
python3 scripts/generate_speech_glyphs.py /path/to/k8x12_bdf_2021-05-05.zip --check
```

script は archive の SHA-256 を固定してから k8x12.bdf を読む。入力文字は
`crates/deskcat-domain/src/local_speech.rs` の `LOCAL_SPEECH_LINES` の3文から取り、
重複を除いた22文字だけを `crates/deskcat-face/src/speech_glyphs.rs` に生成する。
各字形は8×12 cell の行ごとの1 byte として記録し、bit 7 が左端に対応する。
元 archive と全文字 font は repository に置かない。新しい package や実行時依存は無い。
22字の bitmap 本体は22×12 = 264 byte、Unicode scalar と bitmap を組にした
生成 table は型の上で22×16 = 352 byteである。1 cell の RGB565 展開 buffer は
16×24×2 = 768 byteを stack に置く。これらは source から数えた局所的な容量であり、
firmware 全体の RAM／flash 使用量や実行時 peak の測定値ではない。

## MVP の範囲と未確定事項

AGENTS.md は初期 MVP に画像アセットを含めない。ここでの glyph は独り言の**文字**を
表示するための最小データであり、表情の画像や一般画像を追加しない。22字以外は
表示前に分類して拒否する。全画面に中央1行、各 glyph を2倍の16×24 pixel で描く値と
表示時間上限は **暫定** であり、Protocol の PROTO-TBD-007 は未解決である。
ESP32 build で得られる静的使用量と、実 LCD での字形・描画時間・memory の実測を分けて報告する。

## 表情との優先順位

文字表示を受理した時点で、進行中の表情描画を中断する。画面を帯単位で黒くしてから
glyph を1文字ずつ描き、指定された表示時間の間は文字画面を優先する。その間に
`set_expression` を受理した場合は最新の表情名を保持し、文字を消す期限に達したら
その表情を全画面で描き直す。文字の描画が失敗した場合も、文字表示を中止して
最新の表情の描き直しを試みる。表示時間は**受理時刻から**数える暫定動作である。
短い時間では描画完了前に期限に達する可能性があるため、実 LCD の所要時間を測って
PROTO-TBD-007 の最終値と起算点を決める。表情の3種類と矩形は変更しない。

段階 (a) では firmware の描画 adapter と main loop の期限監視を追加した。
段階 (b)（#587）で、`pi_link.rs` が受けた `show_text` を `FaceDrawer::show_text` へ渡し、
受理と拒否の ACK を返すようになった。実 LCD での表示は、まだ確かめていない。
