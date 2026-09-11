#!/usr/bin/env python3
"""`hooks/`配下の回帰test。

hookを子processとして起動し、stdinへhookの入力JSONを渡して、**拒否したか通したか**と
診断文を検査する。基点の検査はfixture repositoryを作って実際に`git fetch`させる。

**主に見るのは、素通りしないことである。**guardが黙って通るようになると、
止めているはずの失敗（boardへのitem追加漏れ、trailerの引き継ぎ漏れ、古い基点）が
また起きる。**そしてhookは失敗しても静かなため、壊れたことに気付けない。**
"""

import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

SCRIPTS_ROOT = Path(__file__).resolve().parent
sys.path.insert(0, str(SCRIPTS_ROOT / "lib"))

import publish_guards as guards  # noqa: E402

sys.path.insert(0, str(SCRIPTS_ROOT))

import review_gate as gate  # noqa: E402

sys.path.insert(0, str(SCRIPTS_ROOT / "hooks"))

import command_line  # noqa: E402

GH_GUARD = str(SCRIPTS_ROOT / "hooks" / "gh_metadata_guard.py")
BASE_GUARD = str(SCRIPTS_ROOT / "hooks" / "branch_base_guard.py")
MERGE_REPORT = str(SCRIPTS_ROOT / "hooks" / "merge_trailer_report.py")
PUSH_GATE = str(SCRIPTS_ROOT / "hooks" / "push_gate.py")
CODERABBIT_GATE = str(SCRIPTS_ROOT / "hooks" / "coderabbit_gate.py")
INSPECTOR_GUARD = str(SCRIPTS_ROOT / "hooks" / "inspector_readonly_guard.py")
MERGE_REPORT = str(SCRIPTS_ROOT / "hooks" / "merge_trailer_report.py")
REPO_ROOT_FOR_TEMPLATES = SCRIPTS_ROOT.parent
ISSUE_TEMPLATE_DIR = REPO_ROOT_FOR_TEMPLATES / ".github" / "ISSUE_TEMPLATE"
PR_TEMPLATE_PATH = REPO_ROOT_FOR_TEMPLATES / ".github" / "pull_request_template.md"

sys.path.insert(0, str(SCRIPTS_ROOT / "hooks"))

import push_gate  # noqa: E402

import inspector_readonly_guard  # noqa: E402

# fixtureのcommitに使うidentity。実行者の設定に依存させない。
GIT_IDENTITY = (
    "-c", "user.name=test",
    "-c", "user.email=test@example.invalid",
    "-c", "commit.gpgsign=false",
)


def _template_sections(path):
    """正本templateから`## `見出しの並びを読む。hook側の抽出方法を複製しない。

    見出し名はhookの実装（`_section_headings`）と同じ正規表現で取る。
    testがhookのロジックと違う方法で見出しを数えると、hookが見ていないずれを
    testが見落とす。
    """
    text = Path(path).read_text(encoding="utf-8")
    return [heading.strip() for heading in re.findall(r"(?m)^## (.+)$", text)]


def _body_with_sections(headings):
    """指定した見出しだけを持つ本文を作る。中身は節が空でなければ何でもよい。"""
    return "\n".join(f"## {heading}\nx\n" for heading in headings)


def _git(root, *arguments):
    result = subprocess.run(
        ["git", "-C", root, *GIT_IDENTITY, *arguments],
        capture_output=True, text=True, encoding="utf-8", errors="replace",
    )
    if result.returncode != 0:
        raise RuntimeError(
            f"git {' '.join(arguments)} failed: {result.stdout}{result.stderr}"
        )
    return result.stdout


def _invoke(script, command, cwd=None, environment=None):
    """hookへcommandを渡し`(exit code, 解釈した出力)`を返す。

    出力が空のときは`None`を返す。**空は「通した」を意味する。**
    """
    payload = json.dumps({"tool_name": "Bash", "tool_input": {"command": command}})
    env = dict(os.environ)
    # 呼び出し側の環境に無効化用の変数が残っていても、testの前提を壊さない。
    env.pop("DESKCAT_SKIP_GH_GUARD", None)
    env.pop("DESKCAT_SKIP_BASE_GUARD", None)
    env.pop("DESKCAT_SKIP_PUSH_GATE", None)
    env.pop("DESKCAT_SKIP_TRUNCATION_GUARD", None)
    if environment:
        env.update(environment)
    result = subprocess.run(
        [sys.executable, script],
        input=payload, capture_output=True, text=True,
        encoding="utf-8", errors="replace", cwd=cwd, timeout=120, env=env,
    )
    text = result.stdout.strip()
    return result.returncode, (json.loads(text) if text else None)


def _reason(output):
    return output["hookSpecificOutput"]["permissionDecisionReason"]


