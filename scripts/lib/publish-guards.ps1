# 公開前検査で共有する定数と判定。
# prepare-pages.ps1、validate-pages-output.ps1、validate-doc-links.ps1、
# test-link-validators.ps1 の4 scriptからdot-sourceする。
# 同じpatternを複数scriptで再定義しない（Governanceの Single Source of Truth）。

Set-StrictMode -Version Latest

# Provider既知のtoken形式と秘密鍵。
# GitHubのsecret scanningと重複するが、ここではPagesへ出す直前の最終確認として使う。
#
# `sk-(?:[A-Za-z0-9]+-)*[A-Za-z0-9]{20,}`はReDoSに見えるが、そうではない。
# 反復部が末尾に`-`を要求し、`-`は`[A-Za-z0-9]`に含まれないため、各反復の範囲は
# 次の`-`の位置で一意に決まる。分割の曖昧性がなく、`(a+)+`型のbacktrackが起きない。
#
# 2026-07-30の実測（.NET regex、非マッチ入力）:
#   `sk-` + `a-` x320 + `!`（644 char）  0.08 ms
#   `sk-` + `aaaaaaaaaa-` x80 + `!`（884 char）  0.08 ms
# 入力長を16倍にしても横ばいで、指数的増加は観測されない。
# atomic group（`(?>...)`）版は同条件でむしろ遅かった。
# 反復部の文字クラスへ`-`を追加する場合は曖昧になるため、この前提が崩れる。
$script:DeskCatSecretPattern = @(
    'gh[pousr]_[A-Za-z0-9]{20,}'
    'github_pat_[A-Za-z0-9_]{20,}'
    'AKIA[0-9A-Z]{16}'
    'AIza[0-9A-Za-z_\-]{35}'
    'sk-(?:[A-Za-z0-9]+-)*[A-Za-z0-9]{20,}'
    'xox[baprs]-[A-Za-z0-9-]{10,}'
    'glpat-[A-Za-z0-9_\-]{20,}'
    '-----BEGIN (?:RSA |EC |DSA |OPENSSH |PGP )?PRIVATE KEY-----'
    '(?im)^\s*(?:password|passwd|secret|api[_-]?key|access[_-]?token|token)\s*[:=]\s*\S{8,}'
) -join '|'

# 個人を特定しうる絶対path。Windows、Linux、macOS、UNC、file scheme。
# `github.com/users/<name>`はGitHubのuser-owned resource（Projects等）の正規URL構造であり、
# ローカルのホームディレクトリpathではないため、直前がgithub.comの場合は除外する。
$script:DeskCatPersonalPathPattern = @(
    '[A-Za-z]:\\Users\\[^\\\s]+'
    '/home/[^/\s]+'
    '(?<!github\.com)/Users/[^/\s]+'
    '\\\\[A-Za-z0-9._-]+\\[A-Za-z0-9$._-]+'
    'file://'
) -join '|'

# Pagesへ出してよいfile拡張子。
$script:DeskCatAllowedExtensions = @(
    '.css', '.gif', '.html', '.ico', '.jpeg', '.jpg', '.markdown', '.md',
    '.png', '.scss', '.svg', '.txt', '.webp', '.yaml', '.yml'
)

# 内容scanの対象とするtext拡張子。
$script:DeskCatTextExtensions = @(
    '.css', '.html', '.markdown', '.md', '.scss', '.svg', '.txt', '.yaml', '.yml'
)

# Pagesのroot直下へ複製する文書。
# `prepare-pages.ps1`が複製し、`validate-doc-links.ps1`が公開対象の判定に使う。
# 二箇所で列挙すると、追加・削除の一方だけが追従する。追加を落とせば公開済み文書への
# linkを不当に失敗させ、削除を落とせば公開していない文書へのlinkを通してしまう。
$script:DeskCatRootDocuments = @('README.md', 'AGENTS.md', 'CONTRIBUTING.md', 'SECURITY.md', 'LICENSE')

