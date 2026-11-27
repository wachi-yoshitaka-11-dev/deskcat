#!/usr/bin/env python3
"""CodeRabbitのreviewを起動するcommandを、規則に反するものは`deny`、それ以外は人間の許可を求める形で止める。

Claude CodeのPreToolUse hookとして、Bash tool呼び出しの前に走る。stdinからhookの
入力JSONを読み、`tool_input.command`だけを見る。実行はしない。

**止めるのはreviewを起動するものだけである。**判定は「`@coderabbitai`と同じ行に
`review`という語があるか」の1点で入る。**枠を消費する操作の名前はどれも`review`を含む。**
`rate limit`、`resolve`、`help`は素通りする。**免除listを持たせると、`rate limit`で
始めて後ろに`full review`を置く形で抜ける**（実測した）。

**返す判定は3段である**（#781）。

1. **`full`の付かない`review`は`deny`。**CONTRIBUTINGの「手動で依頼する前に状態を確認する」は
   初回を`full review`と定め、`review`は新しい変更だけを見るため、reviewがまだ走っていない
   PRでは空振りして枠だけ消える。**`full`の付かない`review`が正しい場面は無い**
   （再依頼の例外も`full review`である）。人間の依頼も`full review`と書けば通る。
2. **対象PRでCodeRabbitのreviewが既に走っているなら、`full review`も`deny`。**完走
   （`Review completed`）、または進行中（`Review in progress`）のcommitが1つでもあれば走っている。
   **例外は、走っていない原因がcheckの説明文の`Review rate limited`か
   `Review stopped after lock loss`だけのとき**で、この場合は下の`ask`に戻る。
3. **それ以外の`full review`は`ask`。**対象のPR番号やcheckの状態を判定できないとき
   （`gh`の失敗、graphqlなど宛先を読めない`gh api`、本文を読めない経路）も`ask`である。
   **判定できないことを素通りにしない。**

**`ask`は人間に届くか**（#781。2026-11-20に公式資料で確認した）。
https://code.claude.com/docs/en/permission-modes は、hookによる`ask`には
「auto modeでもpromptを出す」と書く（hookの`ask`にはclassifierが関与しない）。
https://code.claude.com/docs/en/hooks は、auto modeのhookの`ask`は
「通常はpermission promptへ送られ、classifierは決めない」と書き、promptに答える人がいない
`-p`の実行では、Claude Codeが呼び出しを拒否してreasonをClaudeに返すと書く。
**資料の上では、auto modeの`ask`は人間に届く。**2026-11-19に`ask`をすり抜けた3回の原因は
資料からは説明できず、**未確認のままである。**`dontAsk`と`bypassPermissions`での`ask`の扱いは、
読んだ範囲に記述が無い。**そのため、`ask`に頼る範囲を最小にした。**規則に反すると
字句と記録で決まるもの（上の1と2）は`deny`にし、人間の判断が要るもの（`full review`の初回、再依頼の条件に当たりうる場合、
判定できない場合）と、本文を読めず判定できない経路だけを`ask`にする。**確認のために実際に`@coderabbitai`へ投稿はしていない。**

**環境変数によるバイパスを設けていない。**他のhookの`SKIP_ENV`に当たるものが無いのは
意図である。**AIが自分で外せる抑制は抑制ではない。**この判断は#240で人間が決めた。

**2026-08-28に、承認を経ずに通る経路を3つ塞いだ**（PR #254のreviewで出た）。
`-X GET -X POST`の後勝ち、`--input <file>`、`-F key=@<path>`。**3つとも実測で再現した。**

**2026-08-29に、本文を読めない場合の扱いを素通りからaskへ変えた。**
**`-F body=@-`は意図して選べる経路であり、外したい側が自分で選べる穴だった。**
誤検知の代償は人への確認1回で、見逃しの代償はこのhookが在る理由そのものである。

なぜ必要か。**AIはこの型の失敗を繰り返した。**

1. #153（1 file、+2 −1）に対し、誰も頼んでいないreviewを取りに行き`rate limit`を投げた
2. PR #146／#148／#150。待ち時間を告げられているのに`rate limit`を約60秒ごとに
   投げ続け、**34件**のゴミコメントを積んだ
3. PR #238／#239。承認なしに`full review`を連投し、2本目が`Review rate limited`で
   空振りして安全に関わる変更のreviewが59分止まった
4. 2026-11-19。規則を開かずに`@coderabbitai review`を3回投げた（#781）。うち2回は
   reviewが走った後の直しの commit、1回は初回で空振りした。

**AI側の記憶には禁止規則が既にあり、すべて破った。**記憶は参照するかどうかが
AIの判断に依存する。**同じ型を記憶では止められていない。**そのため機構で止める。

**判定は字句と、対象PRのcheck-runの状態で行う。**意味は判定しない。
GitHubの読み方と、読めないときに通さない扱いは
`gh_metadata_guard.py`の完走の判定に揃えた。
"""