class GhMetadataGuardTests(unittest.TestCase):
    """`gh`の必須metadataを見るhookのtest。"""

    def assertDenied(self, command, *, contains=None):
        code, output = _invoke(GH_GUARD, command)
        self.assertEqual(code, 0, command)
        self.assertIsNotNone(output, f"通してしまった: {command}")
        self.assertEqual(
            output["hookSpecificOutput"]["permissionDecision"], "deny", command
        )
        if contains:
            self.assertIn(contains, _reason(output))

    def assertAllowed(self, command):
        code, output = _invoke(GH_GUARD, command)
        self.assertEqual(code, 0, command)
        self.assertIsNone(output, f"止めてしまった: {command}")

    def test_create_without_project_is_denied(self):
        """`--project`が無いIssue／Pull Request作成を止める。"""
        for command in (
            "gh pr create --title t --body b",
            "gh issue create --title t",
        ):
            with self.subTest(command=command):
                self.assertDenied(command, contains="--project")

    def test_create_with_project_is_allowed(self):
        """`--project value`と`--project=value`のどちらも通す。

        `gh pr create`側の`--base`は、この検査の前提を満たすために足しているだけである。
        **見ている対象は`--project`のままである。**
        """
        for command in (
            "gh pr create --title t --project deskcat --base develop",
            "gh pr create --title t --project=deskcat --base develop",
            "gh issue create --title t --project deskcat",
        ):
            with self.subTest(command=command):
                self.assertAllowed(command)

    def test_short_project_option_is_accepted(self):
        """`--project`の短縮形`-p`を拾う。

        `gh issue create`と`gh pr create`はどちらも`-p, --project`を持つ。
        **長い形だけを見ると`-p deskcat`を誤って拒否する。**誤検知は、hookそのものを
        無効化される側の失敗である。
        """
        for command in (
            "gh pr create --title t -p deskcat --base develop",
            "gh issue create --title t -p deskcat",
            "gh pr create --title t -pdeskcat --base develop",
            "gh pr create --title t -p=deskcat --base develop",
        ):
            with self.subTest(command=command):
                self.assertAllowed(command)

    def test_long_options_are_not_matched_by_the_short_form(self):
        """`-p`の連結判定を、`--p`で始まる長い option へ当てない。"""
        for command in (
            "gh pr create --title t --paginate",
            "gh pr create --title t --draft",
        ):
            with self.subTest(command=command):
                self.assertDenied(command)

    def test_pr_create_without_base_is_denied(self):
        """`--base`が無いPull Request作成を止める。

        **`gh`は省略時にrepositoryのdefault branchを使い、このrepositoryのdefaultは
        `main`である。**2026-08-28にPR #250がbase `main`で作られ、baseの変更まで
        1時間17分かかった。**機構は何も止めなかった。**
        """
        for command in (
            "gh pr create --title t --project deskcat",
            "gh pr create --title t -p deskcat --draft",
        ):
            with self.subTest(command=command):
                self.assertDenied(command, contains="--base")

    def test_pr_create_with_base_is_allowed(self):
        """`--base value`／`--base=value`／短縮形`-B`のいずれも通す。

        **短縮形は大文字の`-B`である**（`gh help pr create`で確認した）。
        """
        for command in (
            "gh pr create --title t -p deskcat --base develop",
            "gh pr create --title t -p deskcat --base=develop",
            "gh pr create --title t -p deskcat -B develop",
            "gh pr create --title t -p deskcat -Bdevelop",
        ):
            with self.subTest(command=command):
                self.assertAllowed(command)

    def test_issue_create_does_not_require_base(self):
        """`gh issue create`へ`--base`を要求しない。

        **`gh issue create`の option 一覧に`--base`は存在しない**
        （`gh help issue create`で確認した。出現0件）。要求すると、満たしようのない
        条件で起票が止まる。
        """
        self.assertAllowed("gh issue create --title t --project deskcat")

    def test_base_short_form_is_case_sensitive(self):
        """`-b`（`--body`）を`-B`（`--base`）と読まない。

        `gh pr create`は両方を持ち、意味が違う。**大小を区別しないと、本文を渡した
        だけのcommandがbase指定として通る。**
        """
        for command in (
            "gh pr create --title t -p deskcat -b 本文",
            "gh pr create --title t -p deskcat --body 本文",
        ):
            with self.subTest(command=command):
                self.assertDenied(command, contains="--base")

    def test_base_value_is_not_judged(self):
        """**`--base`の値の正しさを見ない。**存在するかだけを見る。

        `develop`と書くべきか`main`と書くべきかは、そのPull Requestの目的で決まり、
        字句からは読めない。昇格Pull Requestのbaseは`main`が正しい。
        """
        for command in (
            "gh pr create --title t -p deskcat --base main",
            "gh pr create --title t -p deskcat --base 実在しないbranch",
        ):
            with self.subTest(command=command):
                self.assertAllowed(command)

    def test_help_requires_no_metadata(self):
        """helpの表示だけを求める呼び出しへ、`--project`も`--base`も要求しない。

        **何も作らない呼び出しである。**boardへのitem追加漏れは起きようがない。
        **ここを拒否すると、hookが要求しているoption名を`--help`で調べる手段そのものが
        塞がる。**2026-08-28に`gh issue create --help`が実際に拒否され、
        `gh help issue create`へ回避してoption一覧を確認した。
        **規則を守るために要る情報を、規則が隠している状態だった。**
        """
        for command in (
            "gh pr create --help",
            "gh issue create --help",
            "gh pr create -h",
            "gh issue create -h",
            "gh pr create -p deskcat --help",
        ):
            with self.subTest(command=command):
                self.assertAllowed(command)

    def test_help_needs_no_merge_message(self):
        """`gh pr merge --help`へsquash messageを要求しない。

        **helpの表示はmergeしない。**この検査でとりわけ効く。`--subject`と
        `--body-file`の明示をCONTRIBUTINGが要求しているのに、**そのoption名を
        `--help`で確認できなかった**（2026-08-28に実測。拒否された）。
        """
        for command in (
            "gh pr merge --help",
            "gh pr merge -h",
        ):
            with self.subTest(command=command):
                self.assertAllowed(command)

    def test_help_is_matched_only_as_a_whole_word(self):
        """`-h`の連結判定をしない。`-help`や`-hello`をhelpと読まない。

        `_has_option`の連結判定を当てると、`-h`で始まる語がすべてhelpになる。
        **boolean flagに連結形は無い。**
        """
        for command in (
            "gh pr create --title t -hello",
            "gh issue create --title t -help",
        ):
            with self.subTest(command=command):
                self.assertDenied(command, contains="--project")
        self.assertDenied("gh pr merge 1 --squash -hello", contains="本文")

    def test_compound_command_is_inspected(self):
        """`cd x && gh pr create`を見落とさない。

        `if`条件で`Bash(gh *)`へ絞ると、この形が素通りする。絞らない理由である。
        """
        self.assertDenied("cd /tmp && gh pr create --title t")

    def test_absolute_path_invocation_is_inspected(self):
        """`/usr/local/bin/gh`のような絶対pathでの呼び出しも見る。"""
        self.assertDenied("/usr/local/bin/gh pr create --title t")

    def test_option_after_separator_is_not_credited(self):
        """区切りより後ろの語を、直前の`gh`の引数として数えない。"""
        self.assertAllowed("gh pr view 1 && echo --project")

    def test_merge_without_message_is_denied(self):
        """messageを渡さないmergeを止める。GitHubが合成したmessageにはtrailerが入らない。"""
        self.assertDenied("gh pr merge 1 --squash", contains="18298ae")

    def test_merge_with_unreadable_message_is_denied(self):
        """本文を読めない経路を「入っている」と扱わない。"""
        self.assertDenied("gh pr merge 1 --squash -F -", contains="stdin")
        self.assertDenied(
            "gh pr merge 1 --squash --body-file /nonexistent/body.txt",
            contains="読み出しに失敗",
        )

    def test_merge_message_trailers_are_checked(self):
        """squash messageのtrailerを、fileとinlineの両方で見る。"""
        with tempfile.TemporaryDirectory() as directory:
            complete = Path(directory) / "complete.txt"
            complete.write_text(
                f"body\n\n{gate.TRAILER_CLASS}: x\n{gate.TRAILER_REVIEW}: y\n",
                encoding="utf-8",
            )
            partial = Path(directory) / "partial.txt"
            partial.write_text(
                f"body\n\n{gate.TRAILER_REVIEW}: y\n", encoding="utf-8"
            )
            self.assertAllowed(f"gh pr merge 1 --squash --body-file {complete}")
            self.assertDenied(
                f"gh pr merge 1 --squash --body-file {partial}",
                contains=gate.TRAILER_CLASS,
            )
        self.assertAllowed(
            "gh pr merge 1 --squash --body "
            f'"x\n\n{gate.TRAILER_CLASS}: c\n{gate.TRAILER_REVIEW}: s"'
        )
        self.assertDenied('gh pr merge 1 --squash --body "本文だけ"')

    def test_merge_message_must_parse_as_trailers(self):
        """**`git`がtrailerとして読める形かを見る。文字列の一致では足りない。**

        `c171c52`（[PR #307](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/307)）は、
        trailer blockの直前へ空行なしでコロンの無い`Closes #304`を置いた。
        `git interpret-trailers`は最後の段落がすべてtrailerでないとその段落を
        認めないため、`Change-Class`・`Self-Review`・`Instruction-Change`・`Refs`が
        まるごと無効になり、`develop`のsquash commitは宣言を1つも持たなかった。

        **このhookは通していた。**当時の判定が`f"{name}:" not in text`という
        部分一致で、**文字列としては存在した**ためである（`c171c52`では3種類とも）。
        下のfixtureも同じ性質を持たせてあり、`assertIn`が固定しているのはそこである。
        **部分一致へ戻すとこのtestは落ちる。**
        """
        with tempfile.TemporaryDirectory() as directory:
            broken = Path(directory) / "broken.txt"
            broken.write_text(
                "本文である。\n\n"
                f"Closes #304\n{gate.TRAILER_CLASS}: c\n{gate.TRAILER_REVIEW}: s\n",
                encoding="utf-8",
            )
            separated = Path(directory) / "separated.txt"
            separated.write_text(
                "本文である。\n\nCloses #304\n\n"
                f"{gate.TRAILER_CLASS}: c\n{gate.TRAILER_REVIEW}: s\n",
                encoding="utf-8",
            )
            text = broken.read_text(encoding="utf-8")
            for name in (gate.TRAILER_CLASS, gate.TRAILER_REVIEW):
                self.assertIn(f"{name}:", text, "回帰testの前提が崩れている")
            self.assertDenied(
                f"gh pr merge 1 --squash --subject s --body-file {broken}",
                contains=gate.TRAILER_CLASS,
            )
            # 同じ宣言を空行で区切っただけのmessageは通る。**直し方が示されている。**
            self.assertAllowed(
                f"gh pr merge 1 --squash --subject s --body-file {separated}"
            )

    def test_merge_message_of_only_trailers_is_allowed(self):
        """**本文がtrailerだけの形を誤検知しない。**

        `gh pr merge`が作るcommit messageは`--subject`と本文を空行で連結した形に
        なる。**本文だけを`git`へ渡すと本文の先頭行がsubjectとして扱われ**、段落が
        1つしか無い形になってtrailerが0件と読まれる（2026-09-02に実測）。
        実際のcommitでは有効なtrailer blockであり、**止めれば正しいmergeが
        通らなくなる。**hookはsubjectの行を補ってから`git`へ渡す。
        """
        with tempfile.TemporaryDirectory() as directory:
            only = Path(directory) / "only.txt"
            only.write_text(
                f"{gate.TRAILER_CLASS}: c\n{gate.TRAILER_REVIEW}: s\n",
                encoding="utf-8",
            )
            self.assertAllowed(
                f"gh pr merge 1 --squash --subject s --body-file {only}"
            )

    def test_merge_is_denied_when_git_cannot_be_run(self):
        """**`git`を実行できない場合は通さない。**

        この検査が守っているのは「宣言がsquash commitへ入るか」であり、
        **入らなかったときの手当ては`DECLARATION_EXEMPT`への登録＝Pull Request
        1本である**（`c171c52`が実例）。確認できないまま通すほうが高くつく。
        **止まったときの逃げ道は`DESKCAT_SKIP_GH_GUARD`であり、診断文に書いてある。**
        それを検査しているのが最後の`assertIn`である。
        """
        with tempfile.TemporaryDirectory() as directory:
            body = Path(directory) / "body.txt"
            body.write_text(
                "本文である。\n\n"
                f"{gate.TRAILER_CLASS}: c\n{gate.TRAILER_REVIEW}: s\n",
                encoding="utf-8",
            )
            command = f"gh pr merge 1 --squash --subject s --body-file {body}"
            # 同じmessageが、gitを引ける状態では通ることを先に確かめる。
            self.assertAllowed(command)
            # `git`の無いPATHで起動する。hook自身は絶対pathのpythonで動くため、
            # PATHを空にしてもhookの起動そのものは妨げない。
            empty = Path(directory) / "bin"
            empty.mkdir()
            code, output = _invoke(GH_GUARD, command, environment={"PATH": str(empty)})
            self.assertEqual(code, 0)
            self.assertIsNotNone(output, "gitを実行できないまま通してしまった")
            self.assertEqual(
                output["hookSpecificOutput"]["permissionDecision"], "deny"
            )
            self.assertIn("DESKCAT_SKIP_GH_GUARD", _reason(output))

    def test_unrelated_commands_are_allowed(self):
        """`gh`以外と、`gh`の他のsubcommandは通す。"""
        for command in ("git status", "ls -la", "gh pr view 1 --json state"):
            with self.subTest(command=command):
                self.assertAllowed(command)

    def test_skip_environment_disables_the_guard(self):
        """逃げ道が効く。**効かない逃げ道は、hookごと無効化される。**"""
        code, output = _invoke(
            GH_GUARD, "gh pr create --title t",
            environment={"DESKCAT_SKIP_GH_GUARD": "1"},
        )
        self.assertEqual(code, 0)
        self.assertIsNone(output)

    def test_skip_environment_disables_the_body_section_check(self):
        """決定3の節検査も`DESKCAT_SKIP_GH_GUARD`で無効化される。

        `main()`の先頭で全検査を一括して止める既存の仕組みに乗っているだけだが、
        新設した検査が実際にその経路を通ることは別途確認する必要がある。
        （通常なら節が欠けてdenyされる本文で確認する。）
        """
        code, output = _invoke(
            GH_GUARD,
            'gh issue create --title t --project deskcat '
            '--label type:maintenance --body "## 背景\nx\n"',
            environment={"DESKCAT_SKIP_GH_GUARD": "1"},
        )
        self.assertEqual(code, 0)
        self.assertIsNone(output)

    def test_broken_input_does_not_block(self):
        """hookの入力やcommandが壊れていることを、対象commandの問題として扱わない。"""
        result = subprocess.run(
            [sys.executable, GH_GUARD], input="{ではないJSON",
            capture_output=True, text=True, encoding="utf-8", timeout=60,
        )
        self.assertEqual(result.returncode, 0)
        self.assertEqual(result.stdout.strip(), "")
        # 引用符が閉じていないcommandは、shell自身が落とす。ここで二重に報告しない。
        self.assertAllowed('gh pr create --title "閉じていない')

    # --- 決定3（#348）: `--body`系の本文がtemplateの節を欠いていないかの検査 ---

    def test_body_option_absent_is_not_checked(self):
        """`--body`系が無い呼び出しは節検査の対象外である。

        対話prompt（またはeditor）経由はtemplateを経由しており、迂回にならない。
        `--template`も`type:*` labelも無いのに通ることが、検査が本文の有無で
        分岐していることを示す。
        """
        self.assertAllowed("gh issue create --title t --project deskcat")

    def test_issue_body_with_all_sections_is_allowed_via_template_name(self):
        """`--template`（表示名）で指定したtemplateの全節が揃った本文を通す。"""
        sections = _template_sections(ISSUE_TEMPLATE_DIR / "maintenance_task.md")
        body = _body_with_sections(sections)
        self.assertAllowed(
            "gh issue create --title t --project deskcat "
            f'--template 保守作業 --body "{body}"'
        )

    def test_issue_body_with_all_sections_is_allowed_via_type_label(self):
        """`--template`が無くても、`type:*` labelから一意にtemplateを決めて通す。"""
        sections = _template_sections(ISSUE_TEMPLATE_DIR / "maintenance_task.md")
        body = _body_with_sections(sections)
        self.assertAllowed(
            "gh issue create --title t --project deskcat "
            f'--label type:maintenance --body "{body}"'
        )

    def test_issue_body_missing_sections_is_denied(self):
        """節が欠けたIssue本文を止め、欠けている節名を名指しする。"""
        sections = _template_sections(ISSUE_TEMPLATE_DIR / "maintenance_task.md")
        body = _body_with_sections(sections[:2])  # 先頭2節だけにする
        self.assertDenied(
            "gh issue create --title t --project deskcat "
            f'--label type:maintenance --body "{body}"',
            contains=sections[-1],
        )

    def test_pr_body_with_all_sections_is_allowed(self):
        """`gh pr create`はtemplateが1つだけなので、`--template`無しでも判定できる。"""
        sections = _template_sections(PR_TEMPLATE_PATH)
        body = _body_with_sections(sections)
        self.assertAllowed(
            "gh pr create --title t --project deskcat --base develop "
            f'--body "{body}"'
        )

    def test_pr_body_missing_sections_is_denied(self):
        """節が欠けたPull Request本文を止める。"""
        sections = _template_sections(PR_TEMPLATE_PATH)
        body = _body_with_sections(sections[:1])
        self.assertDenied(
            "gh pr create --title t --project deskcat --base develop "
            f'--body "{body}"',
            contains=sections[-1],
        )

    def test_undeterminable_template_is_denied(self):
        """`--template`も対応する`type:*` labelも無いIssue本文は、素通りさせない。

        **fail-openを選ばない。**存在検査ではなく内容検査へ変えた理由そのものが、
        「決められないなら安全側で止める」である。素通りは選択肢に無い。
        """
        self.assertDenied(
            'gh issue create --title t --project deskcat --body "## 背景\nx\n"',
            contains="決められない",
        )

    def test_ambiguous_type_labels_are_denied(self):
        """複数の`type:*` labelが異なるtemplateへ一致する場合も決められないとする。"""
        self.assertDenied(
            "gh issue create --title t --project deskcat "
            '--label type:bug --label type:maintenance --body "## 背景\nx\n"',
            contains="決められない",
        )

    def test_unknown_template_name_is_denied(self):
        """`--template`の値がどのtemplateの`name:`とも一致しない場合を通さない。"""
        self.assertDenied(
            "gh issue create --title t --project deskcat "
            '--template 存在しない名前 --body "## 背景\nx\n"',
            contains="--template",
        )

    def test_unreadable_create_body_is_denied(self):
        """`gh issue create`／`gh pr create`でも、本文を読めない経路を素通りさせない。

        [#267](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/267)で
        `coderabbit_gate.py`が同じ理由でfail-openをやめた前例と同じ扱いにする。
        """
        self.assertDenied(
            "gh issue create --title t --project deskcat -F -", contains="stdin"
        )
        self.assertDenied(
            "gh pr create --title t --project deskcat --base develop "
            "--body-file /nonexistent/body.txt",
            contains="読み出しに失敗",
        )

    def test_help_skips_body_check(self):
        """`--help`は`--body`が付いていても節検査をしない。何も作らないためである。"""
        self.assertAllowed(
            'gh issue create --title t --body "## 背景\nx\n" --help'
        )


class BranchBaseGuardTests(unittest.TestCase):
    """branchの基点を見るhookのtest。fixture repositoryへ実際にfetchさせる。"""

    def setUp(self):
        self.directory = tempfile.mkdtemp()
        self.addCleanup(guards.remove_tree, Path(self.directory))
        root = self.directory
        _git(root, "init", "--quiet", ".")
        (Path(root) / "note.md").write_text("最初\n", encoding="utf-8")
        _git(root, "add", "note.md")
        _git(root, "commit", "--quiet", "-m", "first")
        self.old = _git(root, "rev-parse", "HEAD").strip()
        (Path(root) / "note.md").write_text("最初\n進んだ\n", encoding="utf-8")
        _git(root, "commit", "--quiet", "-am", "second")
        # 自分自身をoriginにする。networkへ出ずに`git fetch origin`を成立させる。
        _git(root, "branch", "--force", "trunk", "HEAD")
        _git(root, "remote", "add", "origin", root)
        _git(
            root, "config", "remote.origin.fetch",
            "+refs/heads/trunk:refs/remotes/origin/develop",
        )
        _git(root, "fetch", "--quiet", "origin")

    def _at_old_base(self):
        _git(self.directory, "checkout", "--quiet", "-B", "work", self.old)

    def _at_trunk(self):
        _git(self.directory, "checkout", "--quiet", "-B", "work", "trunk")

    def assertDenied(self, command):
        code, output = _invoke(BASE_GUARD, command, cwd=self.directory)
        self.assertEqual(code, 0, command)
        self.assertIsNotNone(output, f"通してしまった: {command}")
        self.assertIn("遅れている", _reason(output))

    def assertAllowed(self, command):
        code, output = _invoke(BASE_GUARD, command, cwd=self.directory)
        self.assertEqual(code, 0, command)
        self.assertIsNone(output, f"止めてしまった: {command}")

    def test_stale_base_is_denied(self):
        """遅れた基点からのbranch作成を止める。`switch -c`と絶対pathも同じに扱う。"""
        self._at_old_base()
        for command in (
            "git checkout -b chore/1-x",
            "git switch -c chore/1-x",
            "git switch --create chore/1-x",
            "/usr/bin/git checkout -b chore/1-x",
        ):
            with self.subTest(command=command):
                self.assertDenied(command)

    def test_separator_does_not_look_like_a_start_point(self):
        """`git checkout -b x && echo done`の`echo`を基点と読まない。

        基点を明示したものと誤って読むと、検査を飛ばしてしまう。
        """
        self._at_old_base()
        self.assertDenied("git checkout -b chore/1-x && echo done")

    def test_current_base_is_allowed(self):
        """基点が`origin/develop`と一致していれば通す。"""
        self._at_trunk()
        self.assertAllowed("git checkout -b chore/1-x")

    def test_explicit_start_point_is_respected(self):
        """基点を明示しているなら、作成者が選んだものとして通す。"""
        self._at_old_base()
        self.assertAllowed("git checkout -b chore/1-x origin/develop")

    def test_hotfix_branch_is_out_of_scope(self):
        """`hotfix/`は`main`から作るのが正しい（ADR-0004）。"""
        self._at_old_base()
        self.assertAllowed("git checkout -b hotfix/1-x")

    def test_unrelated_git_commands_are_allowed(self):
        """branchを作らない操作は見ない。`git branch`もcheckoutを伴わない。"""
        self._at_old_base()
        for command in ("git status", "git branch chore/1-x", "git checkout trunk"):
            with self.subTest(command=command):
                self.assertAllowed(command)

    def test_skip_environment_disables_the_guard(self):
        """逃げ道が効く。"""
        self._at_old_base()
        code, output = _invoke(
            BASE_GUARD, "git checkout -b chore/1-x", cwd=self.directory,
            environment={"DESKCAT_SKIP_BASE_GUARD": "1"},
        )
        self.assertEqual(code, 0)
        self.assertIsNone(output)


