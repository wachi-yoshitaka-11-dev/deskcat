# Repository Safety Baseline

> 記録日: 2026-07-27
> 最終確認日: 2026-08-01（secret scanning custom patternのread-back。2026-07-29と同一端末・同一profile・同一tool版で実施）
> 前回確認日: 2026-07-29（GitHub security設定のread-back）
> 前々回確認日: 2026-07-28（`.env` ignore規則の是正）
> Repository: `wachi-yoshitaka-11-dev/deskcat`
> 基盤commit: `19853d0`

確認済み状態と実行環境の結果欄には、記載時点で実際にcommandを実行して得た値だけを書く。
方針として意図している状態を、確認済みとして記載しない。

本文にはこのほかに方針、背景説明、訂正履歴を含む。これらは実行結果ではない。

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
| `.env`がtrackedか | No。`git ls-files`で確認 |
| `.env`の履歴 | 該当commitなし |
| `.env`がignore対象か | Yes。`git check-ignore`で確認（2026-07-28に修正。下記の訂正記録を参照） |
| 下記patternがignore対象か | Yes。`*.pem`、`*.key`、`id_rsa*`、`credentials.json`、`secrets.*`を`git check-ignore`で確認。**検査したのはこの5 patternだけであり、資格情報形式全般を網羅した確認ではない** |
| `.gitignore`の残りの秘密情報patternがignore対象か | Yes（2026-08-03に確認、commit `d06cdc1`）。`*.p12`、`*.pfx`、`id_ecdsa*`、`id_ed25519*`、`*.local.toml`、`*.local.yml`、`*.local.yaml`、`*.local.json`を`git check-ignore -v`で確認。同時点で`git ls-files`により、これら全patternおよび上記5 patternに一致するtracked fileが無いことも確認した |
| GitHub secret scanning | 有効 |
| GitHub push protection | 有効 |
| Secretらしいmarkerのscan | 対象text fileに一致なし |
| ローカルPDF | 確認・要約後に削除済み |
| 一時的なAI基盤参考資料 | Repository用Governanceへ変換後に削除済み |

Marker scanは基本的な予防策であり、任意の内容にsecretがないことの証明ではない。外部公開の前には、毎回staged changeをreviewする。

GitHubのpush protectionは、**検知を保証しない**。2026-07-29のread-back結果は次のとおりである。
このread-backで確認できるのは**repository levelの設定だけ**であり、
organization／enterprise levelのcustom patternは未確認である（`TBD`）。

```text
secret_scanning                        enabled
secret_scanning_push_protection        enabled
secret_scanning_non_provider_patterns  disabled
secret_scanning_validity_checks        disabled
```

`secret_scanning_non_provider_patterns`は**repository levelで無効**である。
このread-backだけでは、Wi-Fi PSK、社内APIのendpoint、独自形式の鍵といった
特定のsecret形式が**検知対象外だとは判断しない**。organization／enterprise levelの
custom patternが有効であれば検知されうるが、その有無をこのrepositoryのAPIからは
確認できないためである。provider patternの対象範囲もGitHub側の更新で変わりうる。

したがって、検知範囲は「repository levelのnon-provider patternが無効」までを
確定事実とし、それより広い範囲は`TBD`として扱う。より広い主張を記録する前に、
権限のあるscopeでorganization／enterprise levelのcustom patternをread-backする。

この値はrepository levelのnon-provider patternの状態だけを示す。
custom patternはrepository、organization、enterpriseのいずれでも定義でき、
それぞれpush protectionを個別に有効化できる。2026-08-01時点で
`GET /repos/{owner}/{repo}/secret-scanning/custom-patterns`は
`404 Feature not available in this repository`を返し、repository levelの
custom patternは利用できない。organizationとenterprise levelの定義有無は
このrepositoryのAPIからは確認できず、未確認である。

したがって`.gitignore`とstaged diffのreviewを省略しない。
検知pattern構成を変更した場合は、この記録を更新する。

### 実行環境（sanitized）

| 項目 | 値 |
|---|---|
| 実施日 | 2026-07-29 |
| 確認内容 | GitHub security設定のread-back（`secret_scanning`を含む4項目） |
| 端末profile | Docs / Review（[Machine Profiles](../toolchains/machine-profiles.md)） |
| 使用tool | `gh` 2.76.0、`git` 2.44.0、PowerShell 7.6.3 |
| 対象 | `wachi-yoshitaka-11-dev/deskcat` |

| 項目 | 値 |
|---|---|
| 実施日 | 2026-08-01 |
| 確認内容 | secret scanning custom patternのread-back（`GET /repos/{owner}/{repo}/secret-scanning/custom-patterns`） |
| 端末profile | Docs / Review（[Machine Profiles](../toolchains/machine-profiles.md)） |
| 使用tool | `gh` 2.76.0、`git` 2.44.0、PowerShell 7.6.3 |
| 対象 | `wachi-yoshitaka-11-dev/deskcat` |
| 結果 | `404 Feature not available in this repository`。repository levelのcustom patternは利用できない。organization／enterprise levelの定義有無はこのscopeでは確認できず未確認 |

2つのread-backは実施日が異なるため、記録を分ける。1つにまとめると、
どの確認がどの日の実行結果か辿れなくなる。

端末名、個人path、token、shell履歴は記録しない。
この記録は実行済みの確認結果であり、候補値や未実行の予定を含まない。


## 訂正記録

| 日付 | 対象 | 内容 |
|---|---|---|
| 2026-07-28 | `.env`がignore対象か | 当初「Yes」と記載していたが、`.gitignore`に該当規則が存在せず事実と異なっていた。`git check-ignore .env`が非0を返すことを確認したうえで`.gitignore`へ秘密情報の規則群を追加し、再確認して「Yes」とした。記載時に実行確認を行わなかったことが原因である。 |

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
