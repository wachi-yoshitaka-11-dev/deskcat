[CmdletBinding()]
param()

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

# 同一page内のanchorを見逃さないことを、source Markdownと生成HTMLの両方で確認する。
# test frameworkやJekyllを追加せず、各validatorを一時directoryに対して子processで実行する。

. (Join-Path $PSScriptRoot 'lib/publish-guards.ps1')

$validateDocsPath = Join-Path $PSScriptRoot 'validate-doc-links.ps1'
$validateOutputPath = Join-Path $PSScriptRoot 'validate-pages-output.ps1'
$temporaryParent = [System.IO.Path]::GetFullPath([System.IO.Path]::GetTempPath())
$temporaryRoot = Join-Path $temporaryParent ('deskcat-link-validator-tests-' + [guid]::NewGuid().ToString('N'))
$defaultPagesConfigPath = Join-Path $temporaryRoot 'default-pages-config.yml'
$results = [System.Collections.Generic.List[string]]::new()
$failed = 0
$skipped = 0
$pwshExecutable = (Get-Command pwsh -ErrorAction Stop).Source

function Invoke-Validator {
    param(
        [Parameter(Mandatory)][string]$ScriptPath,
        [Parameter(Mandatory)][AllowEmptyCollection()][string[]]$Arguments,
        [ValidateRange(1, 300)][int]$TimeoutSeconds = 20
    )

    $effectiveArguments = @($Arguments)
    if ($ScriptPath -ceq $script:validateOutputPath -and
        -not ($effectiveArguments -icontains '-PagesConfigPath')) {
        # output validatorのunit fixtureをrepository固有の`pages/_config.yml`から分離する。
        # baseurl変更時にvalidator logicと無関係なcaseが壊れないよう、既知のconfigを明示する。
        $effectiveArguments += @('-PagesConfigPath', $script:defaultPagesConfigPath)
    }

    $startInfo = [System.Diagnostics.ProcessStartInfo]::new()
    $startInfo.FileName = $script:pwshExecutable
    $startInfo.UseShellExecute = $false
    $startInfo.RedirectStandardOutput = $true
    $startInfo.RedirectStandardError = $true
    $privatePaths = [System.Collections.Generic.List[string]]::new()
    for ($i = 0; $i -lt $effectiveArguments.Count; $i++) {
        # PowerShell parameter bindingはcase-insensitiveなので、検出側も同じにする。
        if ($effectiveArguments[$i] -in @('-RepositoryRoot', '-SiteRoot', '-PagesConfigPath') -and
            $i + 1 -lt $effectiveArguments.Count) {
            $privatePaths.Add([System.IO.Path]::GetFullPath($effectiveArguments[$i + 1]))
        }
    }
    foreach ($argument in @('-NoProfile', '-File', $ScriptPath) + $effectiveArguments) {
        $startInfo.ArgumentList.Add($argument)
    }

    $process = [System.Diagnostics.Process]::new()
    $process.StartInfo = $startInfo
    try {
        if (-not $process.Start()) {
            throw "Unable to start validator process: $ScriptPath"
        }
        # stdout／stderrを先にasync readし、pipe bufferの飽和でWaitForExitが
        # deadlockしないようにする。
        $stdoutTask = $process.StandardOutput.ReadToEndAsync()
        $stderrTask = $process.StandardError.ReadToEndAsync()
        if (-not $process.WaitForExit($TimeoutSeconds * 1000)) {
            try {
                if (-not $process.HasExited) {
                    $process.Kill($true)
                }
            }
            catch [System.InvalidOperationException] {
                # timeout判定の直後にprocessが自然終了したraceは、timeout結果のまま扱う。
            }
            $process.WaitForExit()
            $null = $stdoutTask.GetAwaiter().GetResult()
            $null = $stderrTask.GetAwaiter().GetResult()
            return [pscustomobject]@{
                ExitCode     = -1
                TimedOut     = $true
                Output       = "Validator timed out after $TimeoutSeconds second(s)."
                PrivatePaths = @($privatePaths)
            }
        }
        $process.WaitForExit()
        $stdout = $stdoutTask.GetAwaiter().GetResult()
        $stderr = $stderrTask.GetAwaiter().GetResult()
        $combinedOutput = @($stdout.TrimEnd(), $stderr.TrimEnd()) |
            Where-Object { -not [string]::IsNullOrWhiteSpace($_) }
        return [pscustomobject]@{
            ExitCode     = $process.ExitCode
            TimedOut     = $false
            Output       = ($combinedOutput -join "`n")
            PrivatePaths = @($privatePaths)
        }
    }
    finally {
        $process.Dispose()
    }
}