TRUNCATION_GUARD = str(SCRIPTS_ROOT / "hooks" / "truncation_guard.py")


class TruncationGuardTests(unittest.TestCase):
    """列挙commandの明示的な切り詰めを見るhookのtest（決定2、#349）。"""

    def assertAsked(self, command, *, contains=None):
        code, output = _invoke(TRUNCATION_GUARD, command)
        self.assertEqual(code, 0, command)
        self.assertIsNotNone(output, f"通してしまった: {command}")
        self.assertEqual(
            output["hookSpecificOutput"]["permissionDecision"], "ask", command
        )
        if contains:
            self.assertIn(contains, _reason(output))

    def assertAllowed(self, command):
        code, output = _invoke(TRUNCATION_GUARD, command)
        self.assertEqual(code, 0, command)
        self.assertIsNone(output, f"止めてしまった: {command}")

    def test_piped_head_after_target_command_is_asked(self):
        """対象commandの出力を`head -N`／`tail -N`へ渡す形をaskする。"""
        for command in (
            "gh pr list | head -8",
            "gh issue list | tail -3",
            "git log | head -5",
            "git rev-list HEAD | head -1",
            "git branch | head -5",
            "gh api repos/x/y/issues | head -20",
        ):
            with self.subTest(command=command):
                self.assertAsked(command)

    def test_reason_names_the_detected_limit(self):
        """診断文が、検出した数値そのものを名指しする。"""
        self.assertAsked("gh pr list | head -8", contains="8")

    def test_head_tail_forms_are_all_detected(self):
        """`-N`／`-n N`／`-nN`／`--lines=N`のいずれの書き方も拾う。"""
        for command in (
            "gh pr list | head -8",
            "gh pr list | head -n 8",
            "gh pr list | head -n8",
            "gh pr list | head --lines=8",
        ):
            with self.subTest(command=command):
                self.assertAsked(command)

    def test_bare_head_without_a_number_is_allowed(self):
        """数の無い`head`／`tail`（既定10行）は対象外にする。

        **数が一切commandに現れない呼び出しまで拾うと、閲覧全般を止めることになる。**
        """
        self.assertAllowed("gh pr list | head")
        self.assertAllowed("gh issue list | tail")

    def test_head_tail_on_unrelated_command_is_allowed(self):
        """対象commandではない出力を`head`へ渡す形は見ない。"""
        self.assertAllowed("gh pr view 1 | head -8")
        self.assertAllowed("head -20 file.txt")
        self.assertAllowed("cat file.txt | head -8")

    def test_max_count_on_git_log_and_rev_list_is_asked(self):
        """`git log`／`git rev-list`の`-n`／`--max-count`をaskする。"""
        for command in (
            "git log -n 20",
            "git log -n20",
            "git rev-list --max-count=5 HEAD",
        ):
            with self.subTest(command=command):
                self.assertAsked(command)

    def test_git_log_without_max_count_is_allowed(self):
        """`-n`／`--max-count`が無い`git log`は対象外にする。"""
        self.assertAllowed("git log --oneline")
        self.assertAllowed("git log")

    def test_limit_on_gh_list_commands_is_asked(self):
        """`gh pr list`／`gh issue list`の`--limit`／`-L`をaskする。"""
        for command in (
            "gh pr list --limit 50",
            "gh issue list --limit=50",
            "gh pr list -L 50",
            "gh pr list -L50",
        ):
            with self.subTest(command=command):
                self.assertAsked(command)

    def test_gh_list_without_limit_is_allowed(self):
        """`--limit`を指定しない呼び出し（既定30件）は、`--json`が無ければ対象外にする。

        **これらのcommandを閲覧のためだけに使う頻度は非常に高く、`--limit`省略は
        そのほとんどを占める。**明示的な数が無い呼び出しまで対象にすると、通常の
        閲覧が毎回止まる。
        """
        self.assertAllowed("gh pr list")
        self.assertAllowed("gh issue list")

    def test_json_without_limit_is_asked(self):
        """`--json`が付いているが`--limit`が無い場合はaskする。

        **PM（`deskcat-66`）が2026-09-05に実測で見つけた反転を塞ぐ。**`--limit`を
        明示する正しい書き方はaskで止まり、省略して既定30件で黙って切れる書き方が
        素通りしていた。`--json`（機械可読）が付いている場合に限り、`--limit`省略も
        対象へ入れる。
        """
        for command in (
            "gh issue list --json number,title",
            "gh pr list --json number",
            "gh issue list --json=number",
        ):
            with self.subTest(command=command):
                self.assertAsked(command, contains="--json")

    def test_json_with_small_limit_is_asked_for_the_limit_reason(self):
        """`--json`と、閾値未満の`--limit`が両方あるときは、`--limit`側の理由でaskする。

        **4は3の穴埋めであり、3が既にaskする場合に重ねて別のaskを出さない。**
        """
        self.assertAsked(
            "gh issue list --json number,title --limit 50", contains="--limit 50"
        )

    def test_json_with_large_limit_is_allowed(self):
        """`--json`が付いていても、`--limit`が閾値以上なら通す。

        **PM（`deskcat-66`）の決定4件のうちの1つ（2026-09-05）。**`--limit`を
        実際の総数以上に明示した書き方まで止めると、規則を守った側が損をする。
        """
        self.assertAllowed("gh issue list --state open --limit 1000 --json number")

    def test_json_on_unrelated_command_is_allowed(self):
        """対象commandではない`--json`は見ない。"""
        self.assertAllowed("gh api repos/x/y/issues --jq .[].number")

    def test_gh_api_limit_is_not_judged(self):
        """`gh api`の`--paginate`要否は対象外にする。

        エンドポイントによってpaginationの要否が変わり、字句だけでは判定できない。
        推測で拾うと誤検知になる。`gh api`はpipeで`head`／`tail`へ渡す形だけを見る。
        """
        self.assertAllowed("gh api repos/x/y/issues")
        self.assertAllowed("gh api repos/x/y/issues --paginate")

    def test_limit_below_threshold_is_asked(self):
        """閾値未満の`--limit`はaskする。"""
        for command in ("gh pr list --limit 50", "gh pr list --limit 999"):
            with self.subTest(command=command):
                self.assertAsked(command)

    def test_limit_at_or_above_threshold_is_allowed(self):
        """閾値以上の`--limit`は通す。

        **PMが実測で見つけた反転の本体（2026-09-05）。**規則どおり大きく明示した
        書き方が毎回止まると、規則を破って省略する側が静かに通ることになり、
        この検査が守りたい向きと逆になる。閾値の根拠はhookのdocstringにある実測
        （Issue 135件／Pull Request 217件／board item 352件、いずれも全state、
        2026-09-05時点）であり、この3つの3倍前後に設定した。
        """
        for command in ("gh pr list --limit 1000", "gh pr list --limit 1500"):
            with self.subTest(command=command):
                self.assertAllowed(command)

    def test_limit_non_numeric_value_is_not_judged_as_large(self):
        """`--limit`の値が数値でなければ「大きい」とみなさず、askする。

        判定できない値を安全側（大きいと仮定して通す）へ倒さない。
        """
        self.assertAsked("gh pr list --limit abc")

    def test_duplicate_limit_uses_the_last_value(self):
        """`--limit`の重複指定は最後の値で判定する（`#355`のreview指摘）。

        GitHub CLIのscalar型`--limit`は、重複指定時に最後の値が有効になる。
        `--limit 1000 --limit 50`の実効値は`50`であり、最初の値（1000、閾値以上）
        で「大きい」と誤判定してはならない。
        """
        self.assertAsked(
            "gh pr list --limit 1000 --limit 50 --json number", contains="--limit 50"
        )

    def test_global_option_before_subcommand_is_detected(self):
        """subcommandの前のglobal optionを見落とさない（`#355`のreview指摘）。

        `git -C <path> log`／`gh --repo <owner/repo> pr list`は、subcommandの
        前にrepositoryを指定するglobal optionを置く。`_match_target`が
        executableの直後だけを見ると、これらを対象外と誤判定して切り詰めを
        見逃す。
        """
        for command in (
            "git -C /tmp/repo log -n 1",
            "gh --repo owner/repo pr list --limit 1",
            "gh -R owner/repo issue list --limit 1",
        ):
            with self.subTest(command=command):
                self.assertAsked(command)

    def test_compound_command_is_inspected(self):
        """`cd x && gh pr list | head -8`を見落とさない。"""
        self.assertAsked("cd /tmp && gh pr list | head -8")

    def test_unrelated_commands_are_allowed(self):
        """対象command以外は見ない。"""
        for command in ("git status", "ls -la", "gh pr view 1 --json state"):
            with self.subTest(command=command):
                self.assertAllowed(command)

    def test_help_is_not_judged(self):
        """helpの表示だけを求める呼び出しは何も列挙しないため対象外にする。"""
        for command in (
            "gh pr list --help",
            "gh pr list --limit 50 --help",
            "git log --help -n 5",
        ):
            with self.subTest(command=command):
                self.assertAllowed(command)

    def test_skip_environment_disables_the_guard(self):
        """逃げ道が効く。"""
        code, output = _invoke(
            TRUNCATION_GUARD, "gh pr list | head -8",
            environment={"DESKCAT_SKIP_TRUNCATION_GUARD": "1"},
        )
        self.assertEqual(code, 0)
        self.assertIsNone(output)

    def test_broken_input_does_not_block(self):
        """hookの入力やcommandが壊れていることを、対象commandの問題として扱わない。"""
        result = subprocess.run(
            [sys.executable, TRUNCATION_GUARD], input="{ではないJSON",
            capture_output=True, text=True, encoding="utf-8", timeout=60,
        )
        self.assertEqual(result.returncode, 0)
        self.assertEqual(result.stdout.strip(), "")
        self.assertAllowed('gh pr list --limit "閉じていない')


STOP_CLAIM_GUARD = str(SCRIPTS_ROOT / "hooks" / "stop_claim_guard.py")

# transcript行の最小形。**PMの決定5により、判定の対象となる形をfixtureとして
# 固定する。**Claude Codeのtranscript形式が変わればこのtestが落ちる。
TRANSCRIPT_HUMAN_STRING = {"type": "user", "message": {"role": "user", "content": "何か送って"}}
TRANSCRIPT_TOOL_RESULT_ONLY = {
    "type": "user",
    "message": {"role": "user", "content": [{"type": "tool_result", "content": "ok"}]},
}
TRANSCRIPT_HUMAN_WITH_TEXT_BLOCK = {
    "type": "user",
    "message": {"role": "user", "content": [{"type": "text", "text": "続けて"}]},
}


def _assistant_text(text):
    return {"type": "assistant", "message": {"role": "assistant", "content": [{"type": "text", "text": text}]}}


def _assistant_tool_use(name, command=None):
    tool_input = {"command": command} if command is not None else {}
    return {
        "type": "assistant",
        "message": {"role": "assistant", "content": [{"type": "tool_use", "name": name, "input": tool_input}]},
    }


