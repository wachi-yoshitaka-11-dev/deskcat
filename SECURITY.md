# Security方針

## 対応version

DeskCatは、対応対象となるfirmwareまたはhost softwareのversionをまだreleaseしていない。

現在のsecurity修正はdefault開発branchを対象とする。version付きreleaseの開始時にこの方針を更新する。

## 脆弱性の報告

次を含むpublic Issueを作成しない。

- 資格情報、鍵、token
- 秘密情報や外部serviceを露出する再現可能な脆弱性
- 危険な物理動作を起こせるfirmware command経路
- サーボまたは電源安全制限の迂回方法
- privateなユーザーデータ

推奨する報告経路:

1. このrepositoryで有効なGitHub Private vulnerability reportingを使用する。
2. private reportを利用できない場合は、脆弱性の詳細を含まないpublic Issueで、非公開の連絡方法をmaintainerへ問い合わせる。

非公開報告には次を含める。

- 影響を受けるcomponentとversion／commit
- 影響
- 再現手順
- 必要なハードウェア
- 秘密情報を除去したlog
- 判明している場合は緩和案

## ハードウェア安全報告

予期しない動作、過熱、繰返しbrownout、電気的損傷、安全制限の迂回は、securityと安全の両面で機微な情報である。

追加再現の前に次を行う。

1. actuator電源を切る。
2. logとreset reasonを保存する。
3. 秘密情報や危険な正確なcommandを公開しない。
4. ハードウェアと配線revisionを記録する。
5. 意図的に制限した追加testを調整する。

## 秘密情報の露出

資格情報をcommitまたは投稿した場合:

1. 直ちにrevokeまたはrotateする。
2. file削除だけで履歴やcacheから消えると考えない。
3. logと外部serviceの活動を確認する。
4. 履歴cleanupは別作業として調整する。

実際の秘密情報をIssue、pull request、test fixture、serial log、AI promptへ送らない。