import json
import re
import subprocess
import sys
import time
from concurrent.futures import ThreadPoolExecutor, as_completed
from concurrent.futures import TimeoutError as FuturesTimeout

import command_line
import gh_metadata_guard as guard

# `@coderabbitai`を呼ぶ本文が渡されうるsubcommand。`gh`の呼び出しに続く2語で見る。
# `gh api`は1語だが、`gh api ... -f body=@coderabbitai ...`の形で投げられるため含める。
COMMENT_SUBCOMMANDS = (("pr", "comment"), ("issue", "comment"), ("pr", "review"))
API_SUBCOMMAND = ("api",)

# `gh api`で読み取りと明示されたmethod。**この場合はcommentを投げられない。**
# `-f`を付けると`gh`は既定でPOSTになるが、`-X GET`を明示すると読み取りのままである。
# **endpointのwhitelistは作らない。**`gh api graphql`のmutationでもcommentは投げられ、
# **endpoint名からは判別できない。**methodだけで絞る。
READ_ONLY_METHODS = ("get", "head")
METHOD_OPTIONS = ("-X", "--method")

# 止める対象。**`review`という語が現れたら止める。**
# `full review`は完全なreview、`review`はincrementalで、**どちらも枠を1件消費する。**
# incrementalはADR-0013で自動reviewを無効にしているこのrepositoryでは必ず空振りするが、
# **空振りでも枠は消える**（2026-08-24、PR #191で実際に焼いた）。
#
# **語の並びを列挙するのではなく、`review`の1語だけを見る。**列挙すると
# `full-review`のような表記の揺れで抜ける。**枠を消費する操作の名前は
# どれも`review`を含む**ため、これで足りる。
REVIEW_WORD = re.compile(r"\breview\b")

# 枠を消費しない操作。**別扱いにしていない。**いずれも`review`という語を含まないため、
# 上の判定で自動的に素通りする。**免除listを持たせると、`rate limit`で始めて
# 後ろに`full review`を置く形で抜けてしまう**（実測した）。
NOT_A_REVIEW = ("rate limit", "resolve", "help", "configuration", "status")

MENTION = "@coderabbitai"


def _note(message):
    """素通りさせた理由をstderrへ1行で残す。**判定は変えない。**

    hookのstdoutは`permissionDecision`のJSONに使うため、**診断はstderrへ出す。**
    素通りはこのhookの設計であって異常ではないが、**黙って通ると
    「止めているはず」と食い違ったことに気付けない。**
    """
    sys.stderr.write(f"coderabbit_gate: {message}\n")


class AskStop(Exception):
    """この呼び出しの判定が`ask`になった。**`main`が集め、`deny`を優先してから出す。**"""

    def __init__(self, reason):
        super().__init__(reason)
        self.reason = reason