class StopClaimGuardTests(unittest.TestCase):
    """完了の主張とtool呼び出しを突き合わせるhookのtest（決定1、#350）。"""

    def setUp(self):
        self._tmpdir = tempfile.TemporaryDirectory()
        self.addCleanup(self._tmpdir.cleanup)

    def _write_transcript(self, entries):
        path = Path(self._tmpdir.name) / "transcript.jsonl"
        with open(path, "w", encoding="utf-8") as handle:
            for entry in entries:
                handle.write(json.dumps(entry, ensure_ascii=False) + "\n")
        return str(path)

    def _scratchpad(self, name="scratchpad"):
        path = Path(self._tmpdir.name) / name
        path.mkdir(exist_ok=True)
        return str(path)

    def _invoke(self, message, transcript_entries, *, scratchpad=None):
        payload = {
            "last_assistant_message": message,
            "transcript_path": self._write_transcript(transcript_entries),
            "scratchpad_dir": scratchpad or self._scratchpad(),
        }
        result = subprocess.run(
            [sys.executable, STOP_CLAIM_GUARD],
            input=json.dumps(payload, ensure_ascii=False),
            capture_output=True, text=True, encoding="utf-8", timeout=60,
        )
        text = result.stdout.strip()
        return result.returncode, (json.loads(text) if text else None)

    def assertBlocked(self, message, transcript_entries, *, contains=None, scratchpad=None):
        code, output = self._invoke(message, transcript_entries, scratchpad=scratchpad)
        self.assertEqual(code, 0, message)
        self.assertIsNotNone(output, f"通してしまった: {message}")
        self.assertEqual(output["decision"], "block", message)
        if contains:
            self.assertIn(contains, output["reason"])

    def assertAllowed(self, message, transcript_entries, *, scratchpad=None):
        code, output = self._invoke(message, transcript_entries, scratchpad=scratchpad)
        self.assertEqual(code, 0, message)
        self.assertIsNone(output, f"止めてしまった: {message}")

    def test_send_claim_without_evidence_is_blocked(self):
        """証拠の無い送信主張をblockし、主張した語を名指しする。"""
        self.assertBlocked(
            "送りました", [TRANSCRIPT_HUMAN_STRING], contains="送りました"
        )

    def test_send_claim_with_evidence_is_allowed(self):
        """`mcp__ccd_session_mgmt__send_message`の呼び出しがあれば通す。"""
        self.assertAllowed(
            "送りました",
            [
                TRANSCRIPT_HUMAN_STRING,
                _assistant_tool_use("mcp__ccd_session_mgmt__send_message"),
                TRANSCRIPT_TOOL_RESULT_ONLY,
            ],
        )

    def test_evidence_across_multiple_assistant_blocks_is_found(self):
        """複数回のtool往復（複数の`requestId`相当）を跨いでも証拠を見つける。

        1つの人間turnの中で、thinking→tool_use→tool_result→…→最後はtextだけ、
        という繰り返しが実際に起きる（担当セッションが自身のtranscriptで観測）。
        直前の1回分だけを見ると、途中のtool呼び出しを見落とす。
        """
        self.assertAllowed(
            "起票しました",
            [
                TRANSCRIPT_HUMAN_STRING,
                {"type": "assistant", "message": {"role": "assistant", "content": [{"type": "thinking", "text": "..."}]}},
                _assistant_tool_use("Bash", "gh issue create --title t --body-file f"),
                TRANSCRIPT_TOOL_RESULT_ONLY,
                _assistant_text("起票しました"),
            ],
        )

    def test_issue_create_evidence_for_filing_claim(self):
        """`起票しました`は`gh issue create`のBash呼び出しで満たす。"""
        self.assertAllowed(
            "起票しました",
            [TRANSCRIPT_HUMAN_STRING, _assistant_tool_use("Bash", "gh issue create --title t")],
        )

    def test_merge_evidence_for_merge_claim(self):
        """`mergeしました`は`gh pr merge`のBash呼び出しで満たす。"""
        self.assertAllowed(
            "mergeしました",
            [TRANSCRIPT_HUMAN_STRING, _assistant_tool_use("Bash", "gh pr merge 1 --squash")],
        )

    def test_push_evidence_for_push_claim(self):
        """`pushしました`は`git push`のBash呼び出しで満たす。"""
        self.assertAllowed(
            "pushしました",
            [TRANSCRIPT_HUMAN_STRING, _assistant_tool_use("Bash", "git push origin main")],
        )

    def test_unrelated_bash_command_is_not_evidence(self):
        """関係ないBash呼び出しは証拠にならない。"""
        self.assertBlocked(
            "pushしました",
            [TRANSCRIPT_HUMAN_STRING, _assistant_tool_use("Bash", "git status")],
        )

    def test_dry_run_push_is_not_evidence(self):
        """`git push --dry-run`は完了の証拠にならない（`#355`のreview指摘）。

        `--dry-run`は実際には何も送信しない。subcommandが一致するだけで
        証拠として数えると、送信していないことを「pushしました」と主張できてしまう。
        """
        self.assertBlocked(
            "pushしました",
            [TRANSCRIPT_HUMAN_STRING, _assistant_tool_use("Bash", "git push --dry-run origin main")],
        )

    def test_auto_merge_is_not_evidence(self):
        """`gh pr merge --auto`は完了の証拠にならない（`#355`のreview指摘）。

        `--auto`は要件が揃うまでの予約であり、その場でmergeするわけではない。
        """
        self.assertBlocked(
            "mergeしました",
            [TRANSCRIPT_HUMAN_STRING, _assistant_tool_use("Bash", "gh pr merge 1 --auto")],
        )

    def test_actual_push_is_still_evidence(self):
        """`--dry-run`を伴わない`git push`は引き続き証拠になる。"""
        self.assertAllowed(
            "pushしました",
            [TRANSCRIPT_HUMAN_STRING, _assistant_tool_use("Bash", "git push origin main")],
        )

    def test_actual_merge_is_still_evidence(self):
        """`--auto`を伴わない`gh pr merge`は引き続き証拠になる。"""
        self.assertAllowed(
            "mergeしました",
            [TRANSCRIPT_HUMAN_STRING, _assistant_tool_use("Bash", "gh pr merge 1 --squash")],
        )

    def test_quoted_claim_is_not_counted(self):
        """引用（`「」`）の中の語は主張として数えない。"""
        self.assertAllowed("彼は「送りました」と言っていた", [TRANSCRIPT_HUMAN_STRING])

    def test_negated_claim_is_not_counted(self):
        """否定形は主張として数えない。"""
        for message in ("まだ送っていません", "送りませんでした"):
            with self.subTest(message=message):
                self.assertAllowed(message, [TRANSCRIPT_HUMAN_STRING])

    def test_future_claim_is_not_counted(self):
        """未来形・意志形は主張として数えない。"""
        for message in ("これから送ります", "送信します"):
            with self.subTest(message=message):
                self.assertAllowed(message, [TRANSCRIPT_HUMAN_STRING])

    def test_no_claim_is_allowed(self):
        """完了を主張する語が無い応答は何も見ない。"""
        self.assertAllowed("承知しました。作業を進めます。", [TRANSCRIPT_HUMAN_STRING])

    def test_same_category_is_blocked_once_then_allowed(self):
        """同じ主張分類に対してblockするのは1回だけとし、2回目は通す。

        **`stop_hook_active`には依存しない**（担当セッションの実測で、
        block後の再実行でもfalseのままだったため）。状態は`scratchpad_dir`
        （セッション内で完結する場所）に持つ。
        """
        scratchpad = self._scratchpad()
        self.assertBlocked("送りました", [TRANSCRIPT_HUMAN_STRING], scratchpad=scratchpad)
        self.assertAllowed("送りました", [TRANSCRIPT_HUMAN_STRING], scratchpad=scratchpad)

    def test_different_session_state_does_not_leak(self):
        """`scratchpad_dir`が違えば、blockの記録は引き継がれない。

        **セッションをまたいで残る場所へ書くと、前のセッションのblockが
        次のセッションを素通りさせる。**`scratchpad_dir`はsession_idを含む
        pathであり、別sessionなら別の場所になる。
        """
        self.assertBlocked(
            "送りました", [TRANSCRIPT_HUMAN_STRING], scratchpad=self._scratchpad("session_a")
        )
        self.assertBlocked(
            "送りました", [TRANSCRIPT_HUMAN_STRING], scratchpad=self._scratchpad("session_b")
        )

    def test_human_turn_boundary_uses_tool_result_only_rule(self):
        """境界判定は「文字列か」ではなく「tool_resultだけで構成されていないか」で行う。

        **PMの決定1（2026-09-06）。**画像添付等、人間の入力でも`content`が
        listになる形（`tool_result`以外のblockを含むlist）を、境界として正しく
        扱えることを確かめる。
        """
        self.assertAllowed(
            "送りました",
            [
                TRANSCRIPT_TOOL_RESULT_ONLY,  # これより前の(無い)tool_useは境界外
                TRANSCRIPT_HUMAN_WITH_TEXT_BLOCK,  # tool_result以外を含むlist→人間の入力
                _assistant_tool_use("mcp__ccd_session_mgmt__send_message"),
            ],
        )

    def test_boundary_not_found_uses_whole_file(self):
        """人間の入力entryが1つも無い場合は、file全体を対象にする。

        **tool_useを多く集める方向であり、blockを減らす安全側である。**
        """
        self.assertAllowed(
            "送りました",
            [_assistant_tool_use("mcp__ccd_session_mgmt__send_message")],
        )

    def test_unreadable_transcript_is_allowed(self):
        """transcriptを読めない場合はblockしない。判定できないことを理由にしない。"""
        payload = {
            "last_assistant_message": "送りました",
            "transcript_path": "/nonexistent/path.jsonl",
            "scratchpad_dir": self._scratchpad(),
        }
        result = subprocess.run(
            [sys.executable, STOP_CLAIM_GUARD],
            input=json.dumps(payload, ensure_ascii=False),
            capture_output=True, text=True, encoding="utf-8", timeout=60,
        )
        self.assertEqual(result.returncode, 0)
        self.assertEqual(result.stdout.strip(), "")

    def test_non_string_transcript_path_is_allowed(self):
        """`transcript_path`が文字列以外のtruthy値でも`TypeError`を出さず通す。

        `#355`のreview指摘。非空listや数値は真偽値としてtruthyだが、`open()`へ
        渡すと`TypeError`になる。
        """
        for transcript_path in ([1, 2], {"a": 1}, 123):
            with self.subTest(transcript_path=transcript_path):
                payload = {
                    "last_assistant_message": "送りました",
                    "transcript_path": transcript_path,
                    "scratchpad_dir": self._scratchpad(),
                }
                result = subprocess.run(
                    [sys.executable, STOP_CLAIM_GUARD],
                    input=json.dumps(payload, ensure_ascii=False),
                    capture_output=True, text=True, encoding="utf-8", timeout=60,
                )
                self.assertEqual(result.returncode, 0)
                self.assertEqual(result.stderr, "")
                self.assertEqual(result.stdout.strip(), "")

    def test_broken_input_does_not_block(self):
        """hookの入力が壊れていることを、対象応答の問題として扱わない。"""
        result = subprocess.run(
            [sys.executable, STOP_CLAIM_GUARD], input="{ではないJSON",
            capture_output=True, text=True, encoding="utf-8", timeout=60,
        )
        self.assertEqual(result.returncode, 0)
        self.assertEqual(result.stdout.strip(), "")

    def test_no_last_assistant_message_is_allowed(self):
        """`last_assistant_message`が無い（または空）payloadは何も見ない。"""
        for payload in ({}, {"last_assistant_message": ""}, {"last_assistant_message": None}):
            with self.subTest(payload=payload):
                result = subprocess.run(
                    [sys.executable, STOP_CLAIM_GUARD],
                    input=json.dumps(payload), capture_output=True, text=True,
                    encoding="utf-8", timeout=60,
                )
                self.assertEqual(result.returncode, 0)
                self.assertEqual(result.stdout.strip(), "")


class MergeTrailerReportTests(unittest.TestCase):
    """merge後の確認hookのtest。

    **`gh`と実際のPull Requestを要する経路は検査しない。**testがnetworkとGitHubの
    状態に依存すると、落ちた理由がhookの誤りか環境かを区別できなくなる。
    ここで見るのは、対象外のcommandで`gh`を呼ばずに抜けることである。
    """

    def test_non_merge_commands_are_ignored(self):
        for command in (
            "git status",
            "gh pr view 1 --json state",
            "gh pr create --title t --project deskcat",
            "echo gh pr merge",
        ):
            with self.subTest(command=command):
                code, output = _invoke(MERGE_REPORT, command)
                self.assertEqual(code, 0, command)
                self.assertIsNone(output, f"報告してしまった: {command}")

    def test_broken_input_is_ignored(self):
        result = subprocess.run(
            [sys.executable, MERGE_REPORT], input="{ではないJSON",
            capture_output=True, text=True, encoding="utf-8", timeout=60,
        )
        self.assertEqual(result.returncode, 0)
        self.assertEqual(result.stdout.strip(), "")


