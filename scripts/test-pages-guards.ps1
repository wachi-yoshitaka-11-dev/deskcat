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
$manifestPath = Join-Path $repositoryRoot 'pages/assets-manifest.psd1'
$assetsRoot = Join-Path $repositoryRoot 'pages/assets'
$docsRoot = Join-Path $repositoryRoot 'docs'

$manifestBackup = Get-Content -LiteralPath $manifestPath -Raw
$temporaryPaths = [System.Collections.Generic.List[string]]::new()
$results = [System.Collections.Generic.List[string]]::new()
$failed = 0

function Invoke-Prepare {
    $output = & pwsh -NoProfile -File $preparePath 2>&1
    return [pscustomobject]@{
        ExitCode = $LASTEXITCODE
        Output   = (($output | ForEach-Object { $_.ToString() }) -join "`n")
    }
}

function Test-Case {
    param(
        [Parameter(Mandatory)][string]$Name,
        [Parameter(Mandatory)][scriptblock]$Arrange,
        [string]$ExpectedMessage
    )

    try {
        & $Arrange
        $run = Invoke-Prepare

        if ([string]::IsNullOrEmpty($ExpectedMessage)) {
            if ($run.ExitCode -eq 0) {
                $script:results.Add("PASS  $Name")
            }
            else {
                $script:failed++
                $script:results.Add("FAIL  $Name -- expected success, got exit $($run.ExitCode)")
                $script:results.Add("      $($run.Output)")
            }
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
        Set-Content -LiteralPath $manifestPath -Value $manifestBackup -NoNewline
        foreach ($path in $temporaryPaths) {
            if (Test-Path -LiteralPath $path) {
                Remove-Item -LiteralPath $path -Force -Recurse
            }
        }
        $temporaryPaths.Clear()
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

    Test-Case -Name 'oversized staged file fails' -ExpectedMessage 'File exceeds the Pages size limit' -Arrange {
        $path = Join-Path $docsRoot '__guardtest-oversize.md'
        Set-Content -LiteralPath $path -Value ('# guard test' + ("`n" + ('x' * 1000)) * 1100) -NoNewline
        $temporaryPaths.Add($path)
    }

    Test-Case -Name 'staging succeeds again after cleanup' -Arrange {}
}
finally {
    Set-Content -LiteralPath $manifestPath -Value $manifestBackup -NoNewline
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

Write-Output "PAGES_GUARD_TESTS=$($results.Count - $failed) passed"