def _ask(reason):
    """この呼び出しの判定を`ask`にする（人間へ許可を求める）。

    **`deny`ではない。**AIの独断を止めるのが目的であり、人間の依頼を塞ぐのではない。
    **その場で終了しない。**同じcommandの後ろの呼び出しに`deny`があれば`deny`を優先する
    （先の呼び出しの`ask`で抜けると、後ろの`deny`が`ask`に下がる）。
    """
    raise AskStop(reason)


def _emit_ask(reason):
    json.dump(
        {
            "hookSpecificOutput": {
                "hookEventName": "PreToolUse",
                "permissionDecision": "ask",
                "permissionDecisionReason": reason,
            }
        },
        sys.stdout,
    )
    sys.stdout.write("\n")
    raise SystemExit(0)


def _deny(reason):
    """PreToolUse hookとして、人間の許可も求めずに拒否して終了する。

    **規則に反すると字句と記録で決まるものだけに使う。**`ask`が人間に届かなかった
    と見られる事例があり（#781）、届かないなら`ask`はAIの判断で通る。
    """
    json.dump(
        {
            "hookSpecificOutput": {
                "hookEventName": "PreToolUse",
                "permissionDecision": "deny",
                "permissionDecisionReason": reason,
            }
        },
        sys.stdout,
    )
    sys.stdout.write("\n")
    raise SystemExit(0)


# `full review`。`full-review`の表記の揺れも`full`付きとして扱う（`ask`の側に残す）。
FULL_REVIEW = re.compile(r"\bfull[\s\-]+review\b")


def _review_kinds(text):
    """`@coderabbitai`と同じ行にある`review`の種類を返す。戻りは`{"full", "bare"}`の部分集合。

    **行単位で見る。**CodeRabbitへのcommandは`@coderabbitai <command>`の形で書き、
    1つのcommentに複数書ける。**本文全体で見ると、mentionと無関係な行の`review`まで
    拾う**（「reviewで出た指摘を反映した」という返信を止めてしまう）。
    **mentionの直後の1語だけで見ると、`rate limit`で始めて後ろに`review`を置く形で抜ける。**
    同じ行という範囲がその中間である。

    **`full review`を取り除いた後に`review`が残れば`bare`である。**
    `full review`と`review`が同じ行にあっても`bare`を落とさない。
    **止めすぎる側へ倒す。**
    """
    kinds = set()
    for line in text.lower().splitlines():
        if MENTION not in line:
            continue
        rest = line.split(MENTION, 1)[1]
        if not REVIEW_WORD.search(rest):
            continue
        if FULL_REVIEW.search(rest):
            kinds.add("full")
        if REVIEW_WORD.search(FULL_REVIEW.sub(" ", rest)):
            kinds.add("bare")
    return kinds


# 本文が載りうるoption。`gh api`は`-f body=...`を複数持てるため、**全件を見る。**
# `gh_metadata_guard.py`の`_option_value`は最初の1件だけを返すので使えない。
BODY_OPTIONS = ("--body", "-b", "-f", "--field", "--raw-field", "-F")
BODY_FILE_OPTIONS = ("--body-file",)

# `gh api`はbody全体をfileから読める（`-`はstdin）。**`BODY_OPTIONS`では拾えない。**
# 値がpathであって本文ではないためである。**塞ぐまでは`--input`に本文を置けば
# 素通りした**（2026-08-28に実測）。
INPUT_OPTIONS = ("--input",)

# `-F`／`--field`は値の`@<path>`をfileとして読む（`@-`はstdin）。
# **`-f`／`--raw-field`は展開せず、文字列のまま送る**（`gh api --help`で確認した）。
# そのため`@`の展開はこの2つの綴りにだけ当てる。**`-f body=@x`をfileとして読むと、
# 実際には送られない内容でaskを出すことになる。**
FIELD_FILE_OPTIONS = ("-F", "--field")


