# 公開asset register

> 状態: Active
> 適用範囲: GitHub Pagesへ公開するimage、font、その他のbinary
> 方針: [ADR-0003](../decisions/0003-public-documentation-publishing.md)

## 目的

公開するbinary assetの出所と再配布許諾を記録する。

Markdownは差分reviewとlink checkの対象になるが、binaryは内容scanが効かない。出所が不明なimageやfontは、[GitHub Pages公開runbook](../runbooks/github-pages-publishing.md)の公開対象外である。この文書は、公開してよいと確認したassetの根拠を残す唯一の場所とする。

## 登録の要件

Assetを追加する前に、次をすべて満たす。

1. 作成者または権利者を特定する。
2. 作成または取得の方法を記録する。
3. 再配布許諾の根拠を記録する。
4. 確認日を記録する。
5. SHA-256を記録する。差し替え時は必ず更新する。**この要件はbinary assetに適用する。**
   Text asset（`.svg`、`.css`、`.scss`、`.txt`）と、scriptがsourceから生成するbinaryは
   内容がdiff reviewと内容scanの対象であり、hashは編集ごとに変わるだけで古くなる。
   `pages/assets-manifest.json`はtext assetの`sha256`を**誤りとして拒否する**ため、
   両者の要件を一致させる。該当欄には固定していない理由を書く。
6. 実機写真ではないimageは、その旨をpage上へ明記する。

許諾の根拠は、assetの由来で書き分ける。

