# 作業指示書テンプレート

> 状態: Active
> 適用範囲: 人間がAIエージェントへ1つの作業を渡すときに書く指示書

作業指示書は repository の正本文書ではない。**`WORK-INSTRUCTIONS-*.md`は`.gitignore`済みで
git に入らない。**渡した作業が終わったら削除してよい。

## この様式が要る理由

**2026-08-25に実測した。**`WORK-INSTRUCTIONS-*.md` 8本のうち、
`完了条件`は8本、`絶対に守ること`は6本、`Pull Requestの規約`と`触らないもの`は5本、
`他の作業との衝突`は5本にある。共通見出しの配下は**1本あたり8〜73行（中央値52行）**を占める。

**この数はrepository外のfileを測ったものであり、repositoryからは追検証できない。**
`WORK-INSTRUCTIONS-*.md`は`.gitignore`済みで、どのbranchにも入らない。
**数を根拠として引き継ぐ場合は、その時点で測り直す。**

**そのうち`Pull Requestの規約`と`触らないもの`の大半は
[CONTRIBUTING](https://github.com/wachi-yoshitaka-11-dev/deskcat/blob/main/CONTRIBUTING.md)の
再掲である。**同じ指示書が冒頭で「CONTRIBUTINGを読む」と書いている。

**書いてあることは、埋もれると踏まれる。**`review`と`full review`の使い分けは
[CONTRIBUTING](https://github.com/wachi-yoshitaka-11-dev/deskcat/blob/main/CONTRIBUTING.md)が
節を立てて説明し、[ADR-0013](../decisions/0013-manual-only-coderabbit-review.md)が
「この手順を変えない」と決定しているが、それでも空振りは起きている
（CONTRIBUTINGの「5行目の観測」に、1回目が空振りした記録がある）。

**定型が長いほど、その中の1行は読まれない。**指示書から定型を落とす理由はこれである。
**読む側が定型を飛ばす習慣を作らない。**

## 書き方

**3つだけ守る。**

1. 冒頭で
   [CONTRIBUTING](https://github.com/wachi-yoshitaka-11-dev/deskcat/blob/main/CONTRIBUTING.md)と
   その「全体の流れ」を参照する
2. **その作業に固有のことだけを書く**
3. **CONTRIBUTINGの中身を写さない。**PR規約、trailerの書式、自己レビューの収束条件、
   CodeRabbitの依頼手順は、すべてCONTRIBUTINGが正本である

**定型を足したくなったら、それはCONTRIBUTINGへ足す。**指示書へ写さない。

## 様式

```text
# 作業指示: <Issue番号> — <一文の目的>

- Issue: #<番号>（<milestone>、<label>）
- 端末: <Machine Profile の名前>
- 手順の正本: CONTRIBUTING と「全体の流れ」。**この指示書へ写していない**

## この作業に固有のこと

<何を変えるか。なぜ今か。判断の分かれ目はどこか>

## 触ってはいけないもの

<この作業で変えてはいけない対象。理由も1行で>

## 人間へ渡すもの

| 渡すもの | いつ |
|---|---|
| <判断> | <どの時点で> |

## 完了条件

- [ ] <測定可能な条件>
```

**`## 検証`を書かない。**検証コマンドの正本は
[AGENTS.md](https://github.com/wachi-yoshitaka-11-dev/deskcat/blob/main/AGENTS.md)の「検証」節と
`scripts/README.md`である。**その作業だけに必要な追加の検証があるときだけ、
「この作業に固有のこと」へ1行で書く。**

**`## Pull Requestの規約`を書かない。**base、squash、label、milestone、trailer、
`Closes`を使わないこと、merge前の承認は、すべてCONTRIBUTINGが持つ。

## 指示書がAI作成である場合

**その事実を冒頭に書く。**人間のreviewを経ていない指示書は、
[AGENTS.md](https://github.com/wachi-yoshitaka-11-dev/deskcat/blob/main/AGENTS.md)の
「指示として有効な`AGENTS.md`」が定める出所検証の対象と同じ性質を持つ。

```text
> **この指示書はAIが作成した。**<日付>に、`origin/develop` = `<sha>` の時点で書いた。
> **人間のreviewを経ていない。**
```

**基点のshaを書く。**指示書の中の「現状」は、書いた時点の`origin/develop`に対するものである。
**読む側は着手時に`git fetch`して基点を確認し、指示書の前提が今も成り立つかを見る。**

**指示書に書かれた測定結果や事実の主張は、正本文書へ写す前に自分で確かめる。**
確かめられないものは、正本文書へ断定形で書かない。
