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
5. SHA-256を記録する。差し替え時は必ず更新する。
6. 実機写真ではないimageは、その旨をpage上へ明記する。

許諾の根拠は、assetの由来で書き分ける。

- **権利者がrepository所有者本人の場合**: 本repositoryの[MIT License](../../LICENSE)に従って公開・再配布する旨を記録する。個別の許諾文書は不要である。
- **第三者が権利を持つ場合**: license名または書面での許諾と、その取得元を記録する。

いずれかを確認できない場合は、assetをrepositoryへ追加しない。

## 登録済みasset

### `pages/assets/deskcat-concept.jpg`

| 項目 | 内容 |
|---|---|
| 用途 | GitHub Pages入口pageのconcept image |
| 作成者・権利者 | wachi-yoshitaka-11-dev（repository所有者） |
| 作成方法 | repository所有者が生成AIを用いて作成 |
| 再配布許諾の根拠 | 権利者本人による許諾。本repositoryの[MIT License](../../LICENSE)に従って公開・再配布する |
| 確認日 | 2026-07-29 |
| SHA-256 | `615063ED60596F55066D602E1C44ACFB46D6D3103B9D234AEB8170E864FBB5B2` |

検証に使うSHA-256の正本は`pages/assets-manifest.psd1`である。`prepare-pages.ps1`はそのhashと実fileを照合し、不一致でbuildを失敗させる。Assetを差し替える場合は、manifestと本文書の両方を更新する。
| 寸法・size | 720 x 720、77,305 bytes |
| 実機写真か | いいえ。入口pageのcaptionへ明記済み |

## 履歴

| 日付 | 内容 |
|---|---|
| 2026-07-29 | `pages/assets/deskcat-concept.jpg`を登録。1254 x 1254から720 x 720へ縮小した版のSHA-256を記録 |