def _option_values(args, names):
    """`--name value`と`--name=value`の値を**全件**返す。

    短縮形の連結（`-fbody=...`）は拾わない。`gh`はこの形を受けるが、
    **拾おうとすると`-F`のような別optionの値まで巻き込む。**取りこぼす側へ倒す。
    """
    found = []
    index = 0
    while index < len(args):
        arg = args[index]
        for name in names:
            if arg == name:
                if index + 1 < len(args):
                    found.append(args[index + 1])
                index += 1
                break
            if arg.startswith(name + "="):
                found.append(arg[len(name) + 1:])
                break
        index += 1
    return found


def _read_body(path, source):
    """fileから本文を読む。戻りは`(本文のlist, 読めなかった理由のlist)`。

    **読めなかったことを呼び出し側へ返す。**以前は空を返して素通りさせていたが、
    **それは外したい側が自分で選べる穴だった**（`-F body=@-`と書けば検査を抜けられる）。
    扱いは`_check`が決める。

    **握りつぶさない。**分類してstderrへ残す
    （AGENTS.mdの「エラーを握りつぶさず、分類、ログ、カウンタを用意する」）。

    **`source`にどのoptionから来たかを渡す。**読める経路が3つ（`--body-file`／
    `--input`／`-F key=@path`）あるため、**どれが読めなかったかを書かないと
    記録から経路を辿れない。**
    """
    if path == "-":
        reason = f"`{source}`のstdin（`-`）から渡されており、hookからは読めない"
        _note(f"本文が{reason}")
        return [], [reason]
    try:
        with open(path, encoding="utf-8") as handle:
            return [handle.read()], []
    except OSError as error:
        reason = f"`{source}`の読み出しに失敗した: {error}"
        _note(f"{reason}ため判定できない")
        return [], [reason]


def _texts(args, is_api=False):
    """引数として渡された本文の候補を返す。

    戻りは`(本文のlist, 読めなかった理由のlist)`。

    **fileから読む経路は3つある**（`--body-file`／`--input`／`-F key=@path`）。
    **読めなかった経路を、読んで何も見つからなかったのと同じに扱わない。**
    以前は同じに扱っており、**`-F body=@-`と書けば検査を抜けられた。**
    """
    # `gh pr comment`／`gh issue comment`／`gh pr review`の`-F`は`--body-file`の短い形である
    # （`gh pr comment --help`で確認した）。`gh api`の`-F`はfieldである。
    body_options = BODY_OPTIONS
    body_file_options = BODY_FILE_OPTIONS
    if not is_api:
        body_options = tuple(o for o in BODY_OPTIONS if o != "-F")
        body_file_options = BODY_FILE_OPTIONS + ("-F",)
    found = _option_values(args, body_options)
    unreadable = []
    for option in (body_file_options, INPUT_OPTIONS):
        for path in _option_values(args, option):
            texts, reasons = _read_body(path, option[0])
            found.extend(texts)
            unreadable.extend(reasons)
    # `-F key=@path`は値をfileから読む。**`key=@path`という文字列のままでは
    # `@coderabbitai`を含まないため、読まないと素通りする**（2026-08-28に実測）。
    for value in _option_values(args, FIELD_FILE_OPTIONS if is_api else ()):
        _, separator, right = value.partition("=")
        if separator and right.startswith("@"):
            texts, reasons = _read_body(right[1:], FIELD_FILE_OPTIONS[0])
            found.extend(texts)
            unreadable.extend(reasons)
    return found, unreadable


# 走っていない原因が枠切れ・lock lossのときだけ、`full review`の再依頼を`deny`から外す
# （CONTRIBUTINGの「手動で依頼する前に状態を確認する」）。**checkの説明文で見分ける。**
RETRYABLE_TEXTS = ("review rate limited", "review stopped after lock loss")

# gh api の宛先。issue番号とPR番号は同じ番号空間にあるため`issues`も`pulls`も見る。
API_TARGET = re.compile(r"^(?:https://[^/]+/(?:api/v3/)?)?/?repos/([^/]+/[^/]+)/(?:issues|pulls)/(\d+)(?:/|$)")