class PushGateTests(unittest.TestCase):
    """`develop`への直接pushを`gate`で見るhookのtest。

    **fixture repositoryへ`review_gate.py`を複製して実際に実行させる。**
    hookが呼ぶのはsubprocessであり、呼べているかどうかを模擬では確かめられない。
    """

    # `docs/decisions/`は`INSTRUCTION_SOURCES`に入る。`Instruction-Change`が要る。
    INSTRUCTION_FILE = "docs/decisions/0001-x.md"

    def setUp(self):
        self.directory = tempfile.mkdtemp()
        self.addCleanup(guards.remove_tree, Path(self.directory))
        root = Path(self.directory)
        self.root = root
        _git(str(root), "init", "--quiet", ".")
        (root / "scripts").mkdir()
        (root / "scripts" / "review_gate.py").write_text(
            (SCRIPTS_ROOT / "review_gate.py").read_text(encoding="utf-8"),
            encoding="utf-8",
        )
        (root / "note.md").write_text("最初\n", encoding="utf-8")
        _git(str(root), "add", "-A")
        _git(str(root), "commit", "--quiet", "-m", "first")
        # 自分自身をoriginにする。networkへ出ずに`origin/develop`を成立させる。
        _git(str(root), "branch", "--force", "trunk", "HEAD")
        _git(str(root), "remote", "add", "origin", str(root))
        _git(
            str(root), "config", "remote.origin.fetch",
            "+refs/heads/trunk:refs/remotes/origin/develop",
        )
        _git(str(root), "fetch", "--quiet", "origin")

    def _instruction_commit(self, declared):
        """指示sourceを触るcommitを1つ積む。`declared`で宣言の有無を切り替える。"""
        target = self.root / self.INSTRUCTION_FILE
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text("# 決定\n\n本文\n", encoding="utf-8")
        trailers = [
            f"{gate.TRAILER_CLASS}: {gate.CLASS_FIXUP}",
            *(
                f"{gate.TRAILER_REVIEW}: {value}"
                for value in gate.REVIEW_DECLARATIONS
            ),
            f"{gate.TRAILER_REFS}: #1",
        ]
        if declared:
            trailers.append(
                f"{gate.TRAILER_INSTRUCTION}: {gate.INSTRUCTION_ACK}"
            )
        _git(str(self.root), "add", "-A")
        _git(
            str(self.root), "commit", "--quiet",
            "-m", "決定を足す\n\n" + "\n".join(trailers) + "\n",
        )

    def _review_gate(self, command):
        return subprocess.run(
            [
                sys.executable, str(self.root / "scripts" / "review_gate.py"),
                command, "--repository-root", str(self.root),
                "--base", "origin/develop", "--head", "HEAD",
            ],
            capture_output=True, text=True, encoding="utf-8", cwd=str(self.root),
        ).returncode

    def assertDenied(self, command, cwd=None):
        code, output = _invoke(PUSH_GATE, command, cwd=cwd or str(self.root))
        self.assertEqual(code, 0, command)
        self.assertIsNotNone(output, f"通してしまった: {command}")
        return _reason(output)

    def assertAllowed(self, command, cwd=None):
        code, output = _invoke(PUSH_GATE, command, cwd=cwd or str(self.root))
        self.assertEqual(code, 0, command)
        self.assertIsNone(output, f"止めてしまった: {command}")

    def test_undeclared_instruction_change_is_denied(self):
        """`Instruction-Change`を持たないまま`develop`へ押すのを止める。

        **`b93b309`で実際に起きた形である。**
        """
        self._instruction_commit(declared=False)
        reason = self.assertDenied("git push origin HEAD:develop")
        self.assertIn(gate.TRAILER_INSTRUCTION, reason)

    def test_receipt_alone_would_have_passed(self):
        """同じcommitが`receipt`だけなら通る。**この差がhookの存在理由である。**"""
        self._instruction_commit(declared=False)
        self.assertEqual(self._review_gate("receipt"), 0)
        self.assertEqual(self._review_gate("gate"), 1)

    def test_declared_instruction_change_is_allowed(self):
        """宣言が揃っていれば通す。"""
        self._instruction_commit(declared=True)
        self.assertAllowed("git push origin HEAD:develop")

    def test_refspec_forms_that_update_develop_are_inspected(self):
        """`develop`を更新する書き方をひととおり拾う。"""
        self._instruction_commit(declared=False)
        _git(str(self.root), "branch", "--force", "develop", "HEAD")
        for command in (
            "git push origin HEAD:develop",
            "git push origin develop",
            "git push origin HEAD:refs/heads/develop",
            "git push origin +HEAD:develop",
            "git push --force-with-lease origin HEAD:develop",
            "cd . && git push origin HEAD:develop",
            "/usr/bin/git push origin HEAD:develop",
            "git -c core.pager=cat push origin HEAD:develop",
            "git --no-pager push origin HEAD:develop",
        ):
            self.assertDenied(command)

    def test_deleting_develop_is_not_read_as_pushing_head(self):
        """`git push origin :develop`はbranchの削除であり、押すcommitが無い。

        `HEAD`を押すものとして扱うと、**無関係な範囲を検査する。**
        """
        self._instruction_commit(declared=False)
        self.assertIsNone(push_gate.pushed_source("git push origin :develop"))
        self.assertAllowed("git push origin :develop")

    def test_tag_option_does_not_hide_an_explicit_refspec(self):
        """`--tags`を併記しても、`develop`を指すrefspecは見る。

        **`--tags`をskip対象に置くと、この形を取り落とす。**
        """
        self._instruction_commit(declared=False)
        self.assertDenied("git push --tags origin HEAD:develop")

    def test_forms_without_a_refspec_are_not_inspected(self):
        """refspecを書かない形は対象にならない。**gapとして文書に書いてある。**"""
        self._instruction_commit(declared=False)
        for command in (
            "git push --mirror origin",
            "git push --all origin",
            "git push --tags origin",
        ):
            self.assertIsNone(push_gate.pushed_source(command), command)

    def test_prefix_of_develop_is_not_matched(self):
        """`develop`で始まるだけのbranchを取り違えない。"""
        self._instruction_commit(declared=False)
        for command in (
            "git push origin HEAD:developer",
            "git push origin HEAD:develop-2",
            "git push origin HEAD:feature/develop",
        ):
            self.assertAllowed(command)

    def test_directory_option_selects_the_repository(self):
        """`git -C <dir> push`は、そのdirectoryのrepositoryを検査する。

        **cwdの側を検査すると、別treeの結果で判断することになる。**
        """
        self._instruction_commit(declared=False)
        other = tempfile.mkdtemp()
        self.addCleanup(guards.remove_tree, Path(other))
        source, directory = push_gate.pushed_source(
            f"git -C {self.root} push origin HEAD:develop"
        )
        self.assertEqual((source, directory), ("HEAD", str(self.root)))
        # cwdをrepositoryの外に置いても、`-C`の先を見て拒否する。
        reason = self.assertDenied(
            f"git -C {self.root} push origin HEAD:develop", cwd=other
        )
        self.assertIn(gate.TRAILER_INSTRUCTION, reason)

    def test_directory_option_outside_a_repository_is_not_inspected(self):
        """`-C`の先がrepositoryでなければ見ない。どこを検査すべきか決まらない。"""
        self._instruction_commit(declared=False)
        other = tempfile.mkdtemp()
        self.addCleanup(guards.remove_tree, Path(other))
        self.assertAllowed(f"git -C {other} push origin HEAD:develop")

    def test_other_destinations_are_out_of_scope(self):
        """`develop`以外へのpushは見ない。Pull Requestが`gate`を通す。"""
        self._instruction_commit(declared=False)
        for command in (
            "git push origin HEAD:feature/x",
            "git push origin HEAD:main",
            "git push fork HEAD:develop",
            "git push --dry-run origin HEAD:develop",
            "git push --delete origin develop",
            "git push --tags origin",
            "git status",
        ):
            self.assertAllowed(command)

    def test_the_word_push_in_an_argument_is_not_an_invocation(self):
        """`echo git push origin HEAD:develop`を呼び出しと読まない。"""
        self._instruction_commit(declared=False)
        self.assertAllowed("echo git push origin HEAD:develop")

    def test_nothing_to_push_is_allowed(self):
        """押すcommitが無ければ検査しない。範囲が空でheadのtrailerを問わない。"""
        _git(str(self.root), "checkout", "--quiet", "-B", "work", "origin/develop")
        self.assertAllowed("git push origin HEAD:develop")

    def test_bare_push_follows_the_upstream(self):
        """`git push`だけの形は、upstreamが`origin/develop`のときだけ対象になる。"""
        self._instruction_commit(declared=False)
        _git(str(self.root), "branch", "--set-upstream-to", "origin/develop")
        self.assertDenied("git push")

    def test_bare_push_to_another_upstream_is_out_of_scope(self):
        """upstreamが`origin/develop`でなければ見ない。"""
        self._instruction_commit(declared=False)
        branch = _git(str(self.root), "rev-parse", "--abbrev-ref", "HEAD").strip()
        # `--set-upstream-to`はremote-tracking refの実在を要求するため、configで置く。
        _git(str(self.root), "config", f"branch.{branch}.remote", "origin")
        _git(str(self.root), "config", f"branch.{branch}.merge", "refs/heads/other")
        self.assertAllowed("git push")

    def test_skip_environment_disables_the_guard(self):
        """逃げ道が効く。**使ったら理由を残すのは人間の側の規則である。**"""
        self._instruction_commit(declared=False)
        code, output = _invoke(
            PUSH_GATE, "git push origin HEAD:develop", cwd=str(self.root),
            environment={"DESKCAT_SKIP_PUSH_GATE": "1"},
        )
        self.assertEqual(code, 0)
        self.assertIsNone(output)

    def test_broken_input_does_not_block(self):
        """壊れた入力で作業を止めない。"""
        result = subprocess.run(
            [sys.executable, PUSH_GATE], input="{壊れている",
            capture_output=True, text=True, encoding="utf-8",
            timeout=60, cwd=str(self.root),
        )
        self.assertEqual(result.returncode, 0)
        self.assertEqual(result.stdout.strip(), "")



class CodeRabbitGateTests(unittest.TestCase):
    """CodeRabbitのreviewを起動するcommandを見るhookのtest。

    **このhookだけ`deny`ではなく`ask`を返す。**他のhookのtestは`deny`を期待するため、
    検査する経路を別に持つ。**止めたいのはAIの独断であって、人間の依頼ではない。**
    """

    def assertAsked(self, command, *, contains=None):
        code, output = _invoke(CODERABBIT_GATE, command)
        self.assertEqual(code, 0, command)
        self.assertIsNotNone(output, f"通してしまった: {command}")
        self.assertEqual(
            output["hookSpecificOutput"]["permissionDecision"], "ask", command
        )
        if contains:
            self.assertIn(contains, _reason(output))

    def assertAllowed(self, command):
        code, output = _invoke(CODERABBIT_GATE, command)
        self.assertEqual(code, 0, command)
        self.assertIsNone(output, f"止めてしまった: {command}")

    def test_review_triggers_are_asked(self):
        """reviewを起動する語を止める。**枠を消費するのはこの2つだけである。**"""
        for command in (
            'gh pr comment 239 --body "@coderabbitai full review"',
            'gh pr comment 239 --body "@coderabbitai review"',
            'gh issue comment 240 --body "@coderabbitai full review"',
            'gh pr comment 239 -b "@coderabbitai full review"',
            'gh pr comment 239 --body="@coderabbitai full review"',
        ):
            with self.subTest(command=command):
                self.assertAsked(command)

    def test_passthrough_words_are_allowed(self):
        """枠を消費しない語は通す。**残数確認とthreadの後始末を塞がない。**"""
        for command in (
            'gh pr comment 239 --body "@coderabbitai rate limit"',
            'gh pr comment 239 --body "@coderabbitai resolve"',
            'gh pr comment 239 --body "@coderabbitai help"',
            'gh pr comment 239 --body "@coderabbitai configuration"',
        ):
            with self.subTest(command=command):
                self.assertAllowed(command)

    def test_spelling_variants_are_asked(self):
        """表記の揺れで抜けない。**語の並びを列挙していた版は`full-review`を通した。**"""
        for command in (
            'gh pr comment 239 --body "@coderabbitai full-review"',
            'gh pr comment 239 --body "@CodeRabbitAI FULL REVIEW"',
            'gh pr comment 239 --body "  @coderabbitai   full review  "',
        ):
            with self.subTest(command=command):
                self.assertAsked(command)

    def test_passthrough_word_does_not_grant_immunity(self):
        """`rate limit`で始めて後ろに`review`を置く形を止める。

        **免除listを先に見る版はこれを通した。**実測して直した。
        """
        self.assertAsked(
            'gh pr comment 239 --body "@coderabbitai rate limit and then full review"'
        )

    def test_review_on_another_line_is_allowed(self):
        """mentionと同じ行に`review`が無ければ通す。

        指摘への返信は別の行で`review`に触れる。**行単位で見る理由である。**
        """
        self.assertAllowed(
            'gh pr comment 239 --body "確認しました\nreviewで出た指摘を反映\n@coderabbitai resolve"'
        )

    def test_mention_without_command_is_allowed(self):
        """`@coderabbitai`だけではreviewが始まらない。止めない。"""
        self.assertAllowed('gh pr comment 239 --body "@coderabbitai"')

    def test_mention_elsewhere_in_body_is_allowed(self):
        """`@coderabbitai`の直後以外にある`review`で止めない。

        指摘への返信は「reviewで出た指摘を反映した」のような文を含む。
        **返信を塞ぐと、threadの後始末ができなくなる。**
        """
        for command in (
            'gh pr comment 239 --body "reviewで出た指摘を反映しました"',
            'gh pr comment 239 --body "full review の結果を記録します"',
        ):
            with self.subTest(command=command):
                self.assertAllowed(command)

    def test_compound_command_is_inspected(self):
        """`cd x && gh pr comment ...`の形を素通りさせない。

        `if`条件で`Bash(gh *)`へ絞るとこの形が抜ける。絞らない理由である。
        """
        self.assertAsked(
            'cd /tmp && gh pr comment 239 --body "@coderabbitai full review"'
        )

    def test_quoted_command_is_not_inspected(self):
        """`echo`の引数として書かれた形は実行ではない。止めない。"""
        self.assertAllowed(
            'echo gh pr comment 239 --body "@coderabbitai full review"'
        )

    def test_api_field_is_inspected(self):
        """`gh api`の`-f body=...`でも止める。**commentはこの経路でも投げられる。**"""
        self.assertAsked(
            'gh api repos/o/r/issues/1/comments -f body="@coderabbitai full review"'
        )

    def test_body_file_is_read(self):
        """`--body-file`の中身を読んで判定する。"""
        with tempfile.TemporaryDirectory() as work:
            path = Path(work) / "body.md"
            path.write_text("@coderabbitai full review\n", encoding="utf-8")
            self.assertAsked(f'gh pr comment 239 --body-file {path}')

    def test_an_unreadable_body_is_asked_not_allowed(self):
        """本文を読めない呼び出しを素通りさせない。**askする。**

        以前は素通りさせ、docstringも「この方向の取りこぼしは意図である」としていた。
        **しかし`-F body=@-`は意図して選べる経路であり、外したい側が自分で選べる穴だった。**
        **判定は`deny`ではなく`ask`である。**誤検知の代償は人への確認1回で、
        見逃しの代償はこのhookが在る理由そのものである。
        """
        for command in (
            "gh pr comment 1 --body-file /nonexistent/body.md",
            "gh api repos/o/r/issues/1/comments --input /nonexistent/body.json",
            "gh api -X POST repos/o/r/issues/1/comments -F body=@/nonexistent/b.txt",
            "gh pr comment 1 --body-file -",
            "gh api -X POST repos/o/r/issues/1/comments -F body=@-",
        ):
            with self.subTest(command=command):
                self.assertAsked(command, contains="読めない")

    def test_a_readable_body_without_review_is_still_allowed(self):
        """**読めた本文にreviewが無ければ素通りする。**askを一律にはしない。"""
        with tempfile.TemporaryDirectory() as directory:
            body = Path(directory) / "b.md"
            body.write_text("@coderabbitai rate limit", encoding="utf-8")
            self.assertAllowed(f"gh pr comment 1 --body-file {body}")
            self.assertAllowed(
                f"gh api repos/o/r/issues/1/comments --input {body}"
            )

    def test_unrelated_commands_are_allowed(self):
        """CodeRabbitに関係しない`gh`と読み取りを通す。"""
        for command in (
            "gh pr view 239 --json body",
            'gh pr comment 239 --body "通常のコメント"',
            "gh pr merge 239 --squash --subject s --body-file /tmp/x",
            "git status",
        ):
            with self.subTest(command=command):
                self.assertAllowed(command)

    def test_broken_input_does_not_block(self):
        """hookの入力が壊れていても止めない。**既存hookと同じ扱いである。**"""
        result = subprocess.run(
            [sys.executable, CODERABBIT_GATE],
            input="{ not json", capture_output=True, text=True,
            encoding="utf-8", errors="replace", timeout=60,
        )
        self.assertEqual(result.returncode, 0)
        self.assertEqual(result.stdout.strip(), "")
        # **握りつぶさない。**素通りの動作は変えずに、理由をstderrへ残す
        # （PR #241のreview指摘）。
        self.assertIn("coderabbit_gate:", result.stderr)

    def test_invalid_utf8_input_is_recorded(self):
        """不正なUTF-8の入力も分類して記録する。**素通りの動作は変えない。**"""
        result = subprocess.run(
            [sys.executable, CODERABBIT_GATE],
            input=b"\xff\xfe not utf-8", capture_output=True, timeout=60,
        )
        self.assertEqual(result.returncode, 0)
        self.assertEqual(result.stdout.decode("utf-8", "replace").strip(), "")
        self.assertIn("coderabbit_gate:", result.stderr.decode("utf-8", "replace"))

    def test_non_mapping_input_does_not_raise(self):
        """妥当なJSONでもmappingでない入力で落ちない。

        `payload.get`／`tool_input.get`は`AttributeError`を出す。
        **hookが例外で落ちると、止めているはずの判定が走らない。**
        PR #241のreview指摘で見つけた。
        """
        for payload in ("[]", '"text"', "null", '{"tool_input": ["x"]}',
                        '{"tool_input": "x"}'):
            with self.subTest(payload=payload):
                result = subprocess.run(
                    [sys.executable, CODERABBIT_GATE],
                    input=payload, capture_output=True, text=True,
                    encoding="utf-8", errors="replace", timeout=60,
                )
                self.assertEqual(result.returncode, 0, payload)
                self.assertEqual(result.stdout.strip(), "", payload)

    def test_read_only_api_method_is_allowed(self):
        """読み取りと明示した`gh api`は見ない。**commentを投げられない。**

        `-f`を付けると`gh`は既定でPOSTになるが、`-X GET`を明示すると読み取りである。
        PR #241のreview指摘で見つけた。
        """
        for command in (
            'gh api -X GET repos/o/r/issues -f body="@coderabbitai full review"',
            'gh api --method HEAD repos/o/r -f body="@coderabbitai review"',
        ):
            with self.subTest(command=command):
                self.assertAllowed(command)

    def test_write_api_method_is_still_inspected(self):
        """書き込みのmethodと、method未指定は引き続き見る。"""
        for command in (
            'gh api -X POST repos/o/r/issues/1/comments -f body="@coderabbitai full review"',
            'gh api repos/o/r/issues/1/comments -f body="@coderabbitai full review"',
            'gh api graphql -f query="mutation { x(body: \\"@coderabbitai review\\") }"',
        ):
            with self.subTest(command=command):
                self.assertAsked(command)

    def test_later_write_method_is_not_masked_by_an_earlier_read(self):
        """`-X GET -X POST`を読み取り扱いにしない。**`gh`は最後の`-X`を使う。**

        `any()`で見ると先頭の`get`が後ろの`post`を隠し、**承認を経ずに素通りした**
        （2026-08-28に実測。PR #254のreviewで出た）。
        """
        self.assertAsked(
            "gh api -X GET -X POST repos/o/r/issues/1/comments"
            ' -f body="@coderabbitai full review"'
        )

    def test_read_method_declared_last_is_still_allowed(self):
        """**後勝ちは両方向へ効く。**最後が読み取りなら素通りさせる。

        `-X POST -X GET`で`gh`が投げるのはGETである。ここを止めると誤検知になる。
        """
        self.assertAllowed(
            "gh api -X POST -X GET repos/o/r/issues/1/comments"
            ' -f body="@coderabbitai full review"'
        )

    def test_input_file_is_read(self):
        """`gh api --input <file>`の本文を読む。

        **値がpathであって本文ではないため、`BODY_OPTIONS`では拾えない。**
        塞ぐまでは`--input`へ本文を置けば素通りした（2026-08-28に実測）。
        """
        with tempfile.TemporaryDirectory() as directory:
            body = Path(directory) / "body.json"
            body.write_text(
                '{"body": "@coderabbitai full review"}', encoding="utf-8"
            )
            self.assertAsked(
                f"gh api repos/o/r/issues/1/comments --input {body}"
            )
            harmless = Path(directory) / "ok.json"
            harmless.write_text(
                '{"body": "@coderabbitai rate limit"}', encoding="utf-8"
            )
            self.assertAllowed(
                f"gh api repos/o/r/issues/1/comments --input {harmless}"
            )

    def test_field_value_from_a_file_is_read(self):
        """`-F key=@<path>`の値をfileから読む。

        **`key=@path`という文字列のままでは`@coderabbitai`を含まないため素通りする**
        （2026-08-28に実測）。
        """
        with tempfile.TemporaryDirectory() as directory:
            body = Path(directory) / "body.txt"
            body.write_text("@coderabbitai full review", encoding="utf-8")
            for option in ("-F", "--field"):
                with self.subTest(option=option):
                    self.assertAsked(
                        f"gh api -X POST repos/o/r/issues/1/comments"
                        f" {option} body=@{body}"
                    )

    def test_raw_field_does_not_expand_at_sign(self):
        """`-f`／`--raw-field`の`@`をfileとして読まない。

        **`gh`は`-f`の値を文字列のまま送る**（`gh api --help`で確認した）。
        fileとして読むと、**実際には送られない内容でaskを出す**ことになる。
        """
        with tempfile.TemporaryDirectory() as directory:
            body = Path(directory) / "body.txt"
            body.write_text("@coderabbitai full review", encoding="utf-8")
            for option in ("-f", "--raw-field"):
                with self.subTest(option=option):
                    self.assertAllowed(
                        f"gh api -X POST repos/o/r/issues/1/comments"
                        f" {option} body=@{body}"
                    )

    def test_body_source_is_named_on_stderr(self):
        """読めなかった経路の名前を記録へ残す。

        **読める経路は3つある。**どれが読めなかったかを書かないと、記録から
        経路を辿れない。
        """
        for command, name in (
            (
                "gh api repos/o/r/issues/1/comments"
                " --input /nonexistent/b.json",
                "--input",
            ),
            (
                "gh api -X POST repos/o/r/issues/1/comments"
                " -F body=@/nonexistent/b.txt",
                "-F",
            ),
        ):
            with self.subTest(name=name):
                payload = json.dumps(
                    {"tool_name": "Bash", "tool_input": {"command": command}}
                )
                result = subprocess.run(
                    [sys.executable, CODERABBIT_GATE], input=payload,
                    capture_output=True, text=True,
                    encoding="utf-8", timeout=60,
                )
                # **askで止めたうえで**、経路の名前を記録へ残す。
                decision = json.loads(result.stdout.strip())
                self.assertEqual(
                    decision["hookSpecificOutput"]["permissionDecision"], "ask"
                )
                self.assertIn(name, result.stderr)

    def test_an_unreadable_body_is_recorded_on_stderr(self):
        """読めない`--body-file`をaskで止めつつ、**理由をstderrへも残す。**

        AGENTS.mdの「エラーを握りつぶさず、分類、ログ、カウンタを用意する」に従う。
        **askの診断文と、stderrの記録の両方に残す。**前者は人が読み、後者は後から辿る。
        """
        payload = json.dumps({
            "tool_name": "Bash",
            "tool_input": {"command": "gh pr comment 1 --body-file /nonexistent/x.md"},
        })
        result = subprocess.run(
            [sys.executable, CODERABBIT_GATE],
            input=payload, capture_output=True, text=True,
            encoding="utf-8", errors="replace", timeout=60,
        )
        self.assertEqual(result.returncode, 0)
        decision = json.loads(result.stdout.strip())
        self.assertEqual(
            decision["hookSpecificOutput"]["permissionDecision"], "ask"
        )
        self.assertIn("--body-file", _reason(decision))
        self.assertIn("coderabbit_gate:", result.stderr)
        self.assertIn("--body-file", result.stderr)

    def test_no_bypass_environment_variable(self):
        """**環境変数で無効化できないことを確認する。**

        他のhookは`DESKCAT_SKIP_*`を持つが、このhookは意図して持たない。
        **AIが自分で外せる抑制は抑制ではない。**#240で人間が決めた。
        """
        command = 'gh pr comment 239 --body "@coderabbitai full review"'
        for name in (
            "DESKCAT_SKIP_GH_GUARD",
            "DESKCAT_SKIP_CODERABBIT_GATE",
            "DESKCAT_SKIP_PUSH_GATE",
        ):
            with self.subTest(name=name):
                code, output = _invoke(CODERABBIT_GATE, command, environment={name: "1"})
                self.assertEqual(code, 0)
                self.assertIsNotNone(output, f"{name}で素通りした")
                self.assertEqual(
                    output["hookSpecificOutput"]["permissionDecision"], "ask"
                )