# 上のうち、複製はされるが生成siteでHTMLにならないもの。
# `README.md`はJekyllのreadme-indexとroot `index.md`が競合してpage URLを持たない。
# `CONTRIBUTING.md`も同様にHTML化されない（2026-07-29に公開siteで実測）。
# `LICENSE`はMarkdownではないためrender対象にならない。
# relative linkを張ると`.md`のまま残り、Pages output validationで失敗する。
# 参照する場合は絶対URLを使う。
$script:DeskCatUnrenderedRootDocuments = @('README.md', 'CONTRIBUTING.md', 'LICENSE')

# 大量欠落に気付くための下限。stagingするMarkdownと、生成siteのHTMLの両方へ適用する。
# 実際の件数から余裕を取った値であり、増えた分に追従して上げる必要はない。
# 2箇所で別々に持つと、片方だけ更新して検知力が食い違う。
$script:DeskCatMinimumPublishedCount = 35

# `docs/` から複製してよい拡張子。
# 画像はbinaryのため内容scanが効かない。承認なしに公開されることを防ぐため、
# ここでは複製せず、公開したい図版は pages/ 配下へ明示的に置く運用とする。
$script:DeskCatDocsCopyExtensions = @('.md', '.markdown')

# Markdownとして扱う拡張子。link検査の対象選別と、anchor検査が可能かどうかの
# 判定に使う。`DeskCatDocsCopyExtensions`とは目的が違うため別に定義する。
# 一方だけを変えたときに、もう一方が黙って追従しないようにする。
$script:DeskCatMarkdownExtensions = @('.md', '.markdown')

# `Get-Content -Raw`は空fileに対して$nullを返す。そのまま正規表現へ渡すと例外になり、
# 「問題を報告して失敗」ではなく「検査自体がcrash」する。空文字列へ正規化する。
function Get-DeskCatFileText {
    [CmdletBinding()]
    param([Parameter(Mandatory)][string]$Path)
    $text = Get-Content -LiteralPath $Path -Raw
    if ($null -eq $text) { return '' }
    return $text
}

# 指定pathspec配下でGitが追跡しているfileの集合を返す。
#
# 比較はcase-sensitive（`Ordinal`）にする。CIのubuntu-24.04はcase-sensitive
# filesystemであり、`OrdinalIgnoreCase`だとcase違いの未追跡fileを「追跡済み」と
# 誤判定し、reviewを経ていないfileが公開される。一致しなければ「未追跡」へ倒す
# fail-closedとする。
function Get-DeskCatTrackedFiles {
    [CmdletBinding()]
    param(
        [Parameter(Mandatory)][string]$RepositoryRoot,
        [Parameter(Mandatory)][string[]]$PathSpec
    )

    $tracked = [System.Collections.Generic.HashSet[string]]::new(
        [System.StringComparer]::Ordinal)

    # `--`でpathspecを明示する。省くと、環境によってpathspecが引数として
    # 解釈されず結果が空になることがある。`-`で始まるpathspecがoptionと
    # 誤解される問題も同時に防ぐ。
    #
    # `core.quotePath=false`を明示する。既定では非ASCIIのpathがdouble quoteと
    # octal escapeで出力され、実pathではなくescape sequenceを保持してしまう。
    # `Get-DeskCatTrackedSymlinks`も同じ設定で読み、両者のpath表記を揃える。
    $output = @(& git -C $RepositoryRoot -c core.quotePath=false ls-files -- @PathSpec 2>$null)
    if ($LASTEXITCODE -ne 0) {
        throw "Unable to enumerate tracked files under $($PathSpec -join ', '). Run inside a Git checkout."
    }

    foreach ($line in $output) {
        $null = $tracked.Add($line)
    }

    # comma演算子で返す。そのまま返すとPowerShellがHashSetを展開し、
    # 一致が1件のときだけ呼び出し側がStringを受け取る。すると`.Contains()`が
    # HashSet.Containsではなく String.Contains（部分一致）になり、
    # `LICENSE`が`LIC`を含むと判定して追跡判定が壊れる。0件なら$nullになり
    # StrictModeで`.Count`が例外になる。HashSetのまま返して両方を防ぐ。
    return ,$tracked
}