CONTRIBUTING_SECTION = "CONTRIBUTINGの「手動で依頼する前に状態を確認する」"


_DEADLINE = None


class Undetermined(Exception):
    """対象のPRまたはcheckの状態を判定できなかった。**`ask`に倒す。**"""


def _target(subcommand, args, global_repo, deadline):
    """`(repo, PR番号)`を返す。読めなければ`Undetermined`。"""
    if subcommand == API_SUBCOMMAND:
        for arg in args:
            match = API_TARGET.match(arg)
            if match:
                return match.group(1), int(match.group(2))
        raise Undetermined("`gh api`の宛先からPR番号を読めない")
    selector = guard._merge_selector(args)
    repo_option = guard._repo_option(args, global_repo, guard.MERGE_VALUE_OPTIONS)
    view = ["pr", "view"] + ([selector] if selector else [])
    view += ["-R", repo_option] if repo_option else []
    view += ["--json", "number,url"]
    try:
        info = json.loads(guard._gh_json(view, deadline))
        match = re.match(r"https://[^/]+/([^/]+/[^/]+)/pull/\d+", info["url"])
        return match.group(1), int(info["number"])
    except (OSError, subprocess.SubprocessError, ValueError, KeyError,
            AttributeError, TypeError) as error:
        raise Undetermined(
            f"対象のPRを取得できない（{type(error).__name__}）"
        ) from error


def _commit_coderabbit(repo, sha, deadline):
    """commit 1つのCodeRabbitの記録を`[{"completed", "pending", "text"}]`で返す。"""
    runs = json.loads(guard._gh_json([
        "api", f"repos/{repo}/commits/{sha}/check-runs?per_page=100",
    ], deadline)).get("check_runs", [])
    runs = [run for run in runs if run.get("name") == guard.CODERABBIT_CHECK_NAME]
    if runs:
        return [
            {
                "completed": guard._run_review_completed(run),
                "pending": run.get("status") != "completed",
                "text": " ".join(
                    ((run.get("output") or {}).get("title") or "",
                     (run.get("output") or {}).get("summary") or "")
                ).lower(),
            }
            for run in runs
        ]
    statuses = json.loads(guard._gh_json([
        "api", f"repos/{repo}/commits/{sha}/statuses?per_page=100",
    ], deadline))
    # statusは履歴を全件、新しい順に返す。**進行中は最新の1件だけで見る**
    # （`Review in progress`の後に`Review rate limited`で止まった記録がある）。
    statuses = [
        item for item in statuses if item.get("context") == guard.CODERABBIT_CHECK_NAME
    ]
    return [
        {
            "completed": item.get("state") == "success"
            and (item.get("description") or "").startswith(guard.REVIEW_COMPLETED_TEXT),
            "pending": index == 0 and item.get("state") == "pending",
            "text": (item.get("description") or "").lower(),
        }
        for index, item in enumerate(statuses)
    ]


def _review_state(repo, number, deadline):
    """PRのCodeRabbitの状態を返す。`"ran"`／`"pending"`／`"retryable"`／`"none"`。

    **どのcommitで完走していてもranである。**対応commitのheadは`Review skipped`になる。
    読めなければ`Undetermined`。
    """
    try:
        shas = guard._gh_json([
            "api", "--paginate", f"repos/{repo}/pulls/{number}/commits?per_page=100",
            "--jq", ".[].sha",
        ], deadline).split()
        records = []
        pool = ThreadPoolExecutor(max_workers=guard.COMMIT_CHECK_WORKERS)
        try:
            futures = [
                pool.submit(_commit_coderabbit, repo, sha, deadline) for sha in shas
            ]
            for future in as_completed(
                futures, timeout=max(0, deadline - time.monotonic())
            ):
                records.extend(future.result())
        finally:
            # 走っているworkerは同じ期限で終わる。待たずに抜けると時間切れで判定が捨てられる。
            pool.shutdown(wait=True, cancel_futures=True)
    except (OSError, subprocess.SubprocessError, ValueError, KeyError,
            AttributeError, TypeError, FuturesTimeout) as error:
        raise Undetermined(
            f"CodeRabbitのcheckの状態を取得できない（{type(error).__name__}）"
        ) from error
    if any(record["completed"] for record in records):
        return "ran"
    if any(record["pending"] for record in records):
        return "pending"
    if any(word in record["text"] for record in records for word in RETRYABLE_TEXTS):
        return "retryable"
    return "none"


