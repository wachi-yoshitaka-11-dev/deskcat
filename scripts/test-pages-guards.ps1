[CmdletBinding()]
param()

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

# `prepare-pages.ps1`の公開境界が、想定した入力で失敗することを確認する。
# 依存toolを増やさないため、test frameworkは使わずplain PowerShellで書く。
#
# 各caseは一時fileまたはmanifestの一時改変で異常状態を作り、`finally`で
# 必ず元へ戻す。Gitのindexは変更しない。

$repositoryRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$preparePath = Join-Path $PSScriptRoot 'prepare-pages.ps1'
$stagingRoot = Join-Path $repositoryRoot '.pages-src'
$manifestPath = Join-Path $repositoryRoot 'pages/assets-manifest.psd1'
$assetsRoot = Join-Path $repositoryRoot 'pages/assets'
$docsRoot = Join-Path $repositoryRoot 'docs'

$portalPagePath = Join-Path $repositoryRoot 'pages/404.md'

# 追跡状態のguardを検証するために、実indexの場所を解決しておく。
# worktreeでは`.git`がfileのため、pathは`rev-parse`に決めさせる。
$gitIndexPath = & git -C $repositoryRoot rev-parse --git-path index
if ($LASTEXITCODE -ne 0) {
    throw 'Unable to locate the Git index. Run inside a Git checkout.'
}
if (-not [System.IO.Path]::IsPathRooted($gitIndexPath)) {
    $gitIndexPath = Join-Path $repositoryRoot $gitIndexPath
}
$gitIndexPath = [System.IO.Path]::GetFullPath($gitIndexPath)

$manifestBackup = Get-Content -LiteralPath $manifestPath -Raw
$portalPageBackup = Get-Content -LiteralPath $portalPagePath -Raw
$temporaryPaths = [System.Collections.Generic.List[string]]::new()
$results = [System.Collections.Generic.List[string]]::new()
$failed = 0
$skipped = 0

function Invoke-Prepare {
    $output = & pwsh -NoProfile -File $preparePath 2>&1
    return [pscustomobject]@{
        ExitCode = $LASTEXITCODE
        Output   = (($output | ForEach-Object { $_.ToString() }) -join "`n")
    }
}