# Markdown inline linkのtargetを出現順に返す。
#
# 正規表現だけで文書全体から抽出すると、PowerShell／.NETのpatch版が異なる環境で
# 同一treeに対する走査件数が変わった。ここでは、このrepositoryが使用する
# `[text](target)`と`[text](target "title")`を線形走査し、runtime差を避ける。
# imageの`![text](target)`はlink検査の対象外とする。既存validatorと同じく、
# nested parenthesis、escaped closing parenthesis、single-quote titleは対象外である。
function Get-DeskCatMarkdownLinkTargets {
    [CmdletBinding()]
    [OutputType([string])]
    param([Parameter(Mandatory)][AllowEmptyString()][string]$Content)

    for ($i = 0; $i -lt $Content.Length; $i++) {
        if ($Content[$i] -ne [char]'[') { continue }
        if ($i -gt 0 -and $Content[$i - 1] -eq [char]'!') { continue }

        $closeBracket = $Content.IndexOf([char]']', $i + 1)
        if ($closeBracket -lt 0) { break }
        if ($closeBracket + 1 -ge $Content.Length -or
            $Content[$closeBracket + 1] -ne [char]'(') {
            $i = $closeBracket
            continue
        }

        $targetStart = $closeBracket + 2
        $targetEnd = $targetStart
        while ($targetEnd -lt $Content.Length -and
            $Content[$targetEnd] -ne [char]')' -and
            -not [char]::IsWhiteSpace($Content[$targetEnd])) {
            $targetEnd++
        }
        if ($targetEnd -eq $targetStart) {
            $i = $closeBracket
            continue
        }

        $closeParenthesis = -1
        if ($targetEnd -lt $Content.Length -and $Content[$targetEnd] -eq [char]')') {
            $closeParenthesis = $targetEnd
        }
        elseif ($targetEnd -lt $Content.Length -and
            [char]::IsWhiteSpace($Content[$targetEnd])) {
            $cursor = $targetEnd
            while ($cursor -lt $Content.Length -and
                [char]::IsWhiteSpace($Content[$cursor])) {
                $cursor++
            }
            if ($cursor -lt $Content.Length -and $Content[$cursor] -eq [char]'"') {
                $closeQuote = $Content.IndexOf([char]'"', $cursor + 1)
                if ($closeQuote -ge 0 -and $closeQuote + 1 -lt $Content.Length -and
                    $Content[$closeQuote + 1] -eq [char]')') {
                    $closeParenthesis = $closeQuote + 1
                }
            }
        }

        if ($closeParenthesis -lt 0) {
            $i = $closeBracket
            continue
        }

        Write-Output $Content.Substring($targetStart, $targetEnd - $targetStart)
        $i = $closeParenthesis
    }
}

# fenced code block内の行を除いた行を、出現順に返す。
#
# fence内の`#`はshell commentであり見出しではない。fence内の`[a](b)`もlinkの例示である。
# 見出し走査とlink走査で同じ判定を使う。2箇所で同じstate machineを持つと、
# 片方だけを変えたときにanchor集合とlink集合が別の行を見ることになる。
function Get-DeskCatMarkdownOutsideFences {
    [CmdletBinding()]
    [OutputType([string])]
    param([Parameter(Mandatory)][AllowEmptyString()][string]$Content)

    $inFence = $false
    foreach ($line in $Content -split "`n") {
        if ($line -match '^\s*(?:```|~~~)') {
            $inFence = -not $inFence
            continue
        }
        if (-not $inFence) { Write-Output $line }
    }
}

# Markdown見出しからGitHubが生成するanchorを求める。
# 小文字化し、記号を除去し、空白をhyphenへ変換する。CJKはそのまま残る。
# `validate-doc-links.ps1`がlinkのfragmentと突き合わせるために使う。
function Get-DeskCatHeadingAnchor {
    [CmdletBinding()]
    param([Parameter(Mandatory)][AllowEmptyString()][string]$Heading)

    # GitHubはunderscoreをslugへ残す。除去すると`#some_heading`が解決しない。
    $text = $Heading -replace '[`*\[\]()!"''.,:;/\|<>?~^{}+=&%$#@]', ''
    foreach ($c in @([char]0xFF0F, [char]0x3001, [char]0x3002, [char]0xFF08, [char]0xFF09)) {
        $text = $text.Replace([string]$c, '')
    }
    return $text.Trim().ToLowerInvariant() -replace '\s+', '-'
}