def _check_full_review(subcommand, args, global_repo):
    """`full review`の依頼を、対象PRの状態で`deny`か`ask`に振り分ける。"""
    label = " ".join(subcommand)
    # **hook全体で1つの期限を共有する**（最初の呼び出しで始める）。呼び出しごとに持つと、
    # `full review`が複数ある1つのcommandで合計がhookの30秒を超え、時間切れで判定が捨てられる
    # （`gh_metadata_guard.py`と同じ）。期限を過ぎた照会は`Undetermined`になり`ask`へ倒れる。
    global _DEADLINE
    if _DEADLINE is None:
        _DEADLINE = time.monotonic() + guard.CHECK_DEADLINE_SECONDS
    deadline = _DEADLINE
    try:
        repo, number = _target(subcommand, args, global_repo, deadline)
        state = _review_state(repo, number, deadline)
    except Undetermined as error:
        _ask(
            f"`{label}`で`{MENTION}`へ`full review`を投げようとしている。"
            f" **{error}ため、reviewが既に走ったかを判定できない。**"
            " 判定できないまま通さず、人間へ確認する。"
            f" 初回でなければ{CONTRIBUTING_SECTION}に従い投げ直さない。"
        )
    if state in ("ran", "pending"):
        _deny(
            f"`{label}`の対象 {repo}#{number} は、CodeRabbitのreviewが既に完走している"
            "（または進行中である）。"
            " **`full review`を投げ直さない。**"
            f" {CONTRIBUTING_SECTION}は、依頼したreviewが走った後の対応commitを"
            "自己レビューで見て、投げ直さないと定める。"
            " 再依頼してよいのは、checkの説明文が`Review rate limited`か"
            "`Review stopped after lock loss`でreviewが完走していないときだけである。"
        )
    reason_tail = (
        " 直前のcheckは枠切れかlock lossで、再依頼の条件に当たりうる。"
        " 間隔（全PRの最新投稿から1時間以上）と投稿直前の再確認を満たすか確かめる。"
        if state == "retryable"
        else " このPRではreviewがまだ走っていない。"
    )
    _ask(
        f"`{label}`で`{MENTION}`へ`full review`を投げようとしている。"
        " **CodeRabbitのreview枠は共有資源であり、AIの判断で消費してよいものではない。**"
        " 投げてよいか人間へ確認する。"
        + reason_tail
        + " 判断材料: 誰が求めたか／対象は新しい内容か／直前に同じ資源を使ったか／"
        "対象が複数なら重要度の高いものを1本ずつか。"
        f" 枠を消費しない{'／'.join(f'`{w}`' for w in NOT_A_REVIEW)}は素通りする。"
    )


