---
name: deskcat-preflight
description: DeskCatの作業で、着手前・push前・merge前・merge後に機械で確かめられることを実行する。規則そのものはCONTRIBUTINGが正本であり、このskillは実行だけを持つ。作業指示書へ定型を書き写す代わりに使う。
---

# DeskCat preflight

**このskillは規則を持たない。実行だけを持つ。**

規則の正本は
[CONTRIBUTING](https://github.com/wachi-yoshitaka-11-dev/deskcat/blob/main/CONTRIBUTING.md)と
[AGENTS.md](https://github.com/wachi-yoshitaka-11-dev/deskcat/blob/main/AGENTS.md)である。
**ここへ規則を書き写さない。**書き写すと同じ規則が3箇所になり、片方だけを見た判断が起きる。
それは
[作業指示書テンプレート](https://github.com/wachi-yoshitaka-11-dev/deskcat/blob/main/docs/governance/work-instruction-template.md)が
禁じていることそのものである。

**このskillが要る理由は、指示書へ定型を書き写す運用をやめたためである。**
消えた分を人の記憶に置くと踏まれる。実行できるものは実行へ移す。

## 着手前

```bash
git fetch origin && git rev-parse --short origin/develop
git rev-list --count HEAD..origin/develop
```

**2つ目が0でなければ、その差分だけ基点が古い。**先に揃える。
`.claude/settings.json`のhookが`git checkout -b`を止めるが、**既にあるbranchでは止まらない。**

対象Issueの受け入れ条件を開く。**「担当分が終わった」はIssueの完了ではない。**

```bash
gh issue view <番号> --json body --jq .body | grep -n "^\s*- \[[ x]\]"
```

## push前

```bash
python3 scripts/review_gate.py classify --base origin/develop --head HEAD
python3 scripts/test_review_gate.py && python3 scripts/test_hooks.py
python3 scripts/test_link_validators.py && python3 scripts/test_pages_guards.py
python3 scripts/test_instruction_entrypoint.py
python3 scripts/validate_doc_links.py && python3 scripts/validate_instruction_entrypoint.py
python3 scripts/prepare_pages.py && git status --short
```

**`git status`が汚れていたら`git restore pages/assets-manifest.json`。**

自己レビューの収束条件は
[CONTRIBUTINGの「自己レビュー」](https://github.com/wachi-yoshitaka-11-dev/deskcat/blob/main/CONTRIBUTING.md#自己レビュー)が持つ。
**回した round 数と各 round で出た件数を commit message へ書く。**
書けない round は回していないということである。

**追加commitをしたら回し直す。**merge commitも追加commitである。

## merge前

```bash
gh pr checks <番号>
gh api graphql -f query='query { repository(owner:"wachi-yoshitaka-11-dev", name:"deskcat") {
  pullRequest(number:<番号>) { reviewThreads(first:50) { nodes { isResolved } } } } }' \
  --jq '[.data.repository.pullRequest.reviewThreads.nodes[]|select(.isResolved==false)]|length'
```

**未解決threadが0件であること。**

**checkの色ではなく説明文を読む。**`pass`でもreviewが走っていない状態がある。
どの表示がどれに当たるかは
[GitHubが強制しないもの](https://github.com/wachi-yoshitaka-11-dev/deskcat/blob/main/CONTRIBUTING.md#githubが強制しないもの)が持つ。
**ここへ写さない。**

CodeRabbitへ投げる場合は、**先に残数を確認する。**

```text
@coderabbitai rate limit
```

**`review`ではなく`full review`。**使い分けと、投げてよい条件は
[手動で依頼する前に状態を確認する](https://github.com/wachi-yoshitaka-11-dev/deskcat/blob/main/CONTRIBUTING.md#手動で依頼する前に状態を確認する)が持つ。

**人間の承認を得る。CIが緑でも機械reviewが完走しても承認ではない。**

## merge時

`--subject`と`--body-file`を明示する。**messageを渡さないとGitHubが合成したmessageになり、
trailerがsquash commitへ入らない。**`18298ae`と`619c843`で実際に起きた。

**`Refs`をtrailerとして使う場合はコロンが要る**（`Refs: #204`）。
**コロンの無い行をtrailer blockと同じ段落へ置くと、blockごと無効になる。**

## merge後

```bash
git fetch origin && git log -1 --format=%B origin/develop | git interpret-trailers --parse
python3 scripts/review_gate.py history --base origin/main --head origin/develop
```

**`PostToolUse` hookが自動で確認するが、直接commitでは走らない。**

Issueをcloseしたら、boardの`Target date`をJSTのclose実績日へ設定する。

## 直接commitしてよい範囲

`CLASS=minor`、または`Change-Class: fixup`と`Refs: #<番号>`。

**範囲の上限を、ここへ写さない。**
[後始末（`fixup`）の範囲](https://github.com/wachi-yoshitaka-11-dev/deskcat/blob/main/CONTRIBUTING.md#後始末fixupの範囲)を
**その都度開く。**対象外の列挙は変わりうる。写すと片方だけを見た判断が起きる。

## このskillを長くしない

**長くした瞬間に、指示書の定型と同じ問題が起きる。**読む側が飛ばすようになり、
飛ばす習慣がその中の1行にも及ぶ。

**規則を足したくなったらCONTRIBUTINGへ足す。ここへ書き写さない。**
