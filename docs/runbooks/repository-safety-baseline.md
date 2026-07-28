# Repository Safety Baseline

> 記録日: 2026-07-27
> 最終確認日: 2026-07-28
> Repository: `wachi-yoshitaka-11-dev/deskcat`
> 基盤commit: `19853d0`

## 結果

以下の管理を維持することで、ローカルrepositoryは基盤整備作業を継続できる状態にある。

## 確認済み状態

| 確認項目 | 結果 |
|---|---|
| Remote | `origin`は`https://github.com/wachi-yoshitaka-11-dev/deskcat.git`を参照 |
| Visibility | Public。repository APIへ匿名でaccess可能 |
| Default branch | `main` |
| Remote-tracking branch | `origin/main` |
| 初期tracked file | `.gitignore`、`LICENSE`、`README.md` |
| 現在公開中のlicense | MIT |
| `.env`がtrackedか | No |
| `.env`の履歴 | 該当commitなし |
| `.env`がignore対象か | Yes |
| Secretらしいmarkerのscan | 対象text fileに一致なし |
| ローカルPDF | 確認・要約後に削除済み |
| 一時的なAI基盤参考資料 | Repository用Governanceへ変換後に削除済み |

Marker scanは基本的な予防策であり、任意の内容にsecretがないことの証明ではない。外部公開の前には、毎回staged changeをreviewする。

## 公開ポリシー

- ユーザーから明示的な変更依頼がない限り、現在のpublic visibilityを維持する。
- `.env`、資格情報、key、token、ローカルのsecretをcommitしない。
- 提供された参考fileを、ローカルに存在するという理由だけで公開しない。
- Commit前とpush前にstaged file一覧をreviewする。
- Push、release、tag、visibility、repository settingの変更を外部操作として扱う。

## GitHub認証

最初のGitHub CLI資格情報checkは失敗した。2026-07-28にbrowser認証を完了し、その後、予定していたlabel、milestone、private vulnerability reporting、`main`の最小保護を適用してread-back確認した。

認証状態は期限切れまたは失効する可能性がある。以後のGitHub書き込み前に毎回確認する。Tokenをproject file、文書、command引数、logへ記載しない。

## 公開済みの基盤commit範囲

最終diff review後、commit `19853d0`へ次を含め、`origin/main`への反映を確認した。

- `.editorconfig`
- `.gitattributes`
- `.gitignore`
- `AGENTS.md`
- `docs/planning/development-foundation-plan.md`
- `docs/`
- Repository構成の責務README
- この計画で作成したGitHub community fileとworkflow file

元の`.env`と一時的な参考資料はcommit対象に含めない。