function Test-ValidatorOutputExposesPrivatePath {
    param(
        [Parameter(Mandatory)]$Run
    )

    $comparison = if ($IsWindows) {
        [System.StringComparison]::OrdinalIgnoreCase
    }
    else {
        [System.StringComparison]::Ordinal
    }
    $normalizedOutput = $Run.Output.Replace('\', '/')
    foreach ($privatePath in @($Run.PrivatePaths)) {
        # separator表記を揃え、Windowsではfilesystemと同じくcase違いも検出する。
        $normalizedPrivatePath = $privatePath.Replace('\', '/')
        if (-not [string]::IsNullOrWhiteSpace($normalizedPrivatePath) -and
            $normalizedOutput.IndexOf($normalizedPrivatePath, $comparison) -ge 0) {
            return $true
        }
    }
    return $false
}

function Test-Outcome {
    param(
        [Parameter(Mandatory)][string]$Name,
        [Parameter(Mandatory)]$Run,
        [Parameter(Mandatory)][bool]$ShouldSucceed,
        [string]$ExpectedMessage = '',
        [string]$ForbiddenMessage = ''
    )

    # 機密fixtureがchild outputへ現れた場合は、他の診断より先に検出し、
    # `$Run.Output`をこのtest processから再表示しない。
    if ($Run.TimedOut) {
        $script:failed++
        $script:results.Add("FAIL  $Name -- validator process timed out")
        return
    }
    # validatorの成否に関係なく、user指定のlocal pathをstdout／stderrへ出さない。
    # 個別caseだけにForbiddenMessageを書くと、新しい診断を追加した際に漏れる。
    if (Test-ValidatorOutputExposesPrivatePath -Run $Run) {
        $script:failed++
        $script:results.Add("FAIL  $Name -- local absolute path was exposed in validator output")
        return
    }
    if (-not [string]::IsNullOrWhiteSpace($ForbiddenMessage) -and
        $Run.Output -match [regex]::Escape($ForbiddenMessage)) {
        $script:failed++
        $script:results.Add("FAIL  $Name -- forbidden text was exposed in validator output")
        return
    }
    if ($ShouldSucceed -and $Run.ExitCode -ne 0) {
        $script:failed++
        $script:results.Add("FAIL  $Name -- expected success, got exit $($Run.ExitCode)")
        $script:results.Add("      $($Run.Output)")
        return
    }
    if (-not $ShouldSucceed -and $Run.ExitCode -eq 0) {
        $script:failed++
        $script:results.Add("FAIL  $Name -- expected failure, but validation succeeded")
        return
    }
    if (-not [string]::IsNullOrWhiteSpace($ExpectedMessage) -and
        $Run.Output -notmatch [regex]::Escape($ExpectedMessage)) {
        $script:failed++
        $script:results.Add("FAIL  $Name -- expected message '$ExpectedMessage'")
        $script:results.Add("      $($Run.Output)")
        return
    }

    $script:results.Add("PASS  $Name")
}

function Add-TestSkip {
    param(
        [Parameter(Mandatory)][string]$Name,
        [Parameter(Mandatory)][string]$Reason
    )

    $script:skipped++
    $script:results.Add("SKIP  $Name -- $Reason")
}

try {
    $sourceRoot = Join-Path $temporaryRoot 'source'
    $docsRoot = Join-Path $sourceRoot 'docs'
    $null = New-Item -ItemType Directory -Path $docsRoot -Force
    Set-Content -LiteralPath $defaultPagesConfigPath -Value 'baseurl: /deskcat' -NoNewline

    # harness timeoutが実際にprocess treeを停止し、単なる設定値になっていないことを確認する。
    $timeoutProbeScript = Join-Path $temporaryRoot 'timeout-probe.ps1'
    Set-Content -LiteralPath $timeoutProbeScript -Value 'Start-Sleep -Seconds 5' -NoNewline
    $timeoutProbe = Invoke-Validator -ScriptPath $timeoutProbeScript -Arguments @() -TimeoutSeconds 1
    if (-not $timeoutProbe.TimedOut -or $timeoutProbe.ExitCode -ne -1 -or
        $timeoutProbe.Output -notmatch 'timed out after 1 second') {
        throw 'Validator timeout harness precondition failed.'
    }
    $results.Add('PASS  validator harness bounds child-process runtime')
    Remove-Item -LiteralPath $timeoutProbeScript -Force

    # private-path検出器のpositive／negative control。検出器がno-opでも、各validator
    # caseのoutputがたまたまcleanなら全件成功するため、fixtureで検出力自体を確認する。
    $privacyProbePath = Join-Path $temporaryRoot 'private-path-probe'
    $renderedPrivacyProbePath = $privacyProbePath.Replace('\', '/')
    if ($IsWindows) {
        # WindowsでOrdinalへ退化した場合もpositive controlが失敗するようcaseを変える。
        $renderedPrivacyProbePath = $renderedPrivacyProbePath.ToUpperInvariant()
    }
    $leakingPrivacyProbe = [pscustomobject]@{
        Output       = "synthetic diagnostic: $renderedPrivacyProbePath"
        PrivatePaths = @($privacyProbePath)
    }
    $cleanPrivacyProbe = [pscustomobject]@{
        Output       = 'synthetic diagnostic: extra/page.html'
        PrivatePaths = @($privacyProbePath)
    }
    if (-not (Test-ValidatorOutputExposesPrivatePath -Run $leakingPrivacyProbe) -or
        (Test-ValidatorOutputExposesPrivatePath -Run $cleanPrivacyProbe)) {
        throw 'Validator private-path detection precondition failed.'
    }
    $results.Add('PASS  validator harness detects private paths without false positives')

    $missingRepositoryRoot = Join-Path $temporaryRoot 'missing-repository'
    $run = Invoke-Validator -ScriptPath $validateDocsPath `
        -Arguments @('-repositoryroot', $missingRepositoryRoot)
    if (@($run.PrivatePaths).Count -ne 1 -or
        $run.PrivatePaths[0] -cne [System.IO.Path]::GetFullPath($missingRepositoryRoot)) {
        throw 'Validator harness did not capture the private RepositoryRoot argument.'
    }
    Test-Outcome -Name 'source validator does not expose a missing repository-root path' `
        -Run $run -ShouldSucceed $false -ExpectedMessage 'Repository root does not exist.'

    $zeroSourceRoot = Join-Path $temporaryRoot 'zero-source'
    $null = New-Item -ItemType Directory -Path $zeroSourceRoot -Force
    Set-Content -LiteralPath (Join-Path $zeroSourceRoot 'tracked.txt') -Value 'fixture' -NoNewline
    & git -C $zeroSourceRoot init --quiet
    if ($LASTEXITCODE -ne 0) { throw 'Unable to initialize the zero-source repository.' }
    & git -C $zeroSourceRoot add -- tracked.txt
    if ($LASTEXITCODE -ne 0) { throw 'Unable to add the zero-source text fixture.' }
    $run = Invoke-Validator -ScriptPath $validateDocsPath `
        -Arguments @('-RepositoryRoot', $zeroSourceRoot)
    Test-Outcome -Name 'source validator reports zero tracked Markdown without exposing the root' `
        -Run $run -ShouldSucceed $false -ExpectedMessage 'Unable to enumerate tracked Markdown files.'

    $missingMarkdown = Join-Path $zeroSourceRoot 'missing.md'
    Set-Content -LiteralPath $missingMarkdown -Value '# Missing' -NoNewline
    & git -C $zeroSourceRoot add -- missing.md
    if ($LASTEXITCODE -ne 0) { throw 'Unable to add the missing Markdown fixture.' }
    Remove-Item -LiteralPath $missingMarkdown -Force
    $run = Invoke-Validator -ScriptPath $validateDocsPath `
        -Arguments @('-RepositoryRoot', $zeroSourceRoot)
    Test-Outcome -Name 'source validator reports zero resolved Markdown without exposing the root' `
        -Run $run -ShouldSucceed $false -ExpectedMessage 'No tracked Markdown files resolved.'

    $sourcePage = Join-Path $docsRoot 'page.md'
    Set-Content -LiteralPath $sourcePage -Value "# Existing`n`n[broken](#missing)" -NoNewline

    & git -C $sourceRoot init --quiet
    if ($LASTEXITCODE -ne 0) {
        throw 'Unable to initialize the temporary source repository.'
    }
    & git -C $sourceRoot config core.autocrlf false
    if ($LASTEXITCODE -ne 0) {
        throw 'Unable to configure the temporary source repository.'
    }
    & git -C $sourceRoot add -- docs/page.md
    if ($LASTEXITCODE -ne 0) {
        throw 'Unable to add the temporary Markdown fixture to the Git index.'
    }

    # fixtureがindexへ入ったことを、validatorと同じhelper呼び出しで確認する。
    # ここが空だと、validatorは「追跡fileが無い」として失敗し、
    # anchor検査の結果ではなくsetupの失敗をtestの失敗として報告してしまう。
    $indexed = Get-DeskCatTrackedFiles -RepositoryRoot $sourceRoot -PathSpec '.'
    if ($indexed -isnot [System.Collections.Generic.HashSet[string]] -or
        -not $indexed.Contains('docs/page.md')) {
        $allIndexed = @(& git -C $sourceRoot ls-files) -join ', '
        throw "Temporary fixture is not listed by the validator's tracked-file helper. ls-files(all)=[$allIndexed]"
    }

    $run = Invoke-Validator -ScriptPath $validateDocsPath -Arguments @('-RepositoryRoot', $sourceRoot)
    Test-Outcome -Name 'source validator rejects a broken same-page anchor' `
        -Run $run -ShouldSucceed $false -ExpectedMessage 'Broken anchor'

    Set-Content -LiteralPath $sourcePage -Value "# Existing`n`n[valid](#existing)" -NoNewline
    $run = Invoke-Validator -ScriptPath $validateDocsPath -Arguments @('-RepositoryRoot', $sourceRoot)
    Test-Outcome -Name 'source validator accepts a valid same-page anchor' `
        -Run $run -ShouldSucceed $true -ExpectedMessage 'BROKEN=0'

    # anchor集合を持たないMarkdownへのfragment linkを、無検査で通さないこと。
    # 追跡外やsymlinkのMarkdownはanchor集合に入らない。skipすると、このscriptで
    # ここだけがfail-openになる。
    $untrackedTarget = Join-Path $docsRoot 'untracked.md'
    Set-Content -LiteralPath $untrackedTarget -Value "# Other`n" -NoNewline
    Set-Content -LiteralPath $sourcePage -Value "# Existing`n`n[out](untracked.md#other)" -NoNewline
    $run = Invoke-Validator -ScriptPath $validateDocsPath -Arguments @('-RepositoryRoot', $sourceRoot)
    Test-Outcome -Name 'source validator reports an unverifiable anchor' `
        -Run $run -ShouldSucceed $false -ExpectedMessage 'Unverifiable anchor'
    Remove-Item -LiteralPath $untrackedTarget -Force

    Set-Content -LiteralPath $sourcePage -Value "# Existing`n`n[valid](#existing)" -NoNewline

    $siteRoot = Join-Path $temporaryRoot 'site'
    $requiredHtml = @(
        'index.html',
        '404.html',
        'docs/architecture/index.html',
        'docs/backlog/index.html',
        'docs/governance/index.html',
        'docs/governance/hardware-safety-policy.html',
        'docs/decisions/index.html',
        'docs/hardware/index.html',
        'docs/planning/index.html',
        'docs/protocol/index.html',
        'docs/runbooks/index.html',
        'docs/toolchains/index.html'
    )
    foreach ($relativePath in $requiredHtml) {
        $path = Join-Path $siteRoot $relativePath
        $null = New-Item -ItemType Directory -Path (Split-Path -Parent $path) -Force
        Set-Content -LiteralPath $path -Value '<html><body><h1 id="existing">Existing</h1></body></html>' -NoNewline
    }

    $requiredAssets = @(
        'favicon.ico',
        'assets/css/style.css',
        'assets/deskcat-concept.jpg'
    )
    foreach ($relativePath in $requiredAssets) {
        $path = Join-Path $siteRoot $relativePath
        $null = New-Item -ItemType Directory -Path (Split-Path -Parent $path) -Force
        Set-Content -LiteralPath $path -Value 'fixture' -NoNewline
    }

    $extraRoot = Join-Path $siteRoot 'extra'
    $null = New-Item -ItemType Directory -Path $extraRoot -Force
    $extraCount = $script:DeskCatMinimumPublishedCount - $requiredHtml.Count
    if ($extraCount -lt 1) {
        throw 'The HTML-count boundary fixture requires at least one non-required HTML file.'
    }
    for ($i = 0; $i -lt $extraCount; $i++) {
        $path = Join-Path $extraRoot ("page-$i.html")
        Set-Content -LiteralPath $path -Value '<html><body><h1 id="existing">Existing</h1></body></html>' -NoNewline
    }

    $outputIndex = Join-Path $siteRoot 'index.html'
    $nestedOutputIndex = Join-Path $siteRoot 'docs/architecture/index.html'

    $missingSiteRoot = Join-Path $temporaryRoot 'missing-site'
    $run = Invoke-Validator -ScriptPath $validateOutputPath -Arguments @('-SiteRoot', $missingSiteRoot)
    $expectedPrivatePaths = @(
        [System.IO.Path]::GetFullPath($missingSiteRoot),
        [System.IO.Path]::GetFullPath($defaultPagesConfigPath)
    )
    if ((@($run.PrivatePaths) -join "`u{1f}") -cne ($expectedPrivatePaths -join "`u{1f}")) {
        throw 'Validator harness did not capture the expected private output arguments.'
    }
    Test-Outcome -Name 'output validator does not expose a missing site-root path' `
        -Run $run -ShouldSucceed $false -ExpectedMessage 'Pages output directory does not exist.'

    $missingPagesConfig = Join-Path $temporaryRoot 'missing-pages-config.yml'
    $run = Invoke-Validator -ScriptPath $validateOutputPath `
        -Arguments @('-SiteRoot', $siteRoot, '-PagesConfigPath', $missingPagesConfig)
    $expectedPrivatePaths = @(
        [System.IO.Path]::GetFullPath($siteRoot),
        [System.IO.Path]::GetFullPath($missingPagesConfig)
    )
    if ((@($run.PrivatePaths) -join "`u{1f}") -cne ($expectedPrivatePaths -join "`u{1f}")) {
        throw 'Validator harness did not capture the private output-validator arguments.'
    }
    Test-Outcome -Name 'output validator does not expose a missing config path' `
        -Run $run -ShouldSucceed $false -ExpectedMessage 'Pages config file does not exist.'

    Set-Content -LiteralPath $nestedOutputIndex `
        -Value '<html><body><h1 id="existing">Existing</h1><a href="#missing">broken</a></body></html>' `
        -NoNewline
    $run = Invoke-Validator -ScriptPath $validateOutputPath -Arguments @('-SiteRoot', $siteRoot)
    Test-Outcome -Name 'output validator rejects a broken same-page anchor' `
        -Run $run -ShouldSucceed $false `
        -ExpectedMessage 'Broken anchor in generated site: docs/architecture/index.html -> #missing'
    Set-Content -LiteralPath $nestedOutputIndex `
        -Value '<html><body><h1 id="existing">Existing</h1></body></html>' -NoNewline

    # required outputとHTML件数下限を個別に回帰させる。別のfailure messageではなく、
    # それぞれのguardが実際に動いたことまで確認する。
    $requiredAsset = Join-Path $siteRoot 'favicon.ico'
    Remove-Item -LiteralPath $requiredAsset -Force
    $run = Invoke-Validator -ScriptPath $validateOutputPath -Arguments @('-SiteRoot', $siteRoot)
    Test-Outcome -Name 'output validator rejects a missing required output' `
        -Run $run -ShouldSucceed $false -ExpectedMessage 'Required output is missing: favicon.ico'
    Set-Content -LiteralPath $requiredAsset -Value 'fixture' -NoNewline

    $countBoundaryPage = Join-Path $extraRoot 'page-0.html'
    Remove-Item -LiteralPath $countBoundaryPage -Force
    $run = Invoke-Validator -ScriptPath $validateOutputPath -Arguments @('-SiteRoot', $siteRoot)
    Test-Outcome -Name 'output validator rejects an HTML set below the minimum' `
        -Run $run -ShouldSucceed $false -ExpectedMessage 'Unexpectedly small HTML set'
    Set-Content -LiteralPath $countBoundaryPage `
        -Value '<html><body><h1 id="existing">Existing</h1></body></html>' -NoNewline

    $pdfAsset = Join-Path $extraRoot 'manual.PDF'
    Set-Content -LiteralPath $pdfAsset -Value 'synthetic PDF fixture' -NoNewline
    $run = Invoke-Validator -ScriptPath $validateOutputPath -Arguments @('-SiteRoot', $siteRoot)
    Test-Outcome -Name 'output validator reports a PDF with a relative path' `
        -Run $run -ShouldSucceed $false -ExpectedMessage 'PDF output is not allowed: extra/manual.PDF'
    Remove-Item -LiteralPath $pdfAsset -Force

    # repository外targetが実在しても、_siteのlexical boundary外なら拒否する。
    $outsideTarget = Join-Path $temporaryRoot 'outside-target.html'
    Set-Content -LiteralPath $outsideTarget `
        -Value '<html><body><h1 id="outside">Outside</h1></body></html>' -NoNewline
    Set-Content -LiteralPath $outputIndex `
        -Value '<html><body><h1 id="existing">Existing</h1><a href="../outside-target.html">outside</a></body></html>' `
        -NoNewline
    $run = Invoke-Validator -ScriptPath $validateOutputPath -Arguments @('-SiteRoot', $siteRoot)
    Test-Outcome -Name 'output validator rejects an existing target outside the site root' `
        -Run $run -ShouldSucceed $false -ExpectedMessage 'Link escapes Pages output in index.html'
    Remove-Item -LiteralPath $outsideTarget -Force

    # 秘密情報と個人pathのguardを回帰させる。この2つは公開直前の最終確認であり、
    # patternが退化しても他のcaseはすべて通るため、専用のcaseが無いと無検査になる。
    # fixtureの値は明確に合成のもの（実在しないtoken形式の埋め草）を使う。
    $guardPage = Join-Path $siteRoot 'extra/guard.html'
    $syntheticSecret = 'ghp_' + ('a' * 24)
    Set-Content -LiteralPath $guardPage `
        -Value "<html><body><h1 id=`"$syntheticSecret`">Sensitive</h1><a href=`"missing.html?token=$syntheticSecret`">fixture</a></body></html>" -NoNewline
    Set-Content -LiteralPath $outputIndex `
        -Value '<html><body><h1 id="existing">Existing</h1><a href="/deskcat/extra/guard.html#missing">sensitive target</a></body></html>' `
        -NoNewline
    $run = Invoke-Validator -ScriptPath $validateOutputPath -Arguments @('-SiteRoot', $siteRoot)
    Test-Outcome -Name 'output validator rejects secret-like content' `
        -Run $run -ShouldSucceed $false -ExpectedMessage 'Secret-like content detected: extra/guard.html' `
        -ForbiddenMessage $syntheticSecret

    $syntheticPersonalPath = '/home/exampleuser/notes.md'
    Set-Content -LiteralPath $guardPage `
        -Value "<html><body><a href=`"$syntheticPersonalPath`">fixture</a></body></html>" `
        -NoNewline
    $run = Invoke-Validator -ScriptPath $validateOutputPath -Arguments @('-SiteRoot', $siteRoot)
    Test-Outcome -Name 'output validator rejects a personal absolute path' `
        -Run $run -ShouldSucceed $false -ExpectedMessage 'Personal absolute path detected: extra/guard.html' `
        -ForbiddenMessage $syntheticPersonalPath

    # HTML件数を変えたまま最終caseへ進まない。guard fixtureはここで取り除く。
    Remove-Item -LiteralPath $guardPage -Force

    # 最終artifactではHTML以外のtext出力もscanする。Jekyll変換後のCSS／SVG等を
    # HTMLだけの検査から漏らさない。
    $guardAsset = Join-Path $extraRoot 'guard.css'
    $cssSecret = 'ghp_' + ('b' * 24)
    Set-Content -LiteralPath $guardAsset -Value "/* $cssSecret */" -NoNewline
    $run = Invoke-Validator -ScriptPath $validateOutputPath -Arguments @('-SiteRoot', $siteRoot)
    Test-Outcome -Name 'output validator rejects secret-like content in generated CSS' `
        -Run $run -ShouldSucceed $false -ExpectedMessage 'Secret-like content detected: extra/guard.css' `
        -ForbiddenMessage $cssSecret

    $cssPersonalPath = '/home/exampleuser/generated.css'
    Set-Content -LiteralPath $guardAsset -Value "/* $cssPersonalPath */" -NoNewline
    $run = Invoke-Validator -ScriptPath $validateOutputPath -Arguments @('-SiteRoot', $siteRoot)
    Test-Outcome -Name 'output validator rejects a personal absolute path in generated CSS' `
        -Run $run -ShouldSucceed $false -ExpectedMessage 'Personal absolute path detected: extra/guard.css' `
        -ForbiddenMessage $cssPersonalPath
    Remove-Item -LiteralPath $guardAsset -Force

    $guardLicense = Join-Path $extraRoot 'LICENSE'
    $licenseSecret = 'ghp_' + ('c' * 24)
    Set-Content -LiteralPath $guardLicense -Value $licenseSecret -NoNewline
    $run = Invoke-Validator -ScriptPath $validateOutputPath -Arguments @('-SiteRoot', $siteRoot)
    Test-Outcome -Name 'output validator scans an extensionless LICENSE output' `
        -Run $run -ShouldSucceed $false -ExpectedMessage 'Secret-like content detected: extra/LICENSE' `
        -ForbiddenMessage $licenseSecret
    Remove-Item -LiteralPath $guardLicense -Force

    # SVGなど非HTML assetのfragmentはHTML id検査の対象外だが、asset自体の存在は
    # href/src検査で必須とする。存在するassetは通し、存在しないassetは通さない。
    $fragmentAsset = Join-Path $extraRoot 'sprite.svg'
    Set-Content -LiteralPath $fragmentAsset `
        -Value '<svg xmlns="http://www.w3.org/2000/svg"><symbol id="cat" /></svg>' `
        -NoNewline
    Set-Content -LiteralPath $outputIndex `
        -Value '<html><body><h1 id="existing">Existing</h1><a href="/deskcat/extra/sprite.svg#cat">valid asset fragment</a></body></html>' `
        -NoNewline
    $run = Invoke-Validator -ScriptPath $validateOutputPath -Arguments @('-SiteRoot', $siteRoot)
    Test-Outcome -Name 'output validator accepts a fragment on an existing non-HTML asset' `
        -Run $run -ShouldSucceed $true -ExpectedMessage 'BROKEN_LINKS=0'

    Set-Content -LiteralPath $outputIndex `
        -Value '<html><body><h1 id="existing">Existing</h1><a href="/deskcat/extra/missing.svg#cat">missing asset</a></body></html>' `
        -NoNewline
    $run = Invoke-Validator -ScriptPath $validateOutputPath -Arguments @('-SiteRoot', $siteRoot)
    Test-Outcome -Name 'output validator rejects a fragment on a missing non-HTML asset' `
        -Run $run -ShouldSucceed $false -ExpectedMessage 'Broken local link'

    # raw HTMLではunquoted属性も有効である。quoted属性だけを抽出すると、このlinkを
    # 無検査で通す。一方、data-hrefはnavigation属性ではないため対象にしない。
    Set-Content -LiteralPath $outputIndex `
        -Value '<html><body><h1 id="existing">Existing</h1><a href=/deskcat/extra/missing.svg>missing asset</a></body></html>' `
        -NoNewline
    $run = Invoke-Validator -ScriptPath $validateOutputPath -Arguments @('-SiteRoot', $siteRoot)
    Test-Outcome -Name 'output validator rejects a missing target in an unquoted link attribute' `
        -Run $run -ShouldSucceed $false -ExpectedMessage 'Broken local link'

    Set-Content -LiteralPath $outputIndex `
        -Value '<html><body><h1 id="existing">Existing</h1><div data-href="/deskcat/extra/missing.svg">metadata</div></body></html>' `
        -NoNewline
    $run = Invoke-Validator -ScriptPath $validateOutputPath -Arguments @('-SiteRoot', $siteRoot)
    Test-Outcome -Name 'output validator ignores data-href metadata' `
        -Run $run -ShouldSucceed $true -ExpectedMessage 'BROKEN_LINKS=0'

    # quoted value内の`href=`や`id=`は実属性ではない。開始tag全体へのregexでは
    # `/missing.svg`をlinkとして誤検出するため、attribute tokenizerの境界を固定する。
    Set-Content -LiteralPath $outputIndex `
        -Value '<html><body><h1 id="existing">Existing</h1><div data-note="text href=/deskcat/extra/missing.svg" id=fake>metadata</div><a href="#existing">valid</a></body></html>' `
        -NoNewline
    $run = Invoke-Validator -ScriptPath $validateOutputPath -Arguments @('-SiteRoot', $siteRoot)
    Test-Outcome -Name 'output validator ignores attribute-like text inside a quoted value' `
        -Run $run -ShouldSucceed $true -ExpectedMessage 'BROKEN_LINKS=0'

    Set-Content -LiteralPath $outputIndex `
        -Value '<html><body><h1 id="existing">Existing</h1><div data-note="id=phantom">metadata</div><a href="#phantom">invalid</a></body></html>' `
        -NoNewline
    $run = Invoke-Validator -ScriptPath $validateOutputPath -Arguments @('-SiteRoot', $siteRoot)
    Test-Outcome -Name 'output validator does not create an id from a quoted value' `
        -Run $run -ShouldSucceed $false -ExpectedMessage 'Broken anchor in generated site'

    # HTML tokenizerは同名attributeの先頭だけを採用する。2個目のidを集合へ入れると、
    # browserには存在しないanchorをvalidatorだけが有効と誤認する。
    Set-Content -LiteralPath $outputIndex `
        -Value '<html><body><h1 id="existing" ID="phantom">Existing</h1><a href="#phantom">invalid</a></body></html>' `
        -NoNewline
    $run = Invoke-Validator -ScriptPath $validateOutputPath -Arguments @('-SiteRoot', $siteRoot)
    Test-Outcome -Name 'output validator drops a duplicate id attribute' `
        -Run $run -ShouldSucceed $false -ExpectedMessage 'Broken anchor in generated site'

    Set-Content -LiteralPath $outputIndex `
        -Value '<html><body><h1 id="existing">Existing</h1><a href="#existing" HREF="/deskcat/extra/missing.svg">valid first value</a></body></html>' `
        -NoNewline
    $run = Invoke-Validator -ScriptPath $validateOutputPath -Arguments @('-SiteRoot', $siteRoot)
    Test-Outcome -Name 'output validator drops a duplicate link attribute' `
        -Run $run -ShouldSucceed $true -ExpectedMessage 'BROKEN_LINKS=0'

    # SVG 1.1のnamespaced linkも実navigation属性として検査する。存在するtargetだけを
    # 受理し、同じattribute名のmissing targetを無検査で通さない。
    Set-Content -LiteralPath $outputIndex `
        -Value '<html><body><h1 id="existing">Existing</h1><svg><use xlink:href="/deskcat/extra/sprite.svg#cat"></use></svg></body></html>' `
        -NoNewline
    $run = Invoke-Validator -ScriptPath $validateOutputPath -Arguments @('-SiteRoot', $siteRoot)
    Test-Outcome -Name 'output validator accepts a valid namespaced SVG link' `
        -Run $run -ShouldSucceed $true -ExpectedMessage 'BROKEN_LINKS=0'

    Set-Content -LiteralPath $outputIndex `
        -Value '<html><body><h1 id="existing">Existing</h1><svg><use xlink:href="/deskcat/extra/missing.svg#cat"></use></svg></body></html>' `
        -NoNewline
    $run = Invoke-Validator -ScriptPath $validateOutputPath -Arguments @('-SiteRoot', $siteRoot)
    Test-Outcome -Name 'output validator rejects a missing namespaced SVG link' `
        -Run $run -ShouldSucceed $false -ExpectedMessage 'Broken local link'

    Set-Content -LiteralPath $outputIndex `
        -Value '<html><body><h1 id="existing">Existing</h1><a href="">same page</a></body></html>' `
        -NoNewline
    $run = Invoke-Validator -ScriptPath $validateOutputPath -Arguments @('-SiteRoot', $siteRoot)
    Test-Outcome -Name 'output validator accepts an empty same-page href' `
        -Run $run -ShouldSucceed $true -ExpectedMessage 'BROKEN_LINKS=0'

    Set-Content -LiteralPath $outputIndex `
        -Value '<html><body><h1 id="existing">Existing</h1><img src="#existing"></body></html>' `
        -NoNewline
    $run = Invoke-Validator -ScriptPath $validateOutputPath -Arguments @('-SiteRoot', $siteRoot)
    Test-Outcome -Name 'output validator rejects a src without a resource path' `
        -Run $run -ShouldSucceed $false -ExpectedMessage 'Source attribute has no resource path'

    # text nodeに表示された属性例とHTML comment内のlinkはrendered navigationではない。
    # raw content全体へ属性regexを掛けるとmissing linkとして誤検出する。
    Set-Content -LiteralPath $outputIndex `
        -Value '<html><body><h1 id="existing">Existing</h1><code>href=&quot;/deskcat/extra/missing.svg&quot;</code><!-- <a href="/deskcat/extra/missing.svg">comment</a> --></body></html>' `
        -NoNewline
    $run = Invoke-Validator -ScriptPath $validateOutputPath -Arguments @('-SiteRoot', $siteRoot)
    Test-Outcome -Name 'output validator ignores displayed and commented link examples' `
        -Run $run -ShouldSucceed $true -ExpectedMessage 'BROKEN_LINKS=0'

    # quoted属性内の`>`で開始tagを途中終了せず、その後ろのhrefも検査する。
    Set-Content -LiteralPath $outputIndex `
        -Value '<html><body><h1 id="existing">Existing</h1><a title="1 > 0" href="/deskcat/extra/missing.svg">missing</a></body></html>' `
        -NoNewline
    $run = Invoke-Validator -ScriptPath $validateOutputPath -Arguments @('-SiteRoot', $siteRoot)
    Test-Outcome -Name 'output validator scans attributes after a quoted greater-than sign' `
        -Run $run -ShouldSucceed $false -ExpectedMessage 'Broken local link'

    # script/styleのraw-textとtextarea/titleのRCDATAでは、本文中の`<a>`文字列は
    # 実際のHTML tagではない。
    Set-Content -LiteralPath $outputIndex `
        -Value '<html><head><TITLE><a href="/deskcat/extra/missing.svg">title text</a></TITLE></head><body><h1 id="existing">Existing</h1><SCRIPT>const sample = ''<a href="/deskcat/extra/missing.svg">'';</SCRIPT><TEXTAREA><a href="/deskcat/extra/missing.svg">textarea text</a></TEXTAREA></body></html>' `
        -NoNewline
    $run = Invoke-Validator -ScriptPath $validateOutputPath -Arguments @('-SiteRoot', $siteRoot)
    Test-Outcome -Name 'output validator ignores link-like text in raw-text and RCDATA bodies' `
        -Run $run -ShouldSucceed $true -ExpectedMessage 'BROKEN_LINKS=0'

    # raw-text内のcomment-like textはHTML commentではない。commentを先に除去すると、
    # 後続の実linkまで`<!--`と`-->`の間として消えるため、実linkの検出を回帰させる。
    Set-Content -LiteralPath $outputIndex `
        -Value '<html><body><h1 id="existing">Existing</h1><script>const marker = "<!--";</script><a href="/deskcat/extra/missing.svg">missing</a><!-- actual comment --></body></html>' `
        -NoNewline
    $run = Invoke-Validator -ScriptPath $validateOutputPath -Arguments @('-SiteRoot', $siteRoot)
    Test-Outcome -Name 'output validator preserves real links after comment-like raw text' `
        -Run $run -ShouldSucceed $false -ExpectedMessage 'Broken local link'

    # comment内の偽raw-text tagとquoted属性内のcomment-like textをscanner stateから
    # 分離する。global regex同士では後続の実linkを巻き込んで消し得る組合せである。
    Set-Content -LiteralPath $outputIndex `
        -Value '<html><body><h1 id="existing">Existing</h1><!-- <script>fake</script> --><a title="<!--" href="/deskcat/extra/missing.svg">missing</a><!-- actual comment --></body></html>' `
        -NoNewline
    $run = Invoke-Validator -ScriptPath $validateOutputPath -Arguments @('-SiteRoot', $siteRoot)
    Test-Outcome -Name 'output validator separates comments raw text and quoted attributes' `
        -Run $run -ShouldSucceed $false -ExpectedMessage 'Broken local link'

    Set-Content -LiteralPath $outputIndex `
        -Value '<html><body><h1 id="existing">Existing</h1><p>1 < 2</p><a href="/deskcat/extra/missing.svg">missing</a></body></html>' `
        -NoNewline
    $run = Invoke-Validator -ScriptPath $validateOutputPath -Arguments @('-SiteRoot', $siteRoot)
    Test-Outcome -Name 'output validator keeps scanning after a text less-than sign' `
        -Run $run -ShouldSucceed $false -ExpectedMessage 'Broken local link'

    # HTMLのraw-text要素をscript/styleだけに限定しない。deprecated要素を含め、
    # browserがtextとして扱うfallback内のtag-like textをnavigationに数えない。
    Set-Content -LiteralPath $outputIndex `
        -Value '<html><body><h1 id="existing">Existing</h1><iframe><a href="/deskcat/extra/missing.svg">iframe</a></iframe><xmp><a href="/deskcat/extra/missing.svg">xmp</a></xmp><noembed><a href="/deskcat/extra/missing.svg">noembed</a></noembed><noframes><a href="/deskcat/extra/missing.svg">noframes</a></noframes><a href="#existing">valid</a></body></html>' `
        -NoNewline
    $run = Invoke-Validator -ScriptPath $validateOutputPath -Arguments @('-SiteRoot', $siteRoot)
    Test-Outcome -Name 'output validator ignores every supported raw-text element body' `
        -Run $run -ShouldSucceed $true -ExpectedMessage 'BROKEN_LINKS=0'

    Set-Content -LiteralPath $outputIndex `
        -Value '<html><body><h1 id="existing">Existing</h1><plaintext><a href="/deskcat/extra/missing.svg">text only</a></plaintext></body></html>' `
        -NoNewline
    $run = Invoke-Validator -ScriptPath $validateOutputPath -Arguments @('-SiteRoot', $siteRoot)
    Test-Outcome -Name 'output validator treats content after plaintext as text' `
        -Run $run -ShouldSucceed $true -ExpectedMessage 'BROKEN_LINKS=0'

    # namespaceを追跡しないscannerで`<style/>`を未閉鎖HTML raw-textと決めつけると、
    # inline SVG後の実linkをEOFまで隠す。self-closing markerは安全側に走査を継続する。
    Set-Content -LiteralPath $outputIndex `
        -Value '<html><body><h1 id="existing">Existing</h1><svg><style /></svg><a href="/deskcat/extra/missing.svg">missing</a></body></html>' `
        -NoNewline
    $run = Invoke-Validator -ScriptPath $validateOutputPath -Arguments @('-SiteRoot', $siteRoot)
    Test-Outcome -Name 'output validator keeps scanning after a self-closing raw-text tag' `
        -Run $run -ShouldSucceed $false -ExpectedMessage 'Broken local link'

    # `<!-->`と`--!>`もHTML parser上はcommentを終了する。通常の`-->`だけを探すと、
    # 後続の実linkを未閉鎖commentとして隠せる。
    Set-Content -LiteralPath $outputIndex `
        -Value '<html><body><h1 id="existing">Existing</h1><!--><a href="/deskcat/extra/missing.svg">missing</a></body></html>' `
        -NoNewline
    $run = Invoke-Validator -ScriptPath $validateOutputPath -Arguments @('-SiteRoot', $siteRoot)
    Test-Outcome -Name 'output validator resumes after an abruptly closed empty comment' `
        -Run $run -ShouldSucceed $false -ExpectedMessage 'Broken local link'

    Set-Content -LiteralPath $outputIndex `
        -Value '<html><body><h1 id="existing">Existing</h1><!-- ignored --!><a href="/deskcat/extra/missing.svg">missing</a></body></html>' `
        -NoNewline
    $run = Invoke-Validator -ScriptPath $validateOutputPath -Arguments @('-SiteRoot', $siteRoot)
    Test-Outcome -Name 'output validator resumes after a comment end bang' `
        -Run $run -ShouldSucceed $false -ExpectedMessage 'Broken local link'

    # processing instructionと未知のmarkup declarationはbogus commentとして最初の
    # `>`で終わる。quoteを尊重すると、その後ろの実linkの終端まで誤ってskipする。
    Set-Content -LiteralPath $outputIndex `
        -Value '<html><body><h1 id="existing">Existing</h1><?fixture "><a href="/deskcat/extra/missing.svg">missing</a></body></html>' `
        -NoNewline
    $run = Invoke-Validator -ScriptPath $validateOutputPath -Arguments @('-SiteRoot', $siteRoot)
    Test-Outcome -Name 'output validator resumes after a processing instruction' `
        -Run $run -ShouldSucceed $false -ExpectedMessage 'Broken local link'

    Set-Content -LiteralPath $outputIndex `
        -Value '<html><body><h1 id="existing">Existing</h1><!fixture "><a href="/deskcat/extra/missing.svg">missing</a></body></html>' `
        -NoNewline
    $run = Invoke-Validator -ScriptPath $validateOutputPath -Arguments @('-SiteRoot', $siteRoot)
    Test-Outcome -Name 'output validator resumes after an unknown markup declaration' `
        -Run $run -ShouldSucceed $false -ExpectedMessage 'Broken local link'

    Set-Content -LiteralPath $outputIndex `
        -Value '<html><body><h1 id="existing">Existing</h1></ "><a href="/deskcat/extra/missing.svg">missing</a></body></html>' `
        -NoNewline
    $run = Invoke-Validator -ScriptPath $validateOutputPath -Arguments @('-SiteRoot', $siteRoot)
    Test-Outcome -Name 'output validator resumes after an invalid end-tag opener' `
        -Run $run -ShouldSucceed $false -ExpectedMessage 'Broken local link'

    # 閉じていない長大な開始tagでも、再走査やbacktrackingを起こさず線形に完了すること。
    # scannerが末尾まで到達したら残りをmarkupとして再解釈しない。
    $malformedLongTag = '<html><body><h1 id="existing">Existing</h1><a ' + ('x' * 100000)
    Set-Content -LiteralPath $outputIndex -Value $malformedLongTag -NoNewline
    $run = Invoke-Validator -ScriptPath $validateOutputPath -Arguments @('-SiteRoot', $siteRoot)
    Test-Outcome -Name 'output validator bounds malformed long-tag parsing' `
        -Run $run -ShouldSucceed $true -ExpectedMessage 'BROKEN_LINKS=0'

    Set-Content -LiteralPath $outputIndex `
        -Value '<html><body><h1 ID="existing">Existing</h1><a href="JaVa&#x0a;ScRiPt:alert(1)">unsafe</a></body></html>' `
        -NoNewline
    $run = Invoke-Validator -ScriptPath $validateOutputPath -Arguments @('-SiteRoot', $siteRoot)
    Test-Outcome -Name 'output validator rejects a control-and-entity-obfuscated javascript URL' `
        -Run $run -ShouldSucceed $false -ExpectedMessage 'Unsafe URL scheme' `
        -ForbiddenMessage "JaVa`nScRiPt"

    Set-Content -LiteralPath $outputIndex `
        -Value '<html><body><h1 id="existing">Existing</h1><img src="data:image/svg+xml,fixture"></body></html>' `
        -NoNewline
    $run = Invoke-Validator -ScriptPath $validateOutputPath -Arguments @('-SiteRoot', $siteRoot)
    Test-Outcome -Name 'output validator rejects a data URI asset' `
        -Run $run -ShouldSucceed $false -ExpectedMessage 'Unsafe URL scheme'

    # 明示allowlist外のURI schemeをlocal pathへ落とさない。file:等と同名のfileが
    # 生成物に存在しても、browserではlocal resource以外として解釈される。
    Set-Content -LiteralPath $outputIndex `
        -Value '<html><body><h1 id="existing">Existing</h1><a href="file:fixture.html">file</a></body></html>' `
        -NoNewline
    $run = Invoke-Validator -ScriptPath $validateOutputPath -Arguments @('-SiteRoot', $siteRoot)
    Test-Outcome -Name 'output validator rejects a file URI' `
        -Run $run -ShouldSucceed $false -ExpectedMessage 'Unsafe URL scheme'

    Set-Content -LiteralPath $outputIndex `
        -Value '<html><body><h1 id="existing">Existing</h1><a href="vbscript:msgbox(1)">vbscript</a></body></html>' `
        -NoNewline
    $run = Invoke-Validator -ScriptPath $validateOutputPath -Arguments @('-SiteRoot', $siteRoot)
    Test-Outcome -Name 'output validator rejects a vbscript URI' `
        -Run $run -ShouldSucceed $false -ExpectedMessage 'Unsafe URL scheme'

    Set-Content -LiteralPath $outputIndex `
        -Value '<html><body><h1 id="existing">Existing</h1><a href="deskcat-custom:resource">custom</a></body></html>' `
        -NoNewline
    $run = Invoke-Validator -ScriptPath $validateOutputPath -Arguments @('-SiteRoot', $siteRoot)
    Test-Outcome -Name 'output validator rejects an unapproved custom URI scheme' `
        -Run $run -ShouldSucceed $false -ExpectedMessage 'Unsafe URL scheme'

    # 外部URLのallowlist側も固定し、unsafe scheme追加時に正常系を巻き込まない。
    Set-Content -LiteralPath $outputIndex `
        -Value '<html><body><h1 id="existing">Existing</h1><a href="//example.com/path">relative</a><a href="http://example.com/">http</a><a href="https://example.com/">https</a><a href="mailto:example@example.com">mail</a><a href="tel:+10000000000">tel</a><a href="h&#x0a;ttps://example.com/">normalized</a></body></html>' `
        -NoNewline
    $run = Invoke-Validator -ScriptPath $validateOutputPath -Arguments @('-SiteRoot', $siteRoot)
    Test-Outcome -Name 'output validator accepts every allowlisted external scheme' `
        -Run $run -ShouldSucceed $true -ExpectedMessage 'BROKEN_LINKS=0'

    # .NETのTrim()はNBSPも除去するが、URL parserはNBSPをASCII空白として捨てない。
    # 先頭NBSPを消して既存local pathへ読み替えないことを回帰させる。
    $nonUrlWhitespace = [char]0x00a0
    Set-Content -LiteralPath $outputIndex `
        -Value "<html><body><h1 id=`"existing`">Existing</h1><a href=`"${nonUrlWhitespace}/deskcat/docs/architecture/`">invalid</a></body></html>" `
        -NoNewline
    $run = Invoke-Validator -ScriptPath $validateOutputPath -Arguments @('-SiteRoot', $siteRoot)
    Test-Outcome -Name 'output validator preserves non-URL Unicode whitespace' `
        -Run $run -ShouldSucceed $false -ExpectedMessage 'Broken local link'

    Set-Content -LiteralPath $outputIndex `
        -Value '<html><body><h1 id=unquoted-anchor>Existing</h1><a href=#unquoted-anchor>valid</a></body></html>' `
        -NoNewline
    $run = Invoke-Validator -ScriptPath $validateOutputPath -Arguments @('-SiteRoot', $siteRoot)
    Test-Outcome -Name 'output validator accepts matching unquoted id and link attributes' `
        -Run $run -ShouldSucceed $true -ExpectedMessage 'BROKEN_LINKS=0'

    # 拡張子のpercent-encodingでunconverted Markdown禁止を迂回させない。
    # targetを実在させ、単なるmissing-file errorとの取り違えも防ぐ。
    $encodedMarkdownAsset = Join-Path $extraRoot 'source.md'
    Set-Content -LiteralPath $encodedMarkdownAsset -Value '# Source' -NoNewline
    Set-Content -LiteralPath $outputIndex `
        -Value '<html><body><h1 id="existing">Existing</h1><a href="/deskcat/extra/source%2Emd">encoded markdown</a></body></html>' `
        -NoNewline
    $run = Invoke-Validator -ScriptPath $validateOutputPath -Arguments @('-SiteRoot', $siteRoot)
    Test-Outcome -Name 'output validator rejects a percent-encoded Markdown extension' `
        -Run $run -ShouldSucceed $false -ExpectedMessage 'Unconverted Markdown link in index.html'
    Remove-Item -LiteralPath $encodedMarkdownAsset -Force

    $markdownAsset = Join-Path $extraRoot 'source.markdown'
    Set-Content -LiteralPath $markdownAsset -Value '# Source' -NoNewline
    Set-Content -LiteralPath $outputIndex `
        -Value '<html><body><h1 id="existing">Existing</h1><a href="/deskcat/extra/source.markdown">markdown source</a></body></html>' `
        -NoNewline
    $run = Invoke-Validator -ScriptPath $validateOutputPath -Arguments @('-SiteRoot', $siteRoot)
    Test-Outcome -Name 'output validator rejects the shared markdown extension' `
        -Run $run -ShouldSucceed $false -ExpectedMessage 'Unconverted Markdown link in index.html'
    Remove-Item -LiteralPath $markdownAsset -Force

    # encoded backslashもdecode後にURL separatorへ正規化し、OS間で同じ結果にする。
    Set-Content -LiteralPath $outputIndex `
        -Value '<html><body><h1 id="existing">Existing</h1><a href="/deskcat/docs%5Carchitecture/">encoded separator</a></body></html>' `
        -NoNewline
    $run = Invoke-Validator -ScriptPath $validateOutputPath -Arguments @('-SiteRoot', $siteRoot)
    Test-Outcome -Name 'output validator resolves an encoded path separator consistently' `
        -Run $run -ShouldSucceed $true -ExpectedMessage 'BROKEN_LINKS=0'

    # extensionless URLに`.html`とdirectory indexが同時に対応する場合、配列順で
    # 片方を選ばない。実serverの解決順に依存するため曖昧として停止する。
    $ambiguousHtml = Join-Path $extraRoot 'ambiguous.html'
    $ambiguousIndex = Join-Path $extraRoot 'ambiguous/index.html'
    Set-Content -LiteralPath $ambiguousHtml `
        -Value '<html><body><h1 id="file">File</h1></body></html>' -NoNewline
    $null = New-Item -ItemType Directory -Path (Split-Path -Parent $ambiguousIndex) -Force
    Set-Content -LiteralPath $ambiguousIndex `
        -Value '<html><body><h1 id="directory">Directory</h1></body></html>' -NoNewline
    Set-Content -LiteralPath $outputIndex `
        -Value '<html><body><h1 id="existing">Existing</h1><a href="/deskcat/extra/ambiguous">ambiguous</a></body></html>' `
        -NoNewline
    $run = Invoke-Validator -ScriptPath $validateOutputPath -Arguments @('-SiteRoot', $siteRoot)
    Test-Outcome -Name 'output validator rejects an ambiguous extensionless link' `
        -Run $run -ShouldSucceed $false -ExpectedMessage 'Ambiguous local link'
    Remove-Item -LiteralPath $ambiguousHtml -Force
    Remove-Item -LiteralPath (Split-Path -Parent $ambiguousIndex) -Recurse -Force

    # directoryの存在だけではlinkを解決済みにしない。末尾`/`のURLはindex.htmlが
    # 存在するときだけ受理し、空directoryならbroken linkとして報告する。
    $emptyDirectory = Join-Path $extraRoot 'empty-directory'
    $null = New-Item -ItemType Directory -Path $emptyDirectory -Force
    Set-Content -LiteralPath $outputIndex `
        -Value '<html><body><h1 id="existing">Existing</h1><a href="/deskcat/extra/empty-directory/">missing index</a></body></html>' `
        -NoNewline
    $run = Invoke-Validator -ScriptPath $validateOutputPath -Arguments @('-SiteRoot', $siteRoot)
    Test-Outcome -Name 'output validator rejects a directory link without index.html' `
        -Run $run -ShouldSucceed $false -ExpectedMessage 'Broken local link'

    Set-Content -LiteralPath $outputIndex `
        -Value '<html><body><h1 id="existing">Existing</h1><a href="/deskcat/docs/architecture/">directory index</a></body></html>' `
        -NoNewline
    $run = Invoke-Validator -ScriptPath $validateOutputPath -Arguments @('-SiteRoot', $siteRoot)
    Test-Outcome -Name 'output validator accepts a directory link with index.html' `
        -Run $run -ShouldSucceed $true -ExpectedMessage 'BROKEN_LINKS=0'

    # Windowsのcase-insensitive filesystemでも、Linux Pagesで404になるcase違いを
    # 通さない。実fileは`docs/architecture/index.html`であり、`Architecture`は誤り。
    Set-Content -LiteralPath $outputIndex `
        -Value '<html><body><h1 id="existing">Existing</h1><a href="/deskcat/docs/Architecture/">wrong case</a></body></html>' `
        -NoNewline
    $run = Invoke-Validator -ScriptPath $validateOutputPath -Arguments @('-SiteRoot', $siteRoot)
    Test-Outcome -Name 'output validator rejects a case-mismatched generated path' `
        -Run $run -ShouldSucceed $false -ExpectedMessage 'Broken local link'

    # project Pagesのrootは`/deskcat`である。`/docs/...`をsite rootへ読み替えると、
    # fileが存在するfixtureでは実際の公開URLが404でもvalidatorが通ってしまう。
    Set-Content -LiteralPath $outputIndex `
        -Value '<html><body><h1 id="existing">Existing</h1><a href="/docs/architecture/">outside base path</a></body></html>' `
        -NoNewline
    $run = Invoke-Validator -ScriptPath $validateOutputPath -Arguments @('-SiteRoot', $siteRoot)
    Test-Outcome -Name 'output validator rejects a root-absolute link outside the Pages base path' `
        -Run $run -ShouldSucceed $false -ExpectedMessage 'Root-absolute link is outside Pages base path'

    # base pathはvalidator内の定数ではなくPages configの正本から読む。
    $alternatePagesConfig = Join-Path $temporaryRoot 'alternate-pages-config.yml'
    Set-Content -LiteralPath $alternatePagesConfig -Value 'baseurl: /alternate' -NoNewline
    Set-Content -LiteralPath $outputIndex `
        -Value '<html><body><h1 id="existing">Existing</h1><a href="/alternate/docs/architecture/">alternate base</a></body></html>' `
        -NoNewline
    $run = Invoke-Validator -ScriptPath $validateOutputPath `
        -Arguments @('-SiteRoot', $siteRoot, '-PagesConfigPath', $alternatePagesConfig)
    Test-Outcome -Name 'output validator derives the Pages base path from config' `
        -Run $run -ShouldSucceed $true -ExpectedMessage 'BROKEN_LINKS=0'

    Set-Content -LiteralPath $alternatePagesConfig `
        -Value "baseurl: /alternate`nBaseUrl: /ignored" -NoNewline
    $run = Invoke-Validator -ScriptPath $validateOutputPath `
        -Arguments @('-SiteRoot', $siteRoot, '-PagesConfigPath', $alternatePagesConfig)
    Test-Outcome -Name 'output validator ignores differently cased YAML keys' `
        -Run $run -ShouldSucceed $true -ExpectedMessage 'BROKEN_LINKS=0'

    Set-Content -LiteralPath $alternatePagesConfig -Value 'baseurl: /deskcat/' -NoNewline
    Set-Content -LiteralPath $outputIndex `
        -Value '<html><body><h1 id="existing">Existing</h1><a href="/deskcat/docs/architecture/">trailing slash base</a></body></html>' `
        -NoNewline
    $run = Invoke-Validator -ScriptPath $validateOutputPath `
        -Arguments @('-SiteRoot', $siteRoot, '-PagesConfigPath', $alternatePagesConfig)
    Test-Outcome -Name 'output validator normalizes a trailing slash in baseurl' `
        -Run $run -ShouldSucceed $true -ExpectedMessage 'BROKEN_LINKS=0'

    Set-Content -LiteralPath $alternatePagesConfig `
        -Value 'baseurl: /deskcat # project Pages' -NoNewline
    $run = Invoke-Validator -ScriptPath $validateOutputPath `
        -Arguments @('-SiteRoot', $siteRoot, '-PagesConfigPath', $alternatePagesConfig)
    Test-Outcome -Name 'output validator accepts an unquoted baseurl with a YAML comment' `
        -Run $run -ShouldSucceed $true -ExpectedMessage 'BROKEN_LINKS=0'

    $nonYamlWhitespace = [char]0x00a0
    Set-Content -LiteralPath $alternatePagesConfig `
        -Value "baseurl: /deskcat${nonYamlWhitespace}#not-a-comment" -NoNewline
    $run = Invoke-Validator -ScriptPath $validateOutputPath `
        -Arguments @('-SiteRoot', $siteRoot, '-PagesConfigPath', $alternatePagesConfig)
    Test-Outcome -Name 'output validator does not treat non-YAML whitespace as a comment boundary' `
        -Run $run -ShouldSucceed $false -ExpectedMessage 'contains an unsafe baseurl'

    Set-Content -LiteralPath $alternatePagesConfig `
        -Value "baseurl:${nonYamlWhitespace}/deskcat" -NoNewline
    $run = Invoke-Validator -ScriptPath $validateOutputPath `
        -Arguments @('-SiteRoot', $siteRoot, '-PagesConfigPath', $alternatePagesConfig)
    Test-Outcome -Name 'output validator does not consume non-YAML whitespace after the key' `
        -Run $run -ShouldSucceed $false -ExpectedMessage 'must define exactly one baseurl'

    Set-Content -LiteralPath $alternatePagesConfig -Value 'baseurl:/deskcat' -NoNewline
    $run = Invoke-Validator -ScriptPath $validateOutputPath `
        -Arguments @('-SiteRoot', $siteRoot, '-PagesConfigPath', $alternatePagesConfig)
    Test-Outcome -Name 'output validator rejects a baseurl without YAML value separation' `
        -Run $run -ShouldSucceed $false -ExpectedMessage 'must define exactly one baseurl'

    Set-Content -LiteralPath $alternatePagesConfig `
        -Value "baseurl: /deskcat${nonYamlWhitespace}" -NoNewline
    $run = Invoke-Validator -ScriptPath $validateOutputPath `
        -Arguments @('-SiteRoot', $siteRoot, '-PagesConfigPath', $alternatePagesConfig)
    Test-Outcome -Name 'output validator rejects Unicode whitespace in baseurl' `
        -Run $run -ShouldSucceed $false -ExpectedMessage 'contains an unsafe baseurl'

    Set-Content -LiteralPath $alternatePagesConfig `
        -Value 'baseurl: "/deskcat" # project Pages' -NoNewline
    $run = Invoke-Validator -ScriptPath $validateOutputPath `
        -Arguments @('-SiteRoot', $siteRoot, '-PagesConfigPath', $alternatePagesConfig)
    Test-Outcome -Name 'output validator accepts a double-quoted baseurl with a YAML comment' `
        -Run $run -ShouldSucceed $true -ExpectedMessage 'BROKEN_LINKS=0'

    Set-Content -LiteralPath $alternatePagesConfig `
        -Value "baseurl: '/deskcat' # project Pages" -NoNewline
    $run = Invoke-Validator -ScriptPath $validateOutputPath `
        -Arguments @('-SiteRoot', $siteRoot, '-PagesConfigPath', $alternatePagesConfig)
    Test-Outcome -Name 'output validator accepts a single-quoted baseurl with a YAML comment' `
        -Run $run -ShouldSucceed $true -ExpectedMessage 'BROKEN_LINKS=0'

    Set-Content -LiteralPath $alternatePagesConfig `
        -Value 'baseurl: "/deskcat#fragment" # project Pages' -NoNewline
    $run = Invoke-Validator -ScriptPath $validateOutputPath `
        -Arguments @('-SiteRoot', $siteRoot, '-PagesConfigPath', $alternatePagesConfig)
    Test-Outcome -Name 'output validator preserves a hash inside a quoted baseurl' `
        -Run $run -ShouldSucceed $false -ExpectedMessage 'contains an unsafe baseurl'

    Set-Content -LiteralPath $alternatePagesConfig -Value 'baseurl: ""' -NoNewline
    Set-Content -LiteralPath $outputIndex `
        -Value '<html><body><h1 id="existing">Existing</h1><a href="/docs/architecture/">root Pages</a></body></html>' `
        -NoNewline
    $run = Invoke-Validator -ScriptPath $validateOutputPath `
        -Arguments @('-SiteRoot', $siteRoot, '-PagesConfigPath', $alternatePagesConfig)
    Test-Outcome -Name 'output validator accepts an empty baseurl for root Pages' `
        -Run $run -ShouldSucceed $true -ExpectedMessage 'BROKEN_LINKS=0'

    Set-Content -LiteralPath $alternatePagesConfig -Value 'baseurl: # root Pages' -NoNewline
    $run = Invoke-Validator -ScriptPath $validateOutputPath `
        -Arguments @('-SiteRoot', $siteRoot, '-PagesConfigPath', $alternatePagesConfig)
    Test-Outcome -Name 'output validator accepts an empty baseurl before a YAML comment' `
        -Run $run -ShouldSucceed $true -ExpectedMessage 'BROKEN_LINKS=0'

    Set-Content -LiteralPath $alternatePagesConfig -Value 'baseurl: relative/path' -NoNewline
    $run = Invoke-Validator -ScriptPath $validateOutputPath `
        -Arguments @('-SiteRoot', $siteRoot, '-PagesConfigPath', $alternatePagesConfig)
    Test-Outcome -Name 'output validator rejects an unsafe relative baseurl' `
        -Run $run -ShouldSucceed $false -ExpectedMessage 'contains an unsafe baseurl'

    $unsafeConfigSecret = 'ghp_' + ('d' * 24)
    Set-Content -LiteralPath $alternatePagesConfig -Value "baseurl: $unsafeConfigSecret" -NoNewline
    $run = Invoke-Validator -ScriptPath $validateOutputPath `
        -Arguments @('-SiteRoot', $siteRoot, '-PagesConfigPath', $alternatePagesConfig)
    Test-Outcome -Name 'output validator does not expose an unsafe baseurl value' `
        -Run $run -ShouldSucceed $false -ExpectedMessage 'contains an unsafe baseurl' `
        -ForbiddenMessage $unsafeConfigSecret

    Set-Content -LiteralPath $alternatePagesConfig `
        -Value "baseurl: /first`nbaseurl: /second" -NoNewline
    $run = Invoke-Validator -ScriptPath $validateOutputPath `
        -Arguments @('-SiteRoot', $siteRoot, '-PagesConfigPath', $alternatePagesConfig)
    Test-Outcome -Name 'output validator rejects duplicate Pages base paths' `
        -Run $run -ShouldSucceed $false -ExpectedMessage 'must define exactly one baseurl'

    Set-Content -LiteralPath $alternatePagesConfig `
        -Value "site:`n  baseurl: /nested" -NoNewline
    $run = Invoke-Validator -ScriptPath $validateOutputPath `
        -Arguments @('-SiteRoot', $siteRoot, '-PagesConfigPath', $alternatePagesConfig)
    Test-Outcome -Name 'output validator rejects a config with no top-level baseurl' `
        -Run $run -ShouldSucceed $false -ExpectedMessage 'must define exactly one baseurl'
    Remove-Item -LiteralPath $alternatePagesConfig -Force

    # URIのfragmentでは`?`もdataである。query delimiterとして先にsplitすると、
    # fragmentが空へ退化して壊れたanchorを無検査で通す。
    Set-Content -LiteralPath $outputIndex `
        -Value '<html><body><h1 id="existing">Existing</h1><a href="#missing?part">broken fragment</a></body></html>' `
        -NoNewline
    $run = Invoke-Validator -ScriptPath $validateOutputPath -Arguments @('-SiteRoot', $siteRoot)
    Test-Outcome -Name 'output validator rejects a broken fragment containing a question mark' `
        -Run $run -ShouldSucceed $false -ExpectedMessage 'Broken anchor in generated site'

    Set-Content -LiteralPath $outputIndex `
        -Value '<html><body><h1 id="existing?part">Existing</h1><a href="#existing?part">valid fragment</a></body></html>' `
        -NoNewline
    $run = Invoke-Validator -ScriptPath $validateOutputPath -Arguments @('-SiteRoot', $siteRoot)
    Test-Outcome -Name 'output validator accepts a valid fragment containing a question mark' `
        -Run $run -ShouldSucceed $true -ExpectedMessage 'BROKEN_LINKS=0'

    # reparse-point directory配下の実体を、_site内の通常fileとして受理しない。
    # Windowsでは権限不要のjunction、他環境ではsymbolic linkを使う。
    $reparseTarget = Join-Path $temporaryRoot 'reparse-target'
    $reparseDirectory = Join-Path $extraRoot 'reparse-directory'
    $null = New-Item -ItemType Directory -Path $reparseTarget -Force
    Set-Content -LiteralPath (Join-Path $reparseTarget 'outside.html') `
        -Value '<html><body><h1 id="outside">Outside</h1></body></html>' -NoNewline
    $reparseCreated = $false
    try {
        if ($IsWindows) {
            $null = New-Item -ItemType Junction -Path $reparseDirectory -Target $reparseTarget
        }
        else {
            $null = New-Item -ItemType SymbolicLink -Path $reparseDirectory -Target $reparseTarget
        }
        $reparseCreated = $true
    }
    catch {
        $partialReparse = Get-Item -LiteralPath $reparseDirectory -Force -ErrorAction SilentlyContinue
        if ($null -ne $partialReparse) {
            Remove-Item -LiteralPath $reparseDirectory -Force
        }
        Add-TestSkip -Name 'output validator rejects a reparse-point directory' `
            -Reason 'link creation is not permitted'
    }
    if ($reparseCreated) {
        try {
            Set-Content -LiteralPath $outputIndex `
                -Value '<html><body><h1 id="existing">Existing</h1><a href="/deskcat/extra/reparse-directory/outside.html">reparse target</a></body></html>' `
                -NoNewline
            $run = Invoke-Validator -ScriptPath $validateOutputPath -Arguments @('-SiteRoot', $siteRoot)
            Test-Outcome -Name 'output validator rejects a reparse-point directory' `
                -Run $run -ShouldSucceed $false `
                -ExpectedMessage 'Symbolic or reparse-point output is not allowed: extra/reparse-directory'
        }
        finally {
            $createdReparse = Get-Item -LiteralPath $reparseDirectory -Force -ErrorAction SilentlyContinue
            if ($null -ne $createdReparse) {
                Remove-Item -LiteralPath $reparseDirectory -Force
            }
        }
    }

    Set-Content -LiteralPath $outputIndex `
        -Value '<html><body><h1 id="existing">Existing</h1><a href="#existing">valid</a></body></html>' `
        -NoNewline
    $siteRootAlias = Join-Path $temporaryRoot 'site-root-alias'
    $siteRootAliasCreated = $false
    try {
        if ($IsWindows) {
            $null = New-Item -ItemType Junction -Path $siteRootAlias -Target $siteRoot
        }
        else {
            $null = New-Item -ItemType SymbolicLink -Path $siteRootAlias -Target $siteRoot
        }
        $siteRootAliasCreated = $true
    }
    catch {
        $partialAlias = Get-Item -LiteralPath $siteRootAlias -Force -ErrorAction SilentlyContinue
        if ($null -ne $partialAlias) {
            Remove-Item -LiteralPath $siteRootAlias -Force
        }
        Add-TestSkip -Name 'output validator rejects a reparse-point site root' `
            -Reason 'link creation is not permitted'
    }
    if ($siteRootAliasCreated) {
        try {
            $run = Invoke-Validator -ScriptPath $validateOutputPath -Arguments @('-SiteRoot', $siteRootAlias)
            Test-Outcome -Name 'output validator rejects a reparse-point site root' `
                -Run $run -ShouldSucceed $false `
                -ExpectedMessage 'Symbolic or reparse-point output is not allowed at root: .'
        }
        finally {
            $createdAlias = Get-Item -LiteralPath $siteRootAlias -Force -ErrorAction SilentlyContinue
            if ($null -ne $createdAlias) {
                Remove-Item -LiteralPath $siteRootAlias -Force
            }
        }
    }

    Set-Content -LiteralPath $outputIndex `
        -Value '<html><body><h1 id="existing">Existing</h1><a href="#existing">valid</a></body></html>' `
        -NoNewline
    $run = Invoke-Validator -ScriptPath $validateOutputPath -Arguments @('-SiteRoot', $siteRoot)
    Test-Outcome -Name 'output validator accepts a valid same-page anchor' `
        -Run $run -ShouldSucceed $true -ExpectedMessage 'BROKEN_LINKS=0'
}
finally {
    # GUID付きの専用directoryだけを削除し、一時directory root自体は対象にしない。
    if ((Test-DeskCatPathWithinRoot -Path $temporaryRoot -Root $temporaryParent) -and
        (Split-Path -Leaf $temporaryRoot).StartsWith(
            'deskcat-link-validator-tests-',
            [System.StringComparison]::Ordinal)) {
        if (Test-Path -LiteralPath $temporaryRoot) {
            Remove-Item -LiteralPath $temporaryRoot -Recurse -Force
        }
    }
}

# Markdown link抽出がruntimeや改行形式に依存せず、validatorの対象だけを返すこと。
$parserFixture = "[local](docs/page.md)`r`n" +
    "![image](assets/image.png)`n" +
    "[same](#section)`n" +
    "[multi`nline](docs/other.md `"title`")`n" +
    "[external](https://example.com)`n" +
    "[invalid](docs/invalid.md trailing)"
$parserTargets = @(Get-DeskCatMarkdownLinkTargets -Content $parserFixture)
$expectedTargets = @('docs/page.md', '#section', 'docs/other.md', 'https://example.com')
if (($parserTargets -join "`u{1f}") -cne ($expectedTargets -join "`u{1f}")) {
    $failed++
    $results.Add("FAIL  Markdown link parser returns deterministic targets -- got [$($parserTargets -join ', ')]")
}
else {
    $results.Add('PASS  Markdown link parser returns deterministic targets')
}

# fence判定が見出し走査とlink走査で共通であること。
# 別々に持つと、片方だけを変えたときにanchor集合とlink集合が別の行を見る。
$fenceFixture = "# Heading`n" +
    '```bash' + "`n" +
    "# not a heading`n" +
    "[not a link](fenced.md)`n" +
    '```' + "`n" +
    "[real](real.md)`n" +
    '~~~' + "`n" +
    "# also fenced`n" +
    '~~~' + "`n" +
    '## Tail'
$outside = @(Get-DeskCatMarkdownOutsideFences -Content $fenceFixture) -join "`n"
$expectedOutside = "# Heading`n[real](real.md)`n## Tail"
if ($outside -cne $expectedOutside) {
    $failed++
    $results.Add("FAIL  fence helper drops fenced lines -- got [$($outside -replace "`n", '\n')]")
}
elseif (@(Get-DeskCatMarkdownLinkTargets -Content $outside) -join ',' -cne 'real.md') {
    $failed++
    $results.Add('FAIL  fence helper output feeds the link scanner')
}
else {
    $results.Add('PASS  fence helper is shared by the heading and link scans')
}

# `Get-DeskCatTrackedFiles`が一致件数によらずHashSetを返すこと。
# 展開されると1件のときStringになり、`.Contains()`が部分一致へ変わる。
# `prepare-pages.ps1`は公開判定に`.Contains()`を使うため、追跡判定が壊れる。
$repositoryRootForHelper = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))

$relativeHelperResult = Get-DeskCatPathRelativeToRoot `
    -Path (Join-Path $repositoryRootForHelper 'scripts/test-link-validators.ps1') `
    -Root $repositoryRootForHelper
$outsideHelperPath = Join-Path (Split-Path -Parent $repositoryRootForHelper) 'deskcat-outside-probe.txt'
$outsideHelperRejected = $false
try {
    $null = Get-DeskCatPathRelativeToRoot -Path $outsideHelperPath -Root $repositoryRootForHelper
}
catch {
    $outsideHelperRejected = $_.Exception.Message -eq 'Cannot format a path outside the publication root.'
}
if ($relativeHelperResult -cne 'scripts/test-link-validators.ps1' -or -not $outsideHelperRejected) {
    $failed++
    $results.Add('FAIL  publication path helper normalizes inside paths and rejects outside paths')
}
else {
    $results.Add('PASS  publication path helper normalizes inside paths and rejects outside paths')
}

# Gitのmodeでsymlinkを判定できること。file属性ではcheckout環境で結果が変わり、
# 走査対象と公開物が環境ごとに変わる。`CLAUDE.md`はindex上mode 120000である。
$symlinkSet = Get-DeskCatTrackedSymlinks -RepositoryRoot $repositoryRootForHelper
if ($symlinkSet -isnot [System.Collections.Generic.HashSet[string]]) {
    $failed++
    $actualType = if ($null -eq $symlinkSet) { 'null' } else { $symlinkSet.GetType().Name }
    $results.Add("FAIL  tracked symlink helper returns a set -- got $actualType")
}
elseif (-not $symlinkSet.Contains('CLAUDE.md')) {
    $failed++
    $results.Add('FAIL  tracked symlink helper detects the CLAUDE.md symlink -- not found')
}
elseif ($symlinkSet.Contains('AGENTS.md')) {
    $failed++
    $results.Add('FAIL  tracked symlink helper excludes regular files -- AGENTS.md reported as a symlink')
}
else {
    $results.Add('PASS  tracked symlink helper uses the Git index mode')
}


foreach ($case in @(
    @{
        Name = 'tracked file helper keeps a set for one match'
        PathSpec = 'LICENSE'
        MinCount = 1
        ExpectedExactPath = 'LICENSE'
    }
    @{
        Name = 'tracked file helper keeps a set for many matches'
        PathSpec = 'scripts'
        MinCount = 2
        ExpectedPathRoot = 'scripts'
    }
    @{
        Name = 'tracked file helper returns no match'
        PathSpec = 'no-such-path-xyz'
        ExpectedCount = 0
    }
)) {
    $set = Get-DeskCatTrackedFiles -RepositoryRoot $repositoryRootForHelper -PathSpec $case.PathSpec
    $unexpectedPaths = @()
    if ($set -is [System.Collections.Generic.HashSet[string]]) {
        if ($case.ContainsKey('ExpectedExactPath')) {
            $unexpectedPaths = @($set | Where-Object { $_ -cne $case.ExpectedExactPath })
        }
        elseif ($case.ContainsKey('ExpectedPathRoot')) {
            $expectedRoot = $case.ExpectedPathRoot.TrimEnd('/')
            $expectedPrefix = "$expectedRoot/"
            $unexpectedPaths = @($set | Where-Object {
                    $normalized = $_ -replace '\\', '/'
                    $normalized -cne $expectedRoot -and
                    -not $normalized.StartsWith($expectedPrefix, [System.StringComparison]::Ordinal)
                })
        }
    }

    if ($set -isnot [System.Collections.Generic.HashSet[string]]) {
        $failed++
        $actualType = if ($null -eq $set) { 'null' } else { $set.GetType().Name }
        $results.Add("FAIL  $($case.Name) -- got $actualType, expected HashSet")
    }
    elseif ($case.ContainsKey('ExpectedCount') -and $set.Count -ne $case.ExpectedCount) {
        $failed++
        $results.Add("FAIL  $($case.Name) -- Count $($set.Count), expected $($case.ExpectedCount)")
    }
    elseif ($case.ContainsKey('MinCount') -and $set.Count -lt $case.MinCount) {
        $failed++
        $results.Add("FAIL  $($case.Name) -- Count $($set.Count) is below $($case.MinCount)")
    }
    elseif ($unexpectedPaths.Count -gt 0) {
        $failed++
        $results.Add("FAIL  $($case.Name) -- unexpected path(s): [$($unexpectedPaths -join ', ')]")
    }
    else {
        $results.Add("PASS  $($case.Name)")
    }
}

# 部分一致へ退化していないこと。`LICENSE`は`LIC`を含むが、集合の要素ではない。
$licenseSet = Get-DeskCatTrackedFiles -RepositoryRoot $repositoryRootForHelper -PathSpec 'LICENSE'
if ($licenseSet -isnot [System.Collections.Generic.HashSet[string]]) {
    $failed++
    $actualType = if ($null -eq $licenseSet) { 'null' } else { $licenseSet.GetType().Name }
    $results.Add("FAIL  tracked file helper matches exactly -- got $actualType, expected HashSet")
}
elseif ($licenseSet.Contains('LIC')) {
    $failed++
    $results.Add('FAIL  tracked file helper matches exactly -- substring matched')
}
elseif (-not $licenseSet.Contains('LICENSE')) {
    $failed++
    $results.Add('FAIL  tracked file helper matches exactly -- exact path missing')
}
else {
    $results.Add('PASS  tracked file helper matches exactly')
}

# 非ASCIIのpathが、escape sequenceではなく実pathとして返ること。
# `git ls-files`は既定（`core.quotePath=true`）で非ASCIIをdouble quoteと
# octal escapeにする。escapeされたまま集合へ入ると、`.Contains()`が実pathと
# 一致せず、追跡済みfileをsymlink除外や公開判定で取りこぼす。
# 両helperが同じquoting設定で読むことも併せて確認する。
$quotingRoot = Join-Path $temporaryParent ("deskcat-quote-" + [System.Guid]::NewGuid().ToString('N'))
$null = New-Item -ItemType Directory -Path $quotingRoot -Force
try {
    $nonAsciiName = '日本語ファイル.md'
    Set-Content -LiteralPath (Join-Path $quotingRoot $nonAsciiName) -Value '# 見出し' -NoNewline -Encoding utf8

    & git -C $quotingRoot init --quiet
    if ($LASTEXITCODE -ne 0) { throw 'Unable to initialize the quoting fixture repository.' }
    # global設定に依存させない。helperの`-c core.quotePath=false`を外すと、
    # local設定が有効になって非ASCII pathがescapeされ、必ずtestが失敗する。
    & git -C $quotingRoot config --local core.quotePath true
    if ($LASTEXITCODE -ne 0) { throw 'Unable to configure the quoting fixture repository.' }
    & git -C $quotingRoot add --all
    if ($LASTEXITCODE -ne 0) { throw 'Unable to add the quoting fixture to the Git index.' }

    # helperが無効化すべきGit quotingが、fixtureで実際に発生していることを先に確認する。
    # このpositive controlがなければ、helperから`core.quotePath=false`が消えてもtestが
    # 何も検証せず成功しうる。
    $rawListing = @(& git -C $quotingRoot ls-files)
    if ($LASTEXITCODE -ne 0 -or $rawListing.Count -ne 1 -or
        $rawListing[0] -notmatch '^".*\\[0-7]{3}.*"$') {
        throw 'The quoting fixture precondition failed: git ls-files did not quote the non-ASCII path.'
    }

    # OSのsymlink作成権限に依存させず、Git indexへmode 120000を直接登録する。
    # 通常file列挙とは別の`ls-files -s`経路でも非ASCII pathを実pathで返すことを確認する。
    $nonAsciiSymlinkName = '日本語リンク.md'
    $blobOidOutput = @(& git -C $quotingRoot rev-parse --verify ":$nonAsciiName")
    if ($LASTEXITCODE -ne 0 -or $blobOidOutput.Count -ne 1 -or
        [string]::IsNullOrWhiteSpace($blobOidOutput[0])) {
        throw 'Unable to resolve the quoting fixture blob.'
    }
    $blobOid = $blobOidOutput[0].Trim()
    & git -C $quotingRoot update-index --add --cacheinfo "120000,$blobOid,$nonAsciiSymlinkName"
    if ($LASTEXITCODE -ne 0) { throw 'Unable to add the non-ASCII symlink fixture to the Git index.' }

    $quotedSet = Get-DeskCatTrackedFiles -RepositoryRoot $quotingRoot -PathSpec '.'
    if ($quotedSet -isnot [System.Collections.Generic.HashSet[string]]) {
        $failed++
        $actualType = if ($null -eq $quotedSet) { 'null' } else { $quotedSet.GetType().Name }
        $results.Add("FAIL  tracked file helper keeps a set in the quoting fixture -- got $actualType")
    }
    elseif ($quotedSet.Count -ne 2 -or
        -not $quotedSet.Contains($nonAsciiName) -or
        -not $quotedSet.Contains($nonAsciiSymlinkName)) {
        $failed++
        $results.Add("FAIL  tracked file helper returns the exact unquoted non-ASCII set -- got [$($quotedSet -join ', ')]")
    }
    else {
        $results.Add('PASS  tracked file helper returns the exact unquoted non-ASCII set')
    }

    $quotedSymlinkSet = Get-DeskCatTrackedSymlinks -RepositoryRoot $quotingRoot
    if ($quotedSymlinkSet -isnot [System.Collections.Generic.HashSet[string]]) {
        $failed++
        $actualType = if ($null -eq $quotedSymlinkSet) { 'null' } else { $quotedSymlinkSet.GetType().Name }
        $results.Add("FAIL  tracked symlink helper keeps a set in the quoting fixture -- got $actualType")
    }
    elseif ($quotedSymlinkSet.Count -ne 1 -or
        -not $quotedSymlinkSet.Contains($nonAsciiSymlinkName) -or
        $quotedSymlinkSet.Contains($nonAsciiName)) {
        $failed++
        $results.Add("FAIL  tracked symlink helper returns only the unquoted non-ASCII symlink -- got [$($quotedSymlinkSet -join ', ')]")
    }
    else {
        $results.Add('PASS  tracked symlink helper returns only the unquoted non-ASCII symlink')
    }
}
finally {
    if ((Test-DeskCatPathWithinRoot -Path $quotingRoot -Root $temporaryParent) -and
        (Split-Path -Leaf $quotingRoot).StartsWith('deskcat-quote-', [System.StringComparison]::Ordinal)) {
        if (Test-Path -LiteralPath $quotingRoot) {
            Remove-Item -LiteralPath $quotingRoot -Recurse -Force
        }
    }
}

$passedCount = 0
$failedResultCount = 0
$skippedResultCount = 0
foreach ($result in $results) {
    if ($result.StartsWith('PASS  ', [System.StringComparison]::Ordinal)) {
        $passedCount++
    }
    elseif ($result.StartsWith('FAIL  ', [System.StringComparison]::Ordinal)) {
        $failedResultCount++
    }
    elseif ($result.StartsWith('SKIP  ', [System.StringComparison]::Ordinal)) {
        $skippedResultCount++
    }
}

$results | ForEach-Object { Write-Output $_ }

if ($failedResultCount -ne $failed -or $skippedResultCount -ne $skipped) {
    throw "Link validator result accounting failed: failures=$failed/$failedResultCount skips=$skipped/$skippedResultCount."
}
if ($failed -gt 0) {
    throw "Link validator tests failed: $failed case(s)."
}
Write-Output "LINK_VALIDATOR_TESTS=$passedCount passed, $skipped skipped"