function Test-PrepareOutputExposesStagingRoot {
    param([Parameter(Mandatory)][string]$Output)

    $comparison = if ($IsWindows) {
        [System.StringComparison]::OrdinalIgnoreCase
    }
    else {
        [System.StringComparison]::Ordinal
    }
    $normalizedOutput = $Output.Replace('\', '/')
    $normalizedStagingRoot = $script:stagingRoot.Replace('\', '/')
    return $normalizedOutput.IndexOf($normalizedStagingRoot, $comparison) -ge 0
}

function Test-Case {
    param(
        [Parameter(Mandatory)][string]$Name,
        [Parameter(Mandatory)][scriptblock]$Arrange,
        [string]$ExpectedMessage,
        [string[]]$ForbiddenMessages = @(),
        # Stagingが成功したうえで、生成物の内容まで確認したいcaseで使う。
        # 問題があれば理由文字列を、無ければ$nullを返す。
        [scriptblock]$AssertStaged
    )

    try {
        & $Arrange
        $run = Invoke-Prepare

        if (Test-PrepareOutputExposesStagingRoot -Output $run.Output) {
            $script:failed++
            $script:results.Add("FAIL  $Name -- local staging path was exposed in prepare output")
            return
        }
        foreach ($forbiddenMessage in $ForbiddenMessages) {
            if (-not [string]::IsNullOrWhiteSpace($forbiddenMessage) -and
                $run.Output -match [regex]::Escape($forbiddenMessage)) {
                $script:failed++
                $script:results.Add("FAIL  $Name -- forbidden text was exposed in prepare output")
                return
            }
        }

        if ([string]::IsNullOrEmpty($ExpectedMessage)) {
            if ($run.ExitCode -ne 0) {
                $script:failed++
                $script:results.Add("FAIL  $Name -- expected success, got exit $($run.ExitCode)")
                $script:results.Add("      $($run.Output)")
                return
            }

            # 成功caseはすべて、prepareが報告した相対pathをrepository rootから解決する。
            # callerのcurrent directoryから解決すると、repository外からtestを起動した
            # ときだけ存在しないpathを見て誤判定する。
            if ($run.Output -notmatch 'PAGES_SOURCE=(?<path>.+)') {
                $script:failed++
                $script:results.Add("FAIL  $Name -- PAGES_SOURCE not found in output")
                return
            }
            $reportedStagedRoot = $Matches['path'].TrimEnd([char]13, [char]10, ' ')
            if ([System.IO.Path]::IsPathRooted($reportedStagedRoot)) {
                $script:failed++
                $script:results.Add("FAIL  $Name -- PAGES_SOURCE must be repository-relative")
                return
            }
            $stagedRoot = [System.IO.Path]::GetFullPath(
                (Join-Path $script:repositoryRoot $reportedStagedRoot))
            $comparison = if ($IsWindows) {
                [System.StringComparison]::OrdinalIgnoreCase
            }
            else {
                [System.StringComparison]::Ordinal
            }
            if (-not $stagedRoot.Equals($script:stagingRoot, $comparison)) {
                $script:failed++
                $script:results.Add("FAIL  $Name -- PAGES_SOURCE did not resolve to the staging root")
                return
            }
            if (-not (Test-Path -LiteralPath $stagedRoot -PathType Container)) {
                $script:failed++
                $script:results.Add("FAIL  $Name -- reported staging root does not exist")
                return
            }

            if ($AssertStaged) {
                # staging先に加えてprepareの出力も渡す。どのguardが働いたかを
                # skip理由で確認するcaseがあるため。`$run.Output`はstderrを含む。
                $reason = & $AssertStaged $stagedRoot $run.Output
                if ($reason) {
                    $script:failed++
                    $script:results.Add("FAIL  $Name -- $reason")
                    return
                }
            }

            $script:results.Add("PASS  $Name")
            return
        }

        if ($run.ExitCode -eq 0) {
            $script:failed++
            $script:results.Add("FAIL  $Name -- expected failure, but staging succeeded")
        }
        elseif ($run.Output -notmatch [regex]::Escape($ExpectedMessage)) {
            $script:failed++
            $script:results.Add("FAIL  $Name -- expected message '$ExpectedMessage'")
            $script:results.Add("      $($run.Output)")
        }
        else {
            $script:results.Add("PASS  $Name")
        }
    }
    finally {
        # 次のcaseが実indexから始まるよう、必ず解除する。
        if (Test-Path Env:GIT_INDEX_FILE) {
            Remove-Item Env:GIT_INDEX_FILE
        }
        Set-Content -LiteralPath $manifestPath -Value $manifestBackup -NoNewline
        Set-Content -LiteralPath $portalPagePath -Value $portalPageBackup -NoNewline
        foreach ($path in $temporaryPaths) {
            if (Test-Path -LiteralPath $path) {
                Remove-Item -LiteralPath $path -Force -Recurse
            }
        }
        $temporaryPaths.Clear()
    }
}

# 追跡状態を実indexと切り離して操作する。indexを複製して`GIT_INDEX_FILE`で
# 切り替えるため、実indexとworking treeはどちらも変更しない。
# 環境変数は子processへ引き継がれ、`prepare-pages.ps1`内の`git ls-files`も
# この複製indexを読む。解除はTest-Caseのfinallyで行う。
function New-DetachedIndex {
    $copy = Join-Path ([System.IO.Path]::GetTempPath()) `
        ('deskcat-guard-index-' + [guid]::NewGuid().ToString('N'))
    Copy-Item -LiteralPath $gitIndexPath -Destination $copy -Force
    $temporaryPaths.Add($copy)
    $env:GIT_INDEX_FILE = $copy
}

# 「fileは存在するがindexに無い」状態を作る。
function Use-DetachedIndexWithout {
    param([Parameter(Mandatory)][string]$RepositoryPath)

    New-DetachedIndex
    & git -C $repositoryRoot update-index --force-remove $RepositoryPath
    if ($LASTEXITCODE -ne 0) {
        throw "Unable to build a detached index without ${RepositoryPath}."
    }
}

# 逆に、working treeの一時fileを追跡済みとして見せる。
# 追跡checkを通過しないと到達しない判定（拡張子check等）を試験するために使う。
function Use-DetachedIndexWith {
    param([Parameter(Mandatory)][string]$RepositoryPath)

    New-DetachedIndex
    & git -C $repositoryRoot update-index --add -- $RepositoryPath
    if ($LASTEXITCODE -ne 0) {
        throw "Unable to build a detached index containing ${RepositoryPath}."
    }
}

function Add-ManifestEntry {
    param(
        [Parameter(Mandatory)][string]$Body
    )
    $content = Get-Content -LiteralPath $manifestPath -Raw
    $updated = $content -replace '(?s)(\r?\n\s*\)\s*\r?\n\}\s*)$', "`n        $Body`n    )`n}`n"
    if ($updated -eq $content) {
        throw 'Unable to inject a manifest entry. The manifest layout changed.'
    }
    Set-Content -LiteralPath $manifestPath -Value $updated -NoNewline
}

try {
    # staging path検出器のpositive／negative control。検出器がno-opでも、
    # prepare側の診断がたまたまcleanなら全caseが成功するため、検出力を先に確認する。
    $renderedStagingProbe = $stagingRoot.Replace('\', '/')
    if ($IsWindows) {
        $renderedStagingProbe = $renderedStagingProbe.ToUpperInvariant()
    }
    if (-not (Test-PrepareOutputExposesStagingRoot -Output "synthetic: $renderedStagingProbe") -or
        (Test-PrepareOutputExposesStagingRoot -Output 'synthetic: .pages-src')) {
        throw 'Prepare-output staging-path detection precondition failed.'
    }
    $results.Add('PASS  publish guard harness detects private staging paths without false positives')

    Test-Case -Name 'baseline staging succeeds' -Arrange {}

    Test-Case -Name 'undeclared asset on disk fails' -ExpectedMessage 'Asset is not declared in the manifest' -Arrange {
        $path = Join-Path $assetsRoot '__guardtest-undeclared.png'
        Set-Content -LiteralPath $path -Value 'not a real png' -NoNewline
        $temporaryPaths.Add($path)
    }

    Test-Case -Name 'declared asset missing on disk fails' -ExpectedMessage 'Declared asset is missing' -Arrange {
        Add-ManifestEntry -Body "@{ Path = '__guardtest-absent.png'; Sha256 = '00' }"
    }

    Test-Case -Name 'declared asset not tracked by Git fails' -ExpectedMessage 'Declared asset is not tracked by Git' -Arrange {
        $path = Join-Path $assetsRoot '__guardtest-untracked.png'
        Set-Content -LiteralPath $path -Value 'not a real png' -NoNewline
        $temporaryPaths.Add($path)
        Add-ManifestEntry -Body "@{ Path = '__guardtest-untracked.png'; Sha256 = '00' }"
    }

    Test-Case -Name 'asset SHA-256 mismatch fails' -ExpectedMessage 'Asset SHA-256 does not match the manifest' -Arrange {
        $content = Get-Content -LiteralPath $manifestPath -Raw
        $updated = $content -replace '615063ED[0-9A-Fa-f]+', ('0' * 64)
        if ($updated -eq $content) {
            throw 'Unable to tamper with the recorded SHA-256. The manifest layout changed.'
        }
        Set-Content -LiteralPath $manifestPath -Value $updated -NoNewline
    }

    Test-Case -Name 'binary asset without SHA-256 fails' -ExpectedMessage 'Binary asset must declare Sha256' -Arrange {
        $content = Get-Content -LiteralPath $manifestPath -Raw
        $updated = $content -replace "(?m)^\s*Sha256\s*=.*$", ''
        if ($updated -eq $content) {
            throw 'Unable to remove the recorded SHA-256. The manifest layout changed.'
        }
        Set-Content -LiteralPath $manifestPath -Value $updated -NoNewline
    }

    Test-Case -Name 'disallowed staged file type fails with a relative path' `
        -ExpectedMessage 'File type is not approved for Pages: assets/__guardtest-disallowed.pdf' -Arrange {
        $path = Join-Path $assetsRoot '__guardtest-disallowed.pdf'
        Set-Content -LiteralPath $path -Value 'synthetic PDF fixture' -NoNewline
        $temporaryPaths.Add($path)
        Use-DetachedIndexWith -RepositoryPath 'pages/assets/__guardtest-disallowed.pdf'
        $hash = (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash
        Add-ManifestEntry -Body "@{ Path = '__guardtest-disallowed.pdf'; Sha256 = '$hash' }"
    }

    # Size上限は`pages/404.md`で検証する。`docs/`配下の一時fileはGit追跡外のため
    # 複製されず、size検査へ到達しない。`pages/404.md`は追跡済みのため、
    # 内容を書き換えてもstagingされ、size検査へ到達する。
    Test-Case -Name 'oversized staged file fails' `
        -ExpectedMessage 'File exceeds the Pages size limit: 404.md' -Arrange {
        $filler = ("`n" + ('x' * 1000)) * 1100
        Set-Content -LiteralPath $portalPagePath -Value ($portalPageBackup + $filler) -NoNewline
    }

    $stagedSecret = 'ghp_' + ('e' * 24)
    Test-Case -Name 'secret-like staged content fails without exposing values or local paths' `
        -ExpectedMessage 'Secret-like content detected: 404.md' `
        -ForbiddenMessages @($stagedSecret) -Arrange {
        Set-Content -LiteralPath $portalPagePath -Value "# Synthetic`n$stagedSecret" -NoNewline
    }

    $stagedPersonalPath = '/home/exampleuser/staged.md'
    Test-Case -Name 'personal path in staged content fails without exposing values or local paths' `
        -ExpectedMessage 'Personal absolute path detected: 404.md' `
        -ForbiddenMessages @($stagedPersonalPath) -Arrange {
        Set-Content -LiteralPath $portalPagePath -Value "# Synthetic`n$stagedPersonalPath" -NoNewline
    }

    # ここから下は`docs/`側の公開境界。`pages/assets/`の境界と対で維持する。
    Test-Case -Name 'untracked docs file is not published' -Arrange {
        $path = Join-Path $docsRoot '__guardtest-untracked.md'
        Set-Content -LiteralPath $path -Value '# guard test' -NoNewline
        $temporaryPaths.Add($path)
    } -AssertStaged {
        param($stagedRoot)
        $leaked = Join-Path $stagedRoot 'docs/__guardtest-untracked.md'
        if (Test-Path -LiteralPath $leaked) {
            return 'Untracked docs file was published.'
        }
        return $null
    }

    # `prepare-pages.ps1`の判定順は、Gitのmode 120000によるsymlink →
    # file属性のreparse point → Git追跡 → 拡張子の4段である。
    # 未追跡のfileは追跡checkで弾かれ、拡張子checkへ到達しない。
    # 拡張子checkを実際に通すため、複製indexへ追加して追跡済みに見せる。
    Test-Case -Name 'non-Markdown docs file is not published' -Arrange {
        $path = Join-Path $docsRoot '__guardtest-note.txt'
        Set-Content -LiteralPath $path -Value 'guard test' -NoNewline
        $temporaryPaths.Add($path)
        Use-DetachedIndexWith -RepositoryPath 'docs/__guardtest-note.txt'
    } -AssertStaged {
        param($stagedRoot)
        $leaked = Join-Path $stagedRoot 'docs/__guardtest-note.txt'
        if (Test-Path -LiteralPath $leaked) {
            return 'Non-Markdown docs file was published.'
        }
        return $null
    }

    # portal fileとroot documentの追跡guard。`docs/`と`pages/assets/`の境界と対で維持する。
    # これらは公開に必須のため、skipではなくthrowで止める。
    Test-Case -Name 'untracked portal file fails' `
        -ExpectedMessage 'Required Pages source is not tracked by Git: pages/404.md' `
        -ForbiddenMessages @($portalPagePath) -Arrange {
        Use-DetachedIndexWithout -RepositoryPath 'pages/404.md'
    }

    $agentsPath = Join-Path $repositoryRoot 'AGENTS.md'
    Test-Case -Name 'untracked root document fails' `
        -ExpectedMessage 'Required root document is not tracked by Git: AGENTS.md' `
        -ForbiddenMessages @($agentsPath) -Arrange {
        Use-DetachedIndexWithout -RepositoryPath 'AGENTS.md'
    }

    # symlinkとreparse pointの拒否経路。上記4段のうち先頭2つを個別に通す。
    # **この順序に依存している。**下のreparse case（indexへmode 100644で記録）は、
    # symlink判定を素通りしてreparse point判定へ到達することで成立する。
    # 順序を入れ替えると、どちらのcaseもskip理由の照合で落ちる。
    # Windowsでのsymlink作成にはDeveloper Modeまたは管理者権限が必要なため、
    # 作成できない環境ではSKIPとして記録する。実行していないものをPASSにしない。
    # CIのubuntu runnerでは常に実行される。
    $linkPath = Join-Path $docsRoot '__guardtest-link.md'
    $linkTarget = Join-Path $repositoryRoot 'README.md'
    $canSymlink = $false
    try {
        # 作る前に登録する。作成と削除の間で異常終了すると、`docs/`配下に
        # symlinkが残り、commitへ入りうる。`finally`が拾えるようにしておく。
        $temporaryPaths.Add($linkPath)
        $null = New-Item -ItemType SymbolicLink -Path $linkPath -Target $linkTarget -ErrorAction Stop
        $canSymlink = $true
        Remove-Item -LiteralPath $linkPath -Force
        $null = $temporaryPaths.Remove($linkPath)
    }
    catch {
        # `if ($canSymlink)`が2 caseを囲むため、2件分を計上する。1件で数えると、
        # 実行しなかった範囲を実際より狭く報告することになる。
        $skipped += 2
        $results.Add('SKIP  symlink recorded in the index is not published -- symlink creation unavailable')
        $results.Add('SKIP  reparse point recorded as a regular file is not published -- symlink creation unavailable')
    }

    if ($canSymlink) {
        # 2 caseに分ける。どちらも「公開されない」は同じだが、働くguardが違う。
        # skip理由まで確認しないと、片方のguardを壊しても、もう片方が拾って
        # 両caseがPASSし続ける。
        Test-Case -Name 'symlink recorded in the index is not published' -Arrange {
            $null = New-Item -ItemType SymbolicLink -Path $linkPath -Target $linkTarget
            $temporaryPaths.Add($linkPath)
            # `update-index --add`はsymlinkをmode 120000で記録する。
            # 追跡checkで弾かれると判定の由来が区別できないため、追跡済みにする。
            Use-DetachedIndexWith -RepositoryPath 'docs/__guardtest-link.md'
        } -AssertStaged {
            param($stagedRoot, $output)
            $leaked = Join-Path $stagedRoot 'docs/__guardtest-link.md'
            if (Test-Path -LiteralPath $leaked) {
                return 'Symlink docs file was published.'
            }
            if ($output -notmatch '__guardtest-link\.md \(symlink in Git\)') {
                return "Expected the Git mode guard to skip it. Output: $output"
            }
            return $null
        }

        # indexがregular file（mode 100644）として記録しているのに、working tree上は
        # symlinkという状態。`core.symlinks=false`のcheckoutと実体が食い違う場合に
        # 相当し、Gitのmode判定では拾えない。ここでだけreparse point checkへ到達する。
        Test-Case -Name 'reparse point recorded as a regular file is not published' -Arrange {
            $null = New-Item -ItemType SymbolicLink -Path $linkPath -Target $linkTarget
            $temporaryPaths.Add($linkPath)
            $blob = (& git -C $repositoryRoot rev-parse 'HEAD:README.md')
            if ($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace($blob)) {
                throw 'Unable to resolve a blob for the reparse point fixture.'
            }
            New-DetachedIndex
            & git -C $repositoryRoot update-index --add `
                --cacheinfo "100644,$blob,docs/__guardtest-link.md"
            if ($LASTEXITCODE -ne 0) {
                throw 'Unable to record the reparse point fixture as a regular file.'
            }
        } -AssertStaged {
            param($stagedRoot, $output)
            $leaked = Join-Path $stagedRoot 'docs/__guardtest-link.md'
            if (Test-Path -LiteralPath $leaked) {
                return 'Reparse point docs file was published.'
            }
            if ($output -notmatch '__guardtest-link\.md \(reparse point\)') {
                return "Expected the reparse point guard to skip it. Output: $output"
            }
            return $null
        }
    }

    Test-Case -Name 'staging succeeds again after cleanup' -Arrange {}
}
finally {
    Set-Content -LiteralPath $manifestPath -Value $manifestBackup -NoNewline
    Set-Content -LiteralPath $portalPagePath -Value $portalPageBackup -NoNewline
    foreach ($path in $temporaryPaths) {
        if (Test-Path -LiteralPath $path) {
            Remove-Item -LiteralPath $path -Force -Recurse
        }
    }
}

$results | ForEach-Object { Write-Output $_ }

if ($failed -gt 0) {
    throw "Pages guard tests failed: $failed case(s)."
}

Write-Output "PAGES_GUARD_TESTS=$($results.Count - $failed - $skipped) passed, $skipped skipped"