def _check(subcommand, args, global_repo=None):
    texts, unreadable = _texts(args, is_api=subcommand == API_SUBCOMMAND)
    kinds = set()
    for text in texts:
        kinds |= _review_kinds(text)
    if "bare" in kinds:
        _deny(
            f"`{' '.join(subcommand)}`で`{MENTION}`へ`full`の付かない`review`を投げようとしている。"
            " **このrepositoryは自動reviewを行わない（ADR-0013）ため、`review`は新しい変更だけを"
            "見るため、reviewがまだ走っていないPRでは空振りし、枠だけ消費する。**"
            f" {CONTRIBUTING_SECTION}を開く。初回は`@coderabbitai full review`、"
            "依頼したreviewが走った後の対応commitには投げ直さない。"
        )
    # **読めない本文を足しても、読めた`full review`の`deny`は`ask`に下がらない。**
    # `_check_full_review`は判定できなければ自分で`ask`を返す。
    if "full" in kinds:
        _check_full_review(subcommand, args, global_repo)
    if unreadable:
        # **読めない本文を「reviewではない」と扱わない。**判定はaskであり、人が通せる。
        # **誤検知の代償は確認1回、見逃しの代償はこのgateが在る理由そのものである。**
        # `-F body=@-`は意図して選べる経路であり、**外したい側が自分で選べる穴だった。**
        _ask(
            f"`{' '.join(subcommand)}`の本文をhookから読めない"
            f"（{'／'.join(unreadable)}）。"
            f" **`{MENTION}`へreviewを投げようとしているかを判定できない。**"
            " CodeRabbitのreview枠は共有資源であり、AIの判断で消費してよいものではない。"
            " 読めない経路を「reviewではない」と扱うと、その経路が抑制を外す手段になる。"
            " 投げてよいか人間へ確認する。"
            " **reviewを含まない操作なら、本文をfileへ書いて`--body-file`で渡すと判定できる。**"
        )


def main():
    try:
        payload = json.load(sys.stdin)
    except (json.JSONDecodeError, UnicodeDecodeError) as error:
        # hookの入力を読めないことを、対象commandの問題として扱わない。
        # **素通りさせるが、握りつぶさない。**既存hookはここを黙って返すが、
        # `_note()`を用意した以上この経路だけ黙るのは筋が通らない
        # （PR #241のreview指摘）。**return codeとstdoutは変えない。**
        _note(f"hookの入力を読めなかったため判定できない: {error}")
        return 0
    command = command_line.command_from(payload)
    if command is None:
        # 妥当なJSONでもmappingでないことがある（`[]`／`{"tool_input": ["x"]}`）。
        # **判定は`command_line.command_from`が持つ。**5本のhookへ複製しない（#242）。
        _note("hookの入力からcommandを取り出せなかったため判定できない")
        return 0
    if "gh" not in command:
        # 全Bash呼び出しでこのhookが走る。`if`条件でBash(gh *)へ絞ると
        # `cd x && gh pr comment`が素通りするため、絞らずここで安く抜ける。
        return 0
    pending_ask = None
    for args in command_line.invocations(command, "gh"):
        # global optionを外す。理由は`gh_metadata_guard.py`と同じである（#325）。
        global_repo = (
            command_line.global_option_value(args, "gh", "--repo")
            or command_line.global_option_value(args, "gh", "-R")
        )
        args = command_line.skip_global_options(args, "gh")
        head = tuple(args[:2])
        try:
            _dispatch(head, args, global_repo)
        except AskStop as stop:
            # 最初の`ask`を残して次の呼び出しも見る。`deny`はその場で終了する。
            pending_ask = pending_ask or stop.reason
    if pending_ask:
        _emit_ask(pending_ask)
    return 0


def _dispatch(head, args, global_repo):
    """1つの`gh`呼び出しを検査する。`ask`は`AskStop`、`deny`は終了で返す。"""
    if head in COMMENT_SUBCOMMANDS:
        _check(head, args[2:], global_repo)
    elif tuple(args[:1]) == API_SUBCOMMAND:
        rest = args[1:]
        methods = [m.lower() for m in _option_values(rest, METHOD_OPTIONS)]
        # **最後の`-X`が効く。**`gh`のmethodは単一の文字列option
        # （`gh api --help`の`-X, --method string`）であり、複数回渡すと後勝ちになる。
        # **`any()`で見ると`-X GET -X POST`が読み取り扱いになって素通りする**
        # （2026-08-28に実測）。**渡されていなければ見る側へ倒す。**
        if methods and methods[-1] in READ_ONLY_METHODS:
            # 読み取りと明示されている。**commentは投げられない。**
            return
        _check(API_SUBCOMMAND, rest, global_repo)


if __name__ == "__main__":
    raise SystemExit(main())