class CommandFromTests(unittest.TestCase):
    """`command_line.command_from`のtest。**5本のhookが共有する入口である。**

    妥当なJSONでもmappingでない入力で`AttributeError`を出していた（#242）。
    **hookが例外で落ちると、止めているはずの判定が走らない。**
    """

    def test_valid_payload_returns_command(self):
        self.assertEqual(
            command_line.command_from({"tool_input": {"command": "gh pr create"}}),
            "gh pr create",
        )

    def test_non_mapping_payload_returns_none(self):
        """mappingでないpayloadで例外を出さない。"""
        for payload in ([], "text", None, 0, 1.5, True, ("a",)):
            with self.subTest(payload=payload):
                self.assertIsNone(command_line.command_from(payload))

    def test_non_mapping_tool_input_returns_none(self):
        """`tool_input`がmappingでない入力で例外を出さない。"""
        for value in (["x"], "x", 0, True):
            with self.subTest(value=value):
                self.assertIsNone(command_line.command_from({"tool_input": value}))

    def test_missing_pieces_return_none(self):
        for payload in ({}, {"tool_input": None}, {"tool_input": {}},
                        {"tool_input": {"command": None}},
                        {"tool_input": {"command": 1}},
                        {"tool_input": {"command": ["gh"]}}):
            with self.subTest(payload=payload):
                self.assertIsNone(command_line.command_from(payload))


class HookPayloadShapeTests(unittest.TestCase):
    """**hookが、mappingでない入力で落ちないことを確かめる**（#242）。

    [PR #241](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/241)のreview指摘は`coderabbit_gate.py`に対するものだったが、
    **型で全数走査したら当時の5本すべてに同じ形が残っていた。**指摘は代表例であって全数ではない。

    **後から足したhookもこの一覧へ入れる。**`inspector_readonly_guard.py`は#376で足した。
    """

    HOOKS = (GH_GUARD, BASE_GUARD, PUSH_GATE, MERGE_REPORT, CODERABBIT_GATE,
             INSPECTOR_GUARD)

    MALFORMED = (
        "[]",
        '"text"',
        "null",
        "0",
        '{"tool_input": ["x"]}',
        '{"tool_input": "x"}',
        '{"tool_input": {"command": 1}}',
        "{}",
    )

    def test_no_hook_raises_on_malformed_payload(self):
        """例外を出さず、素通りする。**return code 0かつstdoutが空である。**"""
        for script in self.HOOKS:
            for payload in self.MALFORMED:
                with self.subTest(script=Path(script).name, payload=payload):
                    result = subprocess.run(
                        [sys.executable, script],
                        input=payload, capture_output=True, text=True,
                        encoding="utf-8", errors="replace", timeout=120,
                        env={**os.environ,
                             "DESKCAT_SKIP_GH_GUARD": "",
                             "DESKCAT_SKIP_BASE_GUARD": "",
                             "DESKCAT_SKIP_PUSH_GATE": ""},
                    )
                    self.assertEqual(result.returncode, 0,
                                     f"{Path(script).name} が落ちた: {result.stderr[:300]}")
                    self.assertNotIn("Traceback", result.stderr,
                                     f"{Path(script).name} が例外を出した")
                    self.assertEqual(result.stdout.strip(), "",
                                     f"{Path(script).name} が止めてしまった")