- **権利者がrepository所有者本人の場合**: 本repositoryの[MIT License](https://github.com/wachi-yoshitaka-11-dev/deskcat/blob/c69f9a7d7767a6b4572e9f6920939529765316fc/LICENSE)に従って公開・再配布する旨を記録する。個別の許諾文書は不要である。linkはcommit固定にする。`main`を指すと、後の変更で法的根拠が黙って書き換わる。
- **第三者が権利を持つ場合**: license名または書面での許諾と、その取得元を記録する。

いずれかを確認できない場合は、assetをrepositoryへ追加しない。

## 登録済みasset

### `pages/assets/deskcat-concept.jpg`

| 項目 | 内容 |
|---|---|
| 用途 | GitHub Pages入口pageのconcept image |
| 作成者・権利者 | wachi-yoshitaka-11-dev（repository所有者） |
| 作成方法 | repository所有者がChatGPTを用いて生成 |
| 再配布許諾の根拠 | 権利者本人による許諾。本repositoryの[MIT License](https://github.com/wachi-yoshitaka-11-dev/deskcat/blob/c69f9a7d7767a6b4572e9f6920939529765316fc/LICENSE)（commit `c69f9a7d`時点）に従って公開・再配布する |
| 確認日 | 2026-07-29 |
| SHA-256 | `615063ED60596F55066D602E1C44ACFB46D6D3103B9D234AEB8170E864FBB5B2` |
| 寸法・size | 720 x 720、77,305 bytes |
| 実機写真か | いいえ。入口pageのcaptionへ明記済み |

検証に使うSHA-256の正本は`pages/assets-manifest.json`である。`prepare_pages.py`はそのhashと実fileを照合し、不一致でbuildを失敗させる。Assetを差し替える場合は、manifestと本文書の両方を更新する。

### `pages/assets/deskcat-paw.svg`

| 項目 | 内容 |
|---|---|
| 用途 | 肉球のmotif。背景の透かし、見出しとcardのmarker、footerで`mask-image`として使う |
| 作成者・権利者 | wachi-yoshitaka-11-dev（repository所有者） |
| 作成方法 | repository所有者がconcept image（mugとnotebookに描かれた肉球）を基に、幾何形状だけで作図 |
| 再配布許諾の根拠 | 権利者本人による許諾。本repositoryの[MIT License](https://github.com/wachi-yoshitaka-11-dev/deskcat/blob/c69f9a7d7767a6b4572e9f6920939529765316fc/LICENSE)（commit `c69f9a7d`時点）に従って公開・再配布する |
| 確認日 | 2026-08-21 |
| SHA-256 | 記録しない。text assetであり内容はdiff reviewとscanの対象。manifestは`sha256`を誤りとして拒否する |
| 寸法・size | viewBox 24 x 24、1 KiB未満 |
| 実機写真か | いいえ。写真ではなく単色の図形である |

`mask-image`として使うため色はCSS側のtokenが決める。SVG自身は単色で持ち、暗色modeへ追従させるために色を焼き付けない。

### `pages/assets/deskcat-paw-tile.svg`

| 項目 | 内容 |
|---|---|
| 用途 | 背景の透かし専用のtile。`body::before`が`mask-image`として`repeat`する |
| 作成者・権利者 | wachi-yoshitaka-11-dev（repository所有者） |
| 作成方法 | repository所有者が[`deskcat-paw.svg`](#pagesassetsdeskcat-pawsvg)の肉球を、720 x 720のtileの中へ90個、位置・角度・大きさを変えて配置した。**新しい図形は描いていない。**肉球は`<defs>`に1つだけ持ち、各配置は`<use>`で参照する。配置はseed `20260823`の擬似乱数で決め、外接矩形がトーラス（上下左右がつながった面）で重ならず、すき間が6単位以上空くまで棄却法で採り直した（scale 0.72〜1.15、rotate -40°〜+40°）。端を跨ぐ肉球は反対側へ複製し、`repeat`で継ぎ目が出ないようにしてある |
| 再配布許諾の根拠 | 権利者本人による許諾。`deskcat-paw.svg`の派生であり、本repositoryの[MIT License](https://github.com/wachi-yoshitaka-11-dev/deskcat/blob/c69f9a7d7767a6b4572e9f6920939529765316fc/LICENSE)（commit `c69f9a7d`時点）に従って公開・再配布する |
| 確認日 | 2026-08-23 |
| SHA-256 | 記録しない。text assetであり内容はdiff reviewとscanの対象。manifestは`sha256`を誤りとして拒否する |
| 寸法・size | viewBox 720 x 720、描画要素94（うち継ぎ目用の複製4）、約8 KiB |
| 実機写真か | いいえ。写真ではなく単色の図形である |

**散らした見た目をtileの中で作るためのfileである。**以前は`deskcat-paw.svg`を寸法の違う2枚のmask layerで重ねて散らそうとしていたが、`mask-image`の複数レイヤーは1本のアルファチャンネルへ合成されるため、**重なった肉球が融合して塊になった。**1枚のtileにすれば構造的に起きない。

### `favicon.ico`

| 項目 | 内容 |
|---|---|
| 用途 | 全pageの`<link rel="icon">`が参照するfavicon |
| 作成者・権利者 | wachi-yoshitaka-11-dev（repository所有者） |
| 作成方法 | repository所有者が`pages/assets/deskcat-concept.jpg`の猫顔を32 x 32と16 x 16のpixel artへ落とし、`scripts/prepare_pages.py`が`FAVICON_ART_32`／`FAVICON_ART_16`からICOを組み立てる |
| 再配布許諾の根拠 | 権利者本人による許諾。concept image由来の派生であり、本repositoryの[MIT License](https://github.com/wachi-yoshitaka-11-dev/deskcat/blob/c69f9a7d7767a6b4572e9f6920939529765316fc/LICENSE)（commit `c69f9a7d`時点）に従って公開・再配布する |
| 確認日 | 2026-08-21 |
| SHA-256 | 記録しない。正本はsourceのASCII artであり、内容はdiff reviewで読める。形式と寸法は`scripts/test_pages_guards.py`が検証する |
| 寸法・size | 32 x 32と16 x 16の2枚、5,430 bytes |
| 実機写真か | いいえ。concept imageから作図したpixel artである |

`pages/assets/`には置かない。`prepare_pages.py`がstaging時に生成するため、manifestの対象外である。

## 履歴

| 日付 | 内容 |
|---|---|
| 2026-07-29 | `pages/assets/deskcat-concept.jpg`を登録。1254 x 1254から720 x 720へ縮小した版のSHA-256を記録 |
| 2026-08-21 | `pages/assets/deskcat-paw.svg`と`favicon.ico`を登録。登録要件5のSHA-256をbinary assetへ限定し、text assetと生成binaryはdiff reviewで担保する旨を明記（[ADR-0009](../decisions/0009-pages-own-layout.md)） |
| 2026-08-23 | `pages/assets/deskcat-paw-tile.svg`を登録。背景の透かしを2枚のmask layerから1枚のtileへ変更した（[#185](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/185)） |
