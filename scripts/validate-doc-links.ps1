# リポジトリ内のMarkdown相対linkを検査する。
#
# Pages workflowのlink checkは、公開対象（rootのMarkdownと`docs/`）だけを、
# HTML生成後に検査する。このscriptはそれを補い、`.github/`や各componentの
# READMEを含むリポジトリ全体を、生成前のMarkdownとして検査する。
#
# `pages/index.md`は`.pages-src/`のroot基準で書かれているため、
# repositoryのdirectory構造では解決できない。専用の基準pathで検査する。

[CmdletBinding()]
param(
    [string]$RepositoryRoot = ''
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

. (Join-Path $PSScriptRoot 'lib/publish-guards.ps1')

if ([string]::IsNullOrWhiteSpace($RepositoryRoot)) {
    $RepositoryRoot = Join-Path $PSScriptRoot '..'
}
$root = [System.IO.Path]::GetFullPath($RepositoryRoot)

if (-not (Test-Path -LiteralPath $root -PathType Container)) {
    throw 'Repository root does not exist.'
}

# 検査対象はGitが追跡しているMarkdownだけとする。
# directory走査にすると、生成物、worktree（`.claude/worktrees/`等）、vendor、
# 未追跡の作業用copyまで拾い、repositoryに存在しないfileで失敗する。
$problems = [System.Collections.Generic.List[string]]::new()
$checked = 0
# 検査したlinkの一覧。件数だけでは、環境によって走査範囲が変わっても
# `BROKEN=0`が揃うため同等性を確認できない。実際にWindowsで229件、
# Linux CIで243件と食い違った。内容のdigestを出して環境間で突き合わせる。
$checkedTargets = [System.Collections.Generic.List[string]]::new()
$scanned = 0

# 追跡fileの列挙はpublish-guards.ps1の共有helperを使う。ここで`git ls-files`を
# 直接呼ぶと、同じ呼び出しとerror処理を2箇所で持つことになる。
# helperはOrdinal比較のHashSetを返すため、未解決merge中にstage 1/2/3で
# 重複するpathもここで解消される。
# `@()`で包まない。helperはHashSetをそのまま返すため、包むと1要素の配列になり、
# 列挙がHashSet自身になって走査対象が消える。
#
# 拡張子の絞り込みはgitのpathspecに任せず、PowerShell側で行う。
# 過去のLinux CIでglob pathspecを渡したhelperの結果が0件になったため、
# pathspecのglob解釈や引数展開に依存しない形にして、環境差で走査対象が消えるのを防ぐ。
$trackedSymlinks = Get-DeskCatTrackedSymlinks -RepositoryRoot $root
$trackedAll = Get-DeskCatTrackedFiles -RepositoryRoot $root -PathSpec '.'
$tracked = [System.Collections.Generic.HashSet[string]]::new([System.StringComparer]::Ordinal)
foreach ($entry in $trackedAll) {
    if ($trackedSymlinks.Contains($entry)) { continue }
    if ([System.IO.Path]::GetExtension($entry).ToLowerInvariant() -in
        $script:DeskCatMarkdownExtensions) {
        $null = $tracked.Add($entry)
    }
}
if ($tracked.Count -eq 0) {
    # 0件のときは、gitが何を見ているかを添える。pathspecの解釈、
    # repository rootの解決、indexの中身のどれが原因かをlogから切り分ける。
    $allTracked = @(& git -C $root ls-files 2>&1)
    $allExit = $LASTEXITCODE
    $topLevelOutput = (& git -C $root rev-parse --show-toplevel 2>&1) -join ' '
    $topLevelExit = $LASTEXITCODE
    $pathComparison = if ($IsWindows) {
        [System.StringComparison]::OrdinalIgnoreCase
    }
    else {
        [System.StringComparison]::Ordinal
    }
    $topLevelMatchesRoot = $false
    if ($topLevelExit -eq 0 -and -not [string]::IsNullOrWhiteSpace($topLevelOutput)) {
        $topLevelMatchesRoot = [System.IO.Path]::GetFullPath($topLevelOutput).Equals(
            $root,
            $pathComparison)
    }
    $gitVersion = (& git --version 2>&1) -join ' '
    throw ("Unable to enumerate tracked Markdown files.`n" +
        "  ls-files(all) exit=$allExit count=$($allTracked.Count) -> $($allTracked -join ', ')`n" +
        "  rev-parse exit=$topLevelExit matches-root=$topLevelMatchesRoot`n" +
        "  $gitVersion")
}

# symlinkは走査しない。`CLAUDE.md`は`AGENTS.md`へのsymlinkであり、
# symlinkを解決する環境では同じ内容を2回走査して件数が二重になる。
# 実際にWindows（`core.symlinks=false`）で229件、Linux CIで243件と食い違い、
# 差の14件はすべて`CLAUDE.md`だった。
#
# 判定はfile属性ではなくGitのmode（120000）で行う。作業ツリー上の実体は
# checkout環境で変わるため、属性で判定すると走査対象そのものが環境ごとに変わる。
$markdownFiles = @(
    $tracked |
        ForEach-Object { Join-Path $root $_ } |
        Where-Object { Test-Path -LiteralPath $_ -PathType Leaf } |
        ForEach-Object { Get-Item -LiteralPath $_ }
)

# Pagesへ複製されるfileと、そのうちHTML化されないもの。
# 定義はpublish-guards.ps1にあり、prepare-pages.ps1のcopy対象と同一の変数を使う。
$publishedRootDocuments = $script:DeskCatRootDocuments
$unrenderedRootDocuments = $script:DeskCatUnrenderedRootDocuments

# 走査対象が0件なら、検査が働いていない。追跡fileの列挙か絞り込みが壊れている。
# 実際に一度、helperの戻り値を`@()`で包んだことで列挙がHashSet自身になり、
# 0件のまま「BROKEN=0」を報告した。0件を正常として通さない。
if ($markdownFiles.Count -eq 0) {
    throw 'No tracked Markdown files resolved. The link check is not working.'
}

# 各fileの見出しから生成されるanchor集合。linkのfragmentがここに無ければ、
# 生成siteでpage内jumpが解決しない。fileが存在するだけでは検出できないため、
# link先の存在確認とは別に突き合わせる。過去に見出しの改名で2度壊している。
$anchorsByFile = @{}
foreach ($file in $markdownFiles) {
    $set = [System.Collections.Generic.HashSet[string]]::new([System.StringComparer]::Ordinal)
    # GitHubは同一text の見出しが繰り返されると`-1`、`-2`と採番する。
    # 出現回数を数えないと、2つ目以降の見出しへのlinkを未解決と誤判定する。
    $seen = @{}
    # fenced code block内の`#`で始まる行はshell commentであり見出しではない。
    # 数えると偽のanchorが増え、同名見出しの`-1`／`-2`採番がずれる。
    # このrepositoryのgithub-wiki-home.mdは、実際にbash commentを10行以上含む。
    $outsideFences = @(Get-DeskCatMarkdownOutsideFences `
            -Content (Get-DeskCatFileText -Path $file.FullName))
    foreach ($line in $outsideFences) {
        if ($line -match '^#{1,6}\s+(?<heading>.+?)\s*$') {
            $base = Get-DeskCatHeadingAnchor -Heading $Matches['heading']
            $n = if ($seen.ContainsKey($base)) { $seen[$base] } else { 0 }
            $anchor = if ($n -eq 0) { $base } else { "$base-$n" }
            $seen[$base] = $n + 1
            $null = $set.Add($anchor)
        }
    }
    $anchorsByFile[$file.FullName] = $set
}

foreach ($file in $markdownFiles) {
    $scanned++
    $relativeFile = Get-DeskCatPathRelativeToRoot -Path $file.FullName -Root $root
    $normalizedFile = $relativeFile
    # fenced code block内はlinkの例示でありlinkではない。走査対象から除く。
    # 見出し検出と同じhelperを使い、両者が同じ行を見ることを保証する。
    $content = @(Get-DeskCatMarkdownOutsideFences `
            -Content (Get-DeskCatFileText -Path $file.FullName)) -join "`n"

    # `pages/index.md`と`pages/404.md`はstaging後のroot基準で解決する。
    $isPortal = $normalizedFile -cmatch '^pages/(index|404)\.md$'
    $baseDirectory = if ($isPortal) { $root } else { $file.DirectoryName }

    # 公開対象の文書か。ここからのlinkは生成site上でも解決できなければならない。
    # 比較はcase-sensitiveにする。`-like`と`-contains`は既定でcase-insensitiveであり、
    # CIのcase-sensitive filesystemでは`Docs/`のようなpathを公開対象と誤判定する。
    # 一致しなければ「非公開」へ倒すfail-closedとする。
    $isPublishedSource = (
        $isPortal -or
        $normalizedFile -clike 'docs/*' -or
        @($publishedRootDocuments | Where-Object { $_ -ceq $normalizedFile }).Count -gt 0
    )

    # 生成siteでHTMLになる文書か。HTML化されない文書のlinkは生成siteに現れないため、
    # 「HTML化されないtargetを参照するな」という制約の対象外とする。
    # それらのlinkはGitHubのrepository画面でだけ解決され、そこでは正しく動く。
    $isRenderedSource = $isPublishedSource -and (@($unrenderedRootDocuments | Where-Object { $_ -ceq $normalizedFile }).Count -eq 0)

    foreach ($targetValue in @(Get-DeskCatMarkdownLinkTargets -Content $content)) {
        $target = $targetValue.Trim()

        if ($target -match '^(?i:https?|mailto|tel|data):' -or
            $target.StartsWith('//') -or
            $target.StartsWith('{{')) {
            continue
        }

        $parts = $target -split '#', 2
        $path = $parts[0]
        $fragment = if ($parts.Count -gt 1) { $parts[1] } else { '' }
        $isSamePageFragment = (
            [string]::IsNullOrWhiteSpace($path) -and
            -not [string]::IsNullOrWhiteSpace($fragment)
        )
        if ([string]::IsNullOrWhiteSpace($path) -and -not $isSamePageFragment) {
            continue
        }

        $candidate = if ($isSamePageFragment) {
            $file.FullName
        }
        else {
            $path = [System.Uri]::UnescapeDataString($path)
            if ($path.StartsWith('/')) {
                Join-Path $root $path.TrimStart('/')
            }
            else {
                Join-Path $baseDirectory $path
            }
        }

        $checked++
        $checkedTargets.Add("${normalizedFile}|${target}")
        if (-not (Test-Path -LiteralPath $candidate)) {
            $hint = if ($isPortal) { ' (resolved against staging root)' } else { '' }
            $problems.Add("Broken link in ${relativeFile}: $target$hint")
            continue
        }

        # fragmentが見出しに対応するか。
        #
        # `$anchorsByFile`は追跡下でsymlinkでないMarkdownだけを持つ。targetがそこに
        # 無いままskipすると、`CLAUDE.md`のようなsymlinkや追跡外のMarkdownへの
        # fragmentが無検査で通る。このscriptの他の判定はfail-closedであり、
        # ここだけfail-openにしない。検査できない事実を報告する。
        if (-not [string]::IsNullOrWhiteSpace($fragment)) {
            $candidateFull = [System.IO.Path]::GetFullPath($candidate)
            $wanted = [System.Uri]::UnescapeDataString($fragment).ToLowerInvariant()
            if ($anchorsByFile.ContainsKey($candidateFull)) {
                if (-not $anchorsByFile[$candidateFull].Contains($wanted)) {
                    $problems.Add("Broken anchor in ${relativeFile}: $target (no matching heading)")
                    continue
                }
            }
            elseif ([System.IO.Path]::GetExtension($candidateFull).ToLowerInvariant() -in
                $script:DeskCatMarkdownExtensions) {
                $problems.Add(
                    "Unverifiable anchor in ${relativeFile}: $target " +
                    '(target Markdown is not a tracked non-symlink file)')
                continue
            }
        }

        # 同一page内のfragmentは上で検証済みであり、公開先もsource自身である。
        # repository rootからtarget pathを再分類する必要はない。
        if ($isSamePageFragment) {
            continue
        }

        # 公開文書から非公開pathへの相対linkを、Jekyll build前に検出する。
        # `.github/`や`scripts/`はPagesへ複製されないため、生成siteでは解決できない。
        # validate-pages-output.ps1も検出するが、そちらはbuild後で feedback が遅い。
        if ($isPublishedSource) {
            # repository外に実在するfileへのrelative link（`../../outside.md`など）は
            # Test-Pathを通過する。root基準のrelative pathを作る前に範囲を確認しないと、
            # Substringが例外になり、意図した診断ではなく検査自体のcrashになる。
            $candidateFull = [System.IO.Path]::GetFullPath($candidate)
            if (-not (Test-DeskCatPathWithinRoot -Path $candidateFull -Root $root)) {
                $problems.Add("Published doc ${relativeFile} links outside the repository: $target (use an absolute URL)")
                continue
            }
            $targetNormalized = Get-DeskCatPathRelativeToRoot -Path $candidateFull -Root $root
            # `docs/`配下でも、prepare-pages.ps1が複製するのはMarkdownだけである。
            # 存在するだけで公開対象とみなすと、docs/配下の画像等へのlinkが
            # 生成siteで404になる。directory targetは生成siteのindexへ解決される。
            $targetIsDirectory = Test-Path -LiteralPath $candidate -PathType Container
            $targetExtension = [System.IO.Path]::GetExtension($targetNormalized).ToLowerInvariant()
            # `docs`単体（`docs`、`docs/`、`./docs/`）はdirectory自身を指す。
            # `-like 'docs/*'`は区切り以降を要求するため一致せず、実在するdirectoryを
            # 未公開と誤判定する。比較はcase-sensitiveにして、一致しなければ
            # 「未公開」へ倒すfail-closedを保つ。
            $isPublishedTarget = (
                (($targetNormalized -ceq 'docs' -or $targetNormalized -clike 'docs/*') -and
                    ($targetIsDirectory -or $targetExtension -in $script:DeskCatDocsCopyExtensions)) -or
                @($publishedRootDocuments | Where-Object { $_ -ceq $targetNormalized }).Count -gt 0
            )
            if (-not $isPublishedTarget) {
                $problems.Add("Published doc ${relativeFile} links to unpublished path: $target (use an absolute URL)")
            }
            elseif ($isRenderedSource -and @($unrenderedRootDocuments | Where-Object { $_ -ceq $targetNormalized }).Count -gt 0) {
                $problems.Add("Published doc ${relativeFile} links to a root document that is not rendered as HTML: $target (use an absolute URL)")
            }
        }
    }
}

if ($problems.Count -gt 0) {
    $problems | Sort-Object -Unique | ForEach-Object { [Console]::Error.WriteLine($_) }
    throw "Documentation link validation failed with $($problems.Count) problem(s)."
}

# 並び替えはOrdinalで行う。`Sort-Object -CaseSensitive`はculture依存で、
# WindowsとLinuxで順序が変わりdigestが一致しない。
$checkedTargets.Sort([System.StringComparer]::Ordinal)
$digestSource = ($checkedTargets -join "`n")
$sha = [System.Security.Cryptography.SHA256]::Create()
try {
    $digest = [System.BitConverter]::ToString(
        $sha.ComputeHash([System.Text.Encoding]::UTF8.GetBytes($digestSource))).Replace('-', '')
}
finally {
    $sha.Dispose()
}
Write-Output "MARKDOWN=$scanned LINKS=$checked BROKEN=0 DIGEST=$($digest.Substring(0, 16))"
