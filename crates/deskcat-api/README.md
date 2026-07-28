# deskcat-api

任意のcloud文章生成とlocal fallbackフレーズの境界として計画している。

責務:

- 文章生成器のtrait
- timeoutとrate limit
- 内容と長さの制限
- 決定的なlocal fallback
- 秘密情報を安全に扱う設定

DeskCatの最小local動作は、cloudが利用可能であることに依存させない。