class InspectorReadonlyGuardTests(unittest.TestCase):
    """検査 subagent の`Bash`を読み取りだけに絞るhook（#376）。

    **allowlistである。**「通ってはいけないもの」を数え上げるのではなく、
    通ってよいものだけを列挙し、それ以外が落ちることを確かめる。
    """

    # **`diff`／`show`／`log`／`blame`は`--no-ext-diff --no-textconv`が要る。**
    # option 無しでも git config の外部 helper が走るため（`GIT_HELPER_SUBCOMMANDS`）。
    HELPER = "--no-ext-diff --no-textconv"

    ALLOWED = (
        f"git show {HELPER} HEAD",
        f"git -C /tmp/wt --no-pager diff {HELPER} origin/main..HEAD",
        f"git log {HELPER} --oneline -20",
        f"git diff {HELPER} HEAD~1 HEAD",
        f"git blame {HELPER} AGENTS.md",
        "git rev-parse HEAD",
        "git merge-base --is-ancestor a b",
        f"git show {HELPER} HEAD:AGENTS.md",
        # subcommandの後ろの`-c`はmergeのcombined diffであり読み取り専用である
        f"git log {HELPER} -c HEAD",
        # `--output`への前方一致で誤爆させない
        f"git log {HELPER} --output-indicator-new=X --oneline",
        "git grep -n pattern",
        # **引数の中の`FOO=bar`は代入ではない。**command位置だけを見る
        "grep FOO=bar AGENTS.md",
        f"git diff {HELPER} -- file=1",
        "cat AGENTS.md",
        "wc -l AGENTS.md",
        "head -40 AGENTS.md",
        # **`--no-optional-locks`はglobal位置に要る**
        "git --no-optional-locks status",
        # `rg`の`--pre`と前方一致で誤爆させない
        "rg --pre-glob *.md pattern",
        "rg -n pattern AGENTS.md",
    )

    DENIED = (
        # 書き込みcommandそのもの
        "rm -rf /",
        "sed -i s/a/b/ AGENTS.md",
        "cp a b",
        "tee out",
        # 書ける経路を持つ「読み取りに見える」command
        "sort -o out in",
        "uniq in out",
        # 任意codeを実行できるもの
        "python3 -c 'print(1)'",
        "sh -c 'rm x'",
        "bash -lc 'rm x'",
        "xargs rm",
        # 状態を変えるgit subcommand
        "git checkout -- .",
        "git commit -m x",
        "git branch -D develop",
        "git config --global user.name x",
        "git push origin develop",
        # subcommandが無い
        "git",
        # `shlex`が空白でしか切らないために潰れる形
        "cat a>b",
        "cat a;rm -rf /",
        "git show HEAD > /tmp/x",
        "git show HEAD | tee f",
        "echo $(rm -rf /)",
        "echo `rm -rf /`",
        # 改行で並べた2つ目のcommand
        "git show HEAD\nrm -rf /",
        # 語へ分けられない
        "cat 'unclosed",
        # **subcommandが読み取り専用でも、gitのoptionが任意commandを実行する**
        "git -c core.pager=rm log",
        "git -ccore.pager=rm log",
        "git --config-env=core.pager=P log",
        "git --exec-path=/tmp show",
        "git grep -Oless pattern",
        "git grep --open-files-in-pager=rm x",
        # **subcommandが読み取り専用でも、fileを書くoption**
        "git diff --output=/tmp/x",
        # 環境変数の代入だけで任意commandを起動できる
        "GIT_EXTERNAL_DIFF=rm git show HEAD",
        "env GIT_EXTERNAL_DIFF=rm git show HEAD",
        # **前置語は allowlist の外側から実行の権限や解決先を変える**
        "sudo cat /etc/shadow",
        "env cat AGENTS.md",
        "exec cat AGENTS.md",
        "command cat AGENTS.md",
        "nohup cat AGENTS.md",
        "time cat AGENTS.md",
        # **program をbasenameで認可しない。**名前が一致するだけの別の実行fileである
        "./git show HEAD",
        "../git log",
        "/tmp/cat file",
        "/usr/bin/git status",
        "/usr/bin/git --no-optional-locks status",
        # **`rg`のoptionが任意commandを起動する**
        "rg --pre /tmp/evil.sh pattern",
        "rg --pre=/tmp/evil.sh pattern",
        "rg --hostname-bin /tmp/evil.sh pattern",
        # **`git status`は既定で`.git/index`を書く。**global位置の指定でなければ通さない
        "git status",
        "git status --no-optional-locks",
        # **外部 helper を明示的に起動するoption**
        "git diff --ext-diff HEAD~1 HEAD",
        "git show --textconv HEAD:AGENTS.md",
        "git grep --textconv pattern",
        "git cat-file --filters HEAD:AGENTS.md",
        # **option 無しでも helper は走る。**両方の明示が無ければ通さない
        "git diff HEAD~1 HEAD",
        "git show HEAD",
        "git log -p",
        "git blame AGENTS.md",
        "git diff --no-ext-diff HEAD~1 HEAD",
        "git show --no-textconv HEAD",
    )

    def test_help_option_is_refused_because_it_hijacks_the_subcommand(self):
        """**`git <cmd> --help`は`git help <cmd>`へ書き換わる**（実測）。

        `git version --help`と`git --no-optional-locks status --help`で man が出力された。
        **guardは`version`／`status`を見ているのに、gitが実行するのは`help`である。**
        `man`は`ALLOWED_PROGRAMS`に無いが、git 経由で起動する。

        **3例目の`git rev-parse HEAD --help`は書き換わらない形である。**
        書き換わるのは global option を剥いだ後の先頭に`--help`が来たときだけである。
        **意図した過剰拒否として固定する。**位置の判定を増やすより単純である。
        """
        for command in ("git version --help",
                        "git --no-optional-locks status --help",
                        "git rev-parse HEAD --help"):
            with self.subTest(command=command):
                self.assertIsNotNone(inspector_readonly_guard.check(command))

    def test_grep_pager_option_is_refused_when_abbreviated(self):
        """**短縮拒否を`cat-file`へ限定しない。**

        限定していたとき、`git grep --open-files-in-pag=sha1sum` で
        **`ALLOWED_PROGRAMS`に無い`sha1sum`が実際に実行された**（実測）。
        `git grep --textc` も通っていた。**subcommandを数え上げると数え漏れが穴になる。**
        """
        # **`--ext-di`は穴ではない。**diff 系の parser は短縮を受け付けず、git 自身が落とす。
        # **予防として同じ扱いにしている**ことを固定する。
        for command in ("git grep --open-files-in-pag=sha1sum AGENTS -- AGENTS.md",
                        "git grep --textc zzz -- AGENTS.md",
                        "git diff --ext-di HEAD"):
            with self.subTest(command=command):
                self.assertIsNotNone(inspector_readonly_guard.check(command))

    def test_separator_looking_arguments_are_refused(self):
        """**単独の区切り語も拒否する**（実測で迂回できた）。

        `shlex`は引用符を剥いだ後の語を返すため、**bashが literal な引数として渡す語と
        区別が付かない。**`rg <pattern> { cat --pre <command> <file>`では、
        `command_line.invocations`が`{`で invocation を切り、**その先が無検査になった。**
        検査 subagent の中で実行したところ、**`ALLOWED_PROGRAMS`に無い`/usr/bin/uname`が
        実際に起動した。**

        **pipe も使えなくなる。**判定が`command_line`の語り分けに依存しなくなることを採った。
        """
        # **`command_line.SEPARATORS`の全件が拒否されること。**
        # `!`は`SHELL_METACHARACTERS`の文字を1つも含まないため、metacharacter検査だけでは
        # 素通りし、**`rg <pattern> ! cat --pre sha1sum <file>`で`sha1sum`が実際に起動した。**
        # **この形にしておけば、共有moduleへ区切りが増えても穴にならない。**
        for separator in sorted(command_line.SEPARATORS):
            with self.subTest(separator=separator):
                self.assertIsNotNone(
                    inspector_readonly_guard.check(
                        f"rg pattern {separator} cat --pre sha1sum AGENTS.md"),
                    f"区切り語 {separator!r} が素通りする")
        for command in ("rg Linux ! cat --pre sha1sum AGENTS.md",
                        "git grep Linux ! cat -O sha1sum",
                        "rg Linux { cat --pre /usr/bin/uname AGENTS.md",
                        "git diff --no-ext-diff --no-textconv { cat --output=/tmp/x HEAD",
                        "git grep Linux { cat -O/bin/date AGENTS.md",
                        "git log --no-ext-diff --no-textconv --oneline -20 | head -5"):
            with self.subTest(command=command):
                self.assertIsNotNone(inspector_readonly_guard.check(command))

    def test_signature_verification_is_refused(self):
        """**署名検証は`gpg.program`（既定`gpg`）を起動する**（実測）。

        検査 subagent の中で`git log --show-signature`を実行したところ、
        **`gpg`が`~/.gnupg`に directory と keybox file を作った。**
        `ALLOWED_PROGRAMS`に無い program が、guard を通って file を書いた。

        **`--show-signature`を拒否するだけでは閉じない。**`--format=%GK`でも走る。
        `%G?`／`%GS`／`%GK`は同じ経路であるため、**`%G`を含む語を拒否する。**
        """
        for command in ("git log --no-ext-diff --no-textconv --show-signature -1",
                        "git log --no-ext-diff --no-textconv --show-sig -1",
                        "git log --no-ext-diff --no-textconv -1 --format=%GK",
                        "git log --no-ext-diff --no-textconv -1 --pretty=format:%G?",
                        "git show --no-ext-diff --no-textconv --show-signature HEAD"):
            with self.subTest(command=command):
                self.assertIsNotNone(inspector_readonly_guard.check(command))
        # `%G`を含まない format は通る
        self.assertIsNone(
            inspector_readonly_guard.check(
                "git log --no-ext-diff --no-textconv -1 --format=%H"))

    def test_verbose_status_is_refused(self):
        """**`git status -vv`はtextconvを走らせる**（実測）。

        `-v`は走らせないが、`-vv`はworking treeのpatchを出す過程でtextconvを呼ぶ。
        **`status`は`--no-ext-diff --no-textconv`を受理しない**ため、打ち消す形が無い。
        **verboseそのものを拒否する。**`diff.external`は`-vv`でも走らなかった。
        """
        for command in ("git --no-optional-locks status -vv",
                        "git --no-optional-locks status -v",
                        "git --no-optional-locks status --verbose",
                        "git --no-optional-locks status --verb",
                        "git --no-optional-locks status -sv"):
            with self.subTest(command=command):
                self.assertIsNotNone(inspector_readonly_guard.check(command))
        for command in ("git --no-optional-locks status",
                        "git --no-optional-locks status --short",
                        # **他の subcommand の `-v` は巻き込まない。**
                        "git grep -v pattern",
                        "git log --no-ext-diff --no-textconv -v"):
            with self.subTest(command=command):
                self.assertIsNone(inspector_readonly_guard.check(command))

    def test_rg_search_zip_is_refused(self):
        """**`rg -z`は外部decompressorを起動する。**

        対象fileの拡張子に応じて`gzip`／`xz`／`zstd`等を呼ぶ。**どれも`ALLOWED_PROGRAMS`に無い。**

        **根拠の水準は`--pre`と違う。**`rg --help`で確認できるのはoptionの存在だけであり、
        **外部processの起動は測っていない。**ripgrepの文書化された挙動に依る拒否である。
        """
        for command in ("rg -z pattern", "rg --search-zip pattern", "rg -nz pattern"):
            with self.subTest(command=command):
                self.assertIsNotNone(inspector_readonly_guard.check(command))
        self.assertIsNone(inspector_readonly_guard.check("rg -n pattern"))

    def test_bundled_short_options_are_refused(self):
        """**short optionは束ねられる**（実測）。

        `git grep -nOzzz`は`-n -O zzz`であり、`token.startswith("-O")`では見えない。
        **素通りしたとき、gitがpagerとして`zzz`をexecしようとした。**
        """
        for command in ("git grep -nOzzz AGENTS -- AGENTS.md",
                        "git grep -Ozzz x",
                        "git grep -nO less x"):
            with self.subTest(command=command):
                self.assertIsNotNone(inspector_readonly_guard.check(command))
        for command in ("git grep -n pattern", "git grep -i -n x"):
            with self.subTest(command=command):
                self.assertIsNone(inspector_readonly_guard.check(command))

    def test_git_version_is_allowed_for_recording_the_environment(self):
        """**判定は git の version 依存である。**検査 session 側で記録できるようにする。

        `git --version` は subcommand が無い形になるため拒否される。`git version` と書く。
        """
        self.assertIsNone(inspector_readonly_guard.check("git version"))
        self.assertIsNotNone(inspector_readonly_guard.check("git --version"))

    def test_unlisted_global_option_names_itself_in_the_reason(self):
        """**拒否理由が原因を名指しする。**

        以前は「`git` に subcommand が無い」としか出ず、どの option が原因か分からなかった。
        """
        reason = inspector_readonly_guard.check("git -p log --no-ext-diff --no-textconv")
        self.assertIsNotNone(reason)
        self.assertIn("global option", reason)
        self.assertIn("-p", reason)

    def test_cat_file_denied_options_are_refused_when_abbreviated(self):
        """**`git cat-file`は短縮綴りを受理する**（実測）。

        `--textcon`／`--textc`／`--te`はすべて`--textconv`として実行された
        （textconv driver に `echo TEXTCONV_RAN` する script を設定し、
        **3形とも出力が `TEXTCONV_RAN`、対照の `-p` は file の中身**になった）。
        **完全一致だけを見ると抜けられる。**
        """
        for command in ("git cat-file --textcon HEAD:AGENTS.md",
                        "git cat-file --te HEAD:AGENTS.md",
                        "git cat-file --filt HEAD:AGENTS.md"):
            with self.subTest(command=command):
                self.assertIsNotNone(inspector_readonly_guard.check(command))
        self.assertIsNone(inspector_readonly_guard.check("git cat-file -p HEAD:AGENTS.md"))

    def test_abbreviation_denial_does_not_reach_other_subcommands(self):
        """**短縮拒否を`diff`系へ広げない。**

        `--filter`は`rev-list`の正当な読み取り専用 option であり、`--filters`の短縮ではない。
        **巻き添えで落とすと検査が止まる。**
        （`git log --filter=...`は git 2.34.1 では`unrecognized argument`になる。
        限定の根拠は`rev-list`である。）
        """
        self.assertIsNone(
            inspector_readonly_guard.check("git rev-list --objects --filter=blob:none HEAD"))

    def test_unlisted_global_options_are_refused(self):
        """**許可していない global option は、値を取るかが分からない**（実測）。

        `git --super-prefix rev-parse submodule--helper x`では、guardが`rev-parse`を、
        gitが`submodule--helper`を subcommand として読む。
        **subcommand allowlist も必須 flag も、この読みの上に乗っている。**
        """
        for command in ("git --super-prefix rev-parse submodule--helper x",
                        "git -p log --no-ext-diff --no-textconv",
                        "git --paginate diff --no-ext-diff --no-textconv"):
            with self.subTest(command=command):
                self.assertIsNotNone(inspector_readonly_guard.check(command))
        for command in ("git -C /tmp/wt --no-pager rev-parse HEAD",
                        "git --git-dir=/tmp/x rev-parse HEAD"):
            with self.subTest(command=command):
                self.assertIsNone(inspector_readonly_guard.check(command))

    def test_required_global_option_is_not_satisfied_by_a_value(self):
        """**値として消費される位置では、必須 option は git へ届かない。**

        `git --namespace --no-optional-locks status`では`--no-optional-locks`が
        namespace の値になる。**集合検査のままでは「在る」と誤判定する。**
        """
        self.assertIsNotNone(
            inspector_readonly_guard.check("git --namespace --no-optional-locks status"))
        self.assertIsNone(
            inspector_readonly_guard.check("git --no-optional-locks status"))

    def test_required_helper_options_must_sit_right_after_the_subcommand(self):
        """**必須flagはsubcommandの直後2語でなければ、gitへ届かない**（実測）。

        - `git diff -- f --no-ext-diff` — `--`より後ろはpathspecである
        - `git log -S --no-ext-diff -p` — `-S`が次の語を検索文字列として飲む

        **値を取るoptionを数え上げる形にしない。**位置で決めれば前に何も置けない。
        """
        for command in ("git diff -- f --no-ext-diff --no-textconv",
                        "git log -S --no-ext-diff --no-textconv -p",
                        "git log --oneline --no-ext-diff --no-textconv"):
            with self.subTest(command=command):
                self.assertIsNotNone(inspector_readonly_guard.check(command))
        # 順序は問わない。**位置だけを見る。**
        self.assertIsNone(
            inspector_readonly_guard.check("git diff --no-ext-diff --no-textconv -- f"))
        self.assertIsNone(
            inspector_readonly_guard.check("git diff --no-textconv --no-ext-diff HEAD"))

    def test_allowed_commands_pass(self):
        for command in self.ALLOWED:
            with self.subTest(command=command):
                self.assertIsNone(inspector_readonly_guard.check(command))

    def test_denied_commands_are_refused(self):
        for command in self.DENIED:
            with self.subTest(command=command):
                self.assertIsNotNone(
                    inspector_readonly_guard.check(command),
                    f"{command!r}が通ってしまう",
                )

    def test_empty_command_passes(self):
        """空は拒否理由にしない。**hookは空commandを止める役ではない。**"""
        for command in ("", "   ", "\n"):
            with self.subTest(command=command):
                self.assertIsNone(inspector_readonly_guard.check(command))

    def test_newline_is_split_before_tokenizing(self):
        """**`shlex`は改行を空白として扱う。**行で分けないと2行目が見えない。

        分けずに`tokenize`へ渡すと`git show rm -rf /`という1つの語列になり、
        `rm`がcommand位置に来ない。**この形が実際に素通りすることを固定する。**
        """
        collapsed = command_line.programs("git show HEAD\nrm -rf /")
        self.assertEqual(collapsed, ["git"])
        self.assertIsNotNone(inspector_readonly_guard.check("git show HEAD\nrm -rf /"))

    def test_allowlist_holds_no_program_that_writes_on_its_own(self):
        """allowlistへ書き込めるcommandが紛れ込まないよう固定する。

        **将来`sort`や`tee`を足したくなったときに落ちる。**足すなら、その理由を
        ADR-0020へ書いたうえでこのtestも変える。
        """
        writable = {"sort", "uniq", "sed", "tee", "dd", "cp", "mv", "install",
                    "python3", "python", "sh", "bash", "zsh", "xargs", "awk",
                    "perl", "ruby", "node", "tar", "touch", "mkdir", "rm"}
        overlap = writable.intersection(inspector_readonly_guard.ALLOWED_PROGRAMS)
        self.assertEqual(overlap, set(), f"書き込める program が allowlist にある: {overlap}")

    def test_git_subcommands_hold_no_mutating_verb(self):
        mutating = {"add", "commit", "push", "fetch", "pull", "checkout", "switch",
                    "restore", "reset", "clean", "branch", "tag", "config",
                    "worktree", "stash", "gc", "am", "apply", "rebase", "merge",
                    "cherry-pick", "revert", "mv", "rm", "init", "clone"}
        overlap = mutating.intersection(inspector_readonly_guard.GIT_READONLY_SUBCOMMANDS)
        self.assertEqual(overlap, set(), f"状態を変える subcommand が allowlist にある: {overlap}")

    def test_deny_payload_is_emitted_for_a_write(self):
        """hookとして起動したとき、`permissionDecision: deny`を出す。"""
        payload = json.dumps({"tool_input": {"command": "rm -rf /"}})
        result = subprocess.run(
            [sys.executable, INSPECTOR_GUARD],
            input=payload, capture_output=True, text=True,
            encoding="utf-8", errors="replace", timeout=120,
        )
        self.assertEqual(result.returncode, 0, result.stderr[:300])
        emitted = json.loads(result.stdout)
        self.assertEqual(
            emitted["hookSpecificOutput"]["permissionDecision"], "deny")
        self.assertIn("rm", emitted["hookSpecificOutput"]["permissionDecisionReason"])

    def test_read_only_payload_passes_through(self):
        payload = json.dumps({"tool_input": {"command": "git show --no-ext-diff --no-textconv HEAD"}})
        result = subprocess.run(
            [sys.executable, INSPECTOR_GUARD],
            input=payload, capture_output=True, text=True,
            encoding="utf-8", errors="replace", timeout=120,
        )
        self.assertEqual(result.returncode, 0, result.stderr[:300])
        self.assertEqual(result.stdout.strip(), "")

    def test_both_inspector_agents_wire_the_guard(self):
        """**2つの agent 定義が実際にこのhookを呼んでいることを固定する。**

        hookを置いただけでは何も起きない。`.claude/agents/`のfrontmatterへ
        書かれていて初めて掛かる。**片方だけ書き忘れる形を止める。**
        """
        agents = REPO_ROOT_FOR_TEMPLATES / ".claude" / "agents"
        for name in ("consistency-inspector.md", "fresh-context-reviewer.md"):
            with self.subTest(agent=name):
                text = (agents / name).read_text(encoding="utf-8")
                self.assertIn("inspector_readonly_guard.py", text,
                              f"{name}がguardを呼んでいない")
                self.assertIn("PreToolUse", text, f"{name}のhookがPreToolUseでない")

    def test_transparent_prefixes_do_not_smuggle_an_allowed_program(self):
        """**`command_line`が透過させる前置語を、この guard は透過させない。**

        `sudo cat /etc/shadow`は`cat`だけを見れば allowlist を通る。
        `TRANSPARENT_PREFIXES`のすべてについて、command位置に現れたら落ちることを固定する。
        """
        for prefix in command_line.TRANSPARENT_PREFIXES:
            with self.subTest(prefix=prefix):
                self.assertIsNotNone(
                    inspector_readonly_guard.check(f"{prefix} cat AGENTS.md"),
                    f"{prefix}付きの呼び出しが通ってしまう",
                )

    def test_a_program_with_a_path_is_rejected(self):
        """**basenameで認可しない。**名前が一致するだけの別の実行fileである。

        `./git`は`git`という名前だが、`PATH`上のgitではない。**allowlistの前提
        （名前が実体を決める）が崩れる。**絶対pathも許さない。検査に要らない。
        """
        for command in ("./git show HEAD", "../git log", "/tmp/cat f", "/usr/bin/git status"):
            with self.subTest(command=command):
                reason = inspector_readonly_guard.check(command)
                self.assertIsNotNone(reason, f"{command!r}が通ってしまう")
                self.assertIn("path を含む program", reason)

    def test_rg_options_that_execute_a_command_are_rejected(self):
        """`rg --pre`は検索対象ごとにその command を起動する。`=`付きも同じ。

        **program名のallowlistだけでは足りないことを固定する。**
        """
        for command in (
            "rg --pre /tmp/evil.sh pattern",
            "rg --pre=/tmp/evil.sh pattern",
            "rg --hostname-bin /tmp/evil.sh pattern",
        ):
            with self.subTest(command=command):
                self.assertIsNotNone(inspector_readonly_guard.check(command))
        # **前方一致で誤爆させない。**`--pre-glob`はglobであってcommandではない
        self.assertIsNone(inspector_readonly_guard.check("rg --pre-glob *.md pattern"))

    def test_git_status_requires_the_global_no_optional_locks(self):
        """`git status`は既定でindexをrefreshし`.git/index`を書く。

        **`--no-optional-locks`はglobal optionである。**subcommandの後ろでは
        gitが受け付けないため、その形を「安全な形」として通さない。
        """
        self.assertIsNotNone(inspector_readonly_guard.check("git status"))
        self.assertIsNotNone(
            inspector_readonly_guard.check("git status --no-optional-locks")
        )
        self.assertIsNone(
            inspector_readonly_guard.check("git --no-optional-locks status")
        )

    def test_git_external_helpers_are_rejected_with_and_without_flags(self):
        """外部 helper は明示のoptionでも、option無しでも走る。

        **`--ext-diff`／`--textconv`／`--filters`を拒否するだけでは閉じない。**
        `git diff`は`diff.external`を、`diff`／`show`／`log`／`blame`はtextconvを
        **flag無しで走らせる**（実測）。両方の明示を要求する。
        **`--no-ext-diff`だけではtextconvを止められない。**
        """
        for command in (
            "git diff --ext-diff HEAD~1 HEAD",
            "git show --textconv HEAD:AGENTS.md",
            "git grep --textconv pattern",
            "git cat-file --filters HEAD:AGENTS.md",
        ):
            with self.subTest(explicit=command):
                self.assertIsNotNone(inspector_readonly_guard.check(command))
        for command in (
            "git diff HEAD~1 HEAD",
            "git show HEAD",
            "git log -p",
            "git blame AGENTS.md",
            "git diff --no-ext-diff HEAD~1 HEAD",
            "git show --no-textconv HEAD",
        ):
            with self.subTest(implicit=command):
                self.assertIsNotNone(
                    inspector_readonly_guard.check(command),
                    f"{command!r}が通ってしまう",
                )
        for subcommand in ("diff", "show", "log", "blame"):
            with self.subTest(allowed=subcommand):
                self.assertIsNone(
                    inspector_readonly_guard.check(
                        f"git {subcommand} --no-ext-diff --no-textconv"
                    )
                )

    def test_assignment_like_arguments_are_not_rejected(self):
        """**代入の判定はcommand位置だけに当てる。**

        全語へ当てると`grep FOO=bar file`のような読み取り専用commandまで落ちる。
        **誤検知はhookごと無効化される側の失敗である。**
        """
        self.assertIsNone(inspector_readonly_guard.check("grep FOO=bar AGENTS.md"))
        self.assertIsNone(
            inspector_readonly_guard.check(
                "git diff --no-ext-diff --no-textconv -- file=1"
            )
        )
        self.assertIsNotNone(inspector_readonly_guard.check("FOO=bar cat AGENTS.md"))

    def test_command_starts_keeps_the_prefixes_programs_drops(self):
        """`command_starts`が前置語を残し、`programs`がそれを落とすこと。

        **2つの関数が同じ走査から出ていることを固定する。**別実装にすると、
        片方だけが前置語を数え落とす。
        """
        starts = command_line.command_starts("sudo FOO=1 cat x && git show HEAD")
        self.assertEqual(starts, [(("sudo", "FOO=1"), "cat"), ((), "git")])
        self.assertEqual(
            command_line.programs("sudo FOO=1 cat x && git show HEAD"), ["cat", "git"])

    def test_command_starts_keeps_a_prefix_with_no_program(self):
        """program語が続かない前置語も落とさない。**捨てると呼び出し側が見逃す。**"""
        self.assertEqual(command_line.command_starts("sudo && ls"),
                         [(("sudo",), None), ((), "ls")])

    def _wrapper_command(self, agent):
        """agent frontmatterから`command:`のscalarを読む。

        **PyYAMLを使わない。**このリポジトリはYAML parserを依存に持たず、
        `validate_pages_output.py`も同じ理由でscalarを手で読んでいる。
        **testのためだけに依存を増やさない。**

        対象は`command: "..."`という1行のdouble-quoted scalarに限る。
        **形が変わったらこのtestは落ちる。**黙って読み飛ばさないよう、
        見つからなければ`fail`する。
        """
        text = (REPO_ROOT_FOR_TEMPLATES / ".claude" / "agents" / agent).read_text(
            encoding="utf-8")
        front = text[4:text.index("\n---\n", 4) + 1]
        for line in front.splitlines():
            stripped = line.strip()
            if not stripped.startswith('command: "'):
                continue
            scalar = stripped[len('command: "'):]
            self.assertTrue(scalar.endswith('"'), f"{agent}のcommandが1行で閉じていない")
            # double-quoted scalarのescapeを戻す。`\"`と`\\`だけを扱う。
            return scalar[:-1].replace('\\"', '"').replace("\\\\", "\\")
        self.fail(f"{agent}に`command: \"...\"`の行が無い")

    def _run_wrapper(self, agent, env_extra, payload):
        shell = shutil.which("sh") or "/bin/sh"
        return subprocess.run(
            [shell, "-c", self._wrapper_command(agent)],
            input=payload, capture_output=True, text=True,
            encoding="utf-8", errors="replace", timeout=120,
            env={**os.environ, **env_extra},
        )

    def test_wrapper_blocks_when_the_guard_cannot_start(self):
        """**起動できないときは fail closed である。**

        hookのwrapperが`exit 0`で終わると、guardが無い環境で`Bash`が素通りする。
        **`exit 2`だけがtool呼び出しを止める**（[公式文書](https://code.claude.com/docs/en/hooks)。
        他の非0は「blockしないerror」として扱われ、動作は続行する）。

        guardが無い場合とpython3が無い場合の両方を測る。
        """
        write = json.dumps({"tool_input": {"command": "rm -rf /"}})
        for agent in ("consistency-inspector.md", "fresh-context-reviewer.md"):
            with tempfile.TemporaryDirectory() as empty:
                with self.subTest(agent=agent, case="guard missing"):
                    result = self._run_wrapper(agent, {"CLAUDE_PROJECT_DIR": empty}, write)
                    self.assertEqual(result.returncode, 2, result.stderr[:300])
            with self.subTest(agent=agent, case="python3 missing"):
                result = self._run_wrapper(
                    agent,
                    {"CLAUDE_PROJECT_DIR": str(REPO_ROOT_FOR_TEMPLATES),
                     "PATH": "/nonexistent"},
                    write,
                )
                self.assertEqual(result.returncode, 2, result.stderr[:300])

    def test_wrapper_passes_through_a_working_guard(self):
        """**fail closedにしたことで、正常系まで落としていないことを測る。**"""
        env = {"CLAUDE_PROJECT_DIR": str(REPO_ROOT_FOR_TEMPLATES)}
        for agent in ("consistency-inspector.md", "fresh-context-reviewer.md"):
            with self.subTest(agent=agent, case="read-only"):
                result = self._run_wrapper(
                    agent, env, json.dumps({"tool_input": {"command": "git show --no-ext-diff --no-textconv HEAD"}}))
                self.assertEqual(result.returncode, 0, result.stderr[:300])
                self.assertEqual(result.stdout.strip(), "")
            with self.subTest(agent=agent, case="write"):
                result = self._run_wrapper(
                    agent, env, json.dumps({"tool_input": {"command": "rm -rf /"}}))
                self.assertEqual(result.returncode, 0, result.stderr[:300])
                emitted = json.loads(result.stdout)
                self.assertEqual(
                    emitted["hookSpecificOutput"]["permissionDecision"], "deny")

    def test_the_wrapper_does_not_fail_open(self):
        """`|| exit 0`という形が戻っていないことを、文字列でも固定する。"""
        for agent in ("consistency-inspector.md", "fresh-context-reviewer.md"):
            with self.subTest(agent=agent):
                command = self._wrapper_command(agent)
                self.assertNotIn("|| exit 0", command)
                self.assertIn("exit 2", command)

    def test_the_guard_is_not_wired_globally(self):
        """**`.claude/settings.json`へは置かない。**置くと通常の作業 session が止まる。"""
        settings = REPO_ROOT_FOR_TEMPLATES / ".claude" / "settings.json"
        self.assertNotIn("inspector_readonly_guard", settings.read_text(encoding="utf-8"))


if __name__ == "__main__":
    unittest.main(verbosity=2)