# Gitがsymlink（mode 120000）として記録しているpathの集合を返す。
#
# 作業ツリー上の実体はcheckout環境で変わる。`core.symlinks=false`のWindowsでは
# link先pathを内容とするregular fileになるため、file属性で判定すると環境ごとに
# 結果が変わる。indexのmodeは環境に依存しない。
function Get-DeskCatTrackedSymlinks {
    [CmdletBinding()]
    param([Parameter(Mandatory)][string]$RepositoryRoot)

    $symlinks = [System.Collections.Generic.HashSet[string]]::new(
        [System.StringComparer]::Ordinal)

    # quotingは`Get-DeskCatTrackedFiles`と同じ設定にする。片方だけがescapeされた
    # pathを返すと、symlink除外の突き合わせが非ASCII pathで一致しなくなる。
    $output = @(& git -C $RepositoryRoot -c core.quotePath=false ls-files -s 2>$null)
    if ($LASTEXITCODE -ne 0) {
        throw 'Unable to read the Git index. Run inside a Git checkout.'
    }

    foreach ($line in $output) {
        # `<mode> <sha> <stage>	<path>`
        if ($line -match '^120000\s+\S+\s+\d+	(?<path>.+)$') {
            $null = $symlinks.Add($Matches['path'])
        }
    }

    return ,$symlinks
}

function Test-DeskCatSecretLike {
    [CmdletBinding()]
    param([Parameter(Mandatory)][AllowEmptyString()][string]$Content)
    return [regex]::IsMatch($Content, $script:DeskCatSecretPattern)
}

function Test-DeskCatPersonalPath {
    [CmdletBinding()]
    param([Parameter(Mandatory)][AllowEmptyString()][string]$Content)
    return [regex]::IsMatch(
        $Content,
        $script:DeskCatPersonalPathPattern,
        [System.Text.RegularExpressions.RegexOptions]::IgnoreCase)
}

# path が root の内側かを判定する。
# StartsWith だけでは `_site` と `_site-old` を区別できないため、区切り文字まで含めて比較する。
function Test-DeskCatPathWithinRoot {
    [CmdletBinding()]
    param(
        [Parameter(Mandatory)][string]$Path,
        [Parameter(Mandatory)][string]$Root
    )
    # 比較はcase-sensitiveにする。CIのubuntu-24.04はcase-sensitive filesystemであり、
    # `OrdinalIgnoreCase`だと`_SITE`を`_site`の内側と誤判定する。
    # 判定を誤ると、artifactに含まれないfileへのlinkを検査が通してしまう。
    # 一致しなければ「範囲外」へ倒すfail-closedとする。
    $full = [System.IO.Path]::GetFullPath($Path)
    $rootFull = [System.IO.Path]::GetFullPath($Root).TrimEnd(
        [System.IO.Path]::DirectorySeparatorChar,
        [System.IO.Path]::AltDirectorySeparatorChar)
    if ($full.Equals($rootFull, [System.StringComparison]::Ordinal)) {
        return $true
    }
    $prefix = $rootFull + [System.IO.Path]::DirectorySeparatorChar
    return $full.StartsWith($prefix, [System.StringComparison]::Ordinal)
}

# root内のpathを、CIのOSに依存しない`/`区切りの相対pathへ変換する。
# validatorごとにSubstring／TrimStartを複製すると、root表記やseparatorの扱いが
# 食い違い、local絶対pathを診断へ戻してしまうため、公開scriptで共有する。
function Get-DeskCatPathRelativeToRoot {
    [CmdletBinding()]
    param(
        [Parameter(Mandatory)][string]$Path,
        [Parameter(Mandatory)][string]$Root
    )

    $fullPath = [System.IO.Path]::GetFullPath($Path)
    $rootPath = [System.IO.Path]::GetFullPath($Root)
    if (-not (Test-DeskCatPathWithinRoot -Path $fullPath -Root $rootPath)) {
        # helper自体がlocal pathを再掲すると、診断を相対化する目的を破る。
        throw 'Cannot format a path outside the publication root.'
    }

    return [System.IO.Path]::GetRelativePath($rootPath, $fullPath).Replace('\', '/')
}
