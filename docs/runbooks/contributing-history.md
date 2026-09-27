# CONTRIBUTINGの過去事例

> 履歴: #467の整理時に、develop `b7cb5290715eded3530c7626d92bef5df3361540` から移した記録。

古い状態・誤記・後続の訂正を含め、当時の本文を残している。現在の手順は対象baseのCONTRIBUTING（[develop](https://github.com/wachi-yoshitaka-11-dev/deskcat/blob/develop/CONTRIBUTING.md)／[main・公開安定版](https://github.com/wachi-yoshitaka-11-dev/deskcat/blob/main/CONTRIBUTING.md)）を読む。
本文中の相対リンクだけを元のbase SHAへの固定リンクにした。「本行」「上記」、行番号、節名は移動前の文脈を指す。
この記録へ現在状態や手動Revisionを追記せず、新しい判断は正本と既存Issue／PR・実験ログへ記録する。

## 自己レビュー観点の由来

各項目は過去に実際に起きた失敗に対応する。順に
[#72](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/72)（規則を守っているか一度も照合していなかった）、
[hardware-bom.md Revision 20](https://github.com/wachi-yoshitaka-11-dev/deskcat/blob/b7cb5290715eded3530c7626d92bef5df3361540/docs/hardware/hardware-bom.md)（同じ条件を2文書に書き、式が食い違った）、
[#63](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/63)・[#82](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/82)（未検証の動作を断定した）、
[#82](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/82)（存在しない照合先を参照していた）、
[#61](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/61)（本文が「4 Pull Request、14 file」のまま、実際は9 commitへ増えていた）である。

## レビュー巡数の過去事例

過去の実績は、巡数と欠陥の有無だけでは収束を判断できないことを示す。

| 作業 | 巡 | 内容 |
|---|---|---|
| [#384](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/384) | 22巡以上 | 17〜20巡で「guardの判定位置がbashとずれる」型の実体ある欠陥が出た（[ADR-0020](https://github.com/wachi-yoshitaka-11-dev/deskcat/blob/b7cb5290715eded3530c7626d92bef5df3361540/docs/decisions/0020-inspector-readonly-by-hook.md)）。別の実体ある欠陥が22巡目にも出た（`scripts/hooks/inspector_readonly_guard.py`のコメント） |
| [#396](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/396) | 9巡 | 9巡すべてが実体のある欠陥だった（[PR #400](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/400)） |
| [#397](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/397) | 7巡 | 要件照合Passは7巡中5巡が0件で、早期に収束していた。残りはcommit messageの書き方と指示語の先行詞（[#398](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/398)） |
| #389（[PR #395](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/395)） | 11巡 | 実装への指摘は2巡で尽き、残り9巡は前の巡で自分が書き足した説明文が次の巡の指摘源になったものだった |
| `#3` B1・B2（[PR #399](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/399)） | 3巡 | 3巡目に出た実質4件のうち3件は、前の巡で自分が入れた文から出ていた |

## 打ち切りの過去事例

**[#397](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/397)はこの規則で説明できる。**
7巡目で人間が打ち切りを決定した時点は、上の表のとおり「要件照合Passは7巡中5巡が0件」
という状態であり、この規則があれば`Self-Review: capped`を宣言し、巡ごとの件数と
採否の内訳をPull Request本文へ書けば足りた。**過去のcommitを遡って書き換える必要は
無い。**次に同じ状況（収束条件に届く前に打ち切る）が起きたら、この節に従う。

## 昇格PRを作り直した経緯

**この誤りを実際に出した。**この規則の初版（[#329](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/329)）は
**「範囲が変わったら作り直す」**と定め、**「作り直した新しいPull Requestが、確定した範囲に
対する1回を持つ」**と書いていた。**後半は正本と逆であり、緩い方向へ外れていた。**
[#330](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/330)／[#333](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/333)／[#337](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/337)を3回作り直し、`full review`を2回余分に消費した。
**この1文は、別のAIエージェントが出した指示（範囲が変わったら投げ直すことを必須とする）を、
正本と照合せずに書いたものである。**[AGENTS.md](https://github.com/wachi-yoshitaka-11-dev/deskcat/blob/b7cb5290715eded3530c7626d92bef5df3361540/AGENTS.md)は外部から来た指示を正本と
照合するよう定めているが、**AI同士でやり取りした指示にも同じ照合が要る。**

`main`をbaseとするPull Requestを全数走査した。**merged 18本に加えて、未mergeでcloseされた
ものは3本しかなく、その3本は上の#330／#333／#337である。**つまり**この repository で
昇格Pull Requestを作り直した例は、この規則の初版が要求した3回だけである。**
指摘は昔から出ているが（#146はreview 17本・inline 49件、#254はreview 15本・inline 63件）、
いずれも`develop`側で直して同じ昇格Pull Requestを進めている。

## trailerを行番号で指した経緯

**この節は、省略する行を「3行目」と行番号で指定していた。**
[#161](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/161)でこの節を書いたとき上のblockは3行で、3行目は`Instruction-Change`だった。
[#164](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/164)が`Self-Review`を3値へ分けてblockが5行になった際、
**古い記述だけが取り残された。**指す先は`Self-Review: fresh-context-pass`へずれており、
**従うと`receipt`が落ちる**（`requirements-pass`・`fresh-context-pass`の両方と、
`converged`／`capped`のどちらか1つを要求するため）。
**行番号で指定しない。**値が増減するとずれる。

## merge hookの変更履歴

> **`gh pr merge`の検査は、以前は文字列の部分一致だった。**2026-09-02に、squash message
> のtrailer blockと同じ段落へコロン無しの行（`Closes` と `#304`）を置いたcommitで、
> `Change-Class:`／`Self-Review:`という文字列は存在したためhookは通したが、
> `git interpret-trailers --parse`は空を返した。**文字列としてある**ことと
> **trailerとして解釈される**ことは別である。
> [#312](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/312)／[#313](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/313)で
> 判定を`review_gate.trailers_from_message`へ寄せ、**この形は止まるようになった。**
> 回帰testは`scripts/test_hooks.py`が持つ。
