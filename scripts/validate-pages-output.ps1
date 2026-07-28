[CmdletBinding()]
param(
    [string]$SiteRoot = ''
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$repositoryRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
if ([string]::IsNullOrWhiteSpace($SiteRoot)) {
    $SiteRoot = Join-Path $repositoryRoot '_site'
}
$siteRootPath = [System.IO.Path]::GetFullPath($SiteRoot)

if (-not (Test-Path -LiteralPath $siteRootPath -PathType Container)) {
    throw "Pages output directory does not exist: $siteRootPath"
}

$requiredFiles = @(
    'index.html',
    '404.html',
    'favicon.ico',
    'docs/architecture/index.html',
    'docs/governance/index.html',
    'docs/governance/hardware-safety-policy.html',
    'docs/decisions/index.html',
    'docs/hardware/index.html',
    'docs/protocol/index.html',
    'docs/runbooks/index.html',
    'docs/toolchains/index.html'
)

$problems = [System.Collections.Generic.List[string]]::new()
foreach ($relativePath in $requiredFiles) {
    $candidate = Join-Path $siteRootPath $relativePath
    if (-not (Test-Path -LiteralPath $candidate -PathType Leaf)) {
        $problems.Add("Required output is missing: $relativePath")
    }
}

$files = @(Get-ChildItem -LiteralPath $siteRootPath -Recurse -Force -File)
foreach ($file in $files) {
    if (($file.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) {
        $problems.Add("Symbolic or reparse-point output is not allowed: $($file.FullName)")
    }
    if ($file.Extension.Equals('.pdf', [System.StringComparison]::OrdinalIgnoreCase)) {
        $problems.Add("PDF output is not allowed: $($file.FullName)")
    }
}

$htmlFiles = @($files | Where-Object { $_.Extension.Equals('.html', [System.StringComparison]::OrdinalIgnoreCase) })
if ($htmlFiles.Count -lt 35) {
    $problems.Add("Unexpectedly small HTML set: $($htmlFiles.Count)")
}

$secretPattern = 'ghp_[A-Za-z0-9]{20,}|github_pat_[A-Za-z0-9_]{20,}|AKIA[0-9A-Z]{16}|-----BEGIN (?:RSA |EC |OPENSSH )?PRIVATE KEY-----'
$personalPathPattern = 'C:\\Users\\[^\\\s]+|/home/[^/\s]+|file://'
$attributePattern = '(?i)(?:href|src)\s*=\s*["''](?<value>[^"'']+)["'']'
$basePath = '/deskcat'

foreach ($html in $htmlFiles) {
    $content = Get-Content -LiteralPath $html.FullName -Raw
    if ([regex]::IsMatch($content, $secretPattern)) {
        $problems.Add("Secret-like content detected: $($html.FullName)")
    }
    if ([regex]::IsMatch($content, $personalPathPattern, [System.Text.RegularExpressions.RegexOptions]::IgnoreCase)) {
        $problems.Add("Personal absolute path detected: $($html.FullName)")
    }

    foreach ($match in [regex]::Matches($content, $attributePattern)) {
        $value = [System.Net.WebUtility]::HtmlDecode($match.Groups['value'].Value.Trim())
        if (
            [string]::IsNullOrWhiteSpace($value) -or
            $value.StartsWith('#') -or
            $value.StartsWith('//') -or
            $value -match '^(?i:https?|mailto|tel|data|javascript):'
        ) {
            continue
        }

        $path = ($value -split '[?#]', 2)[0]
        if ([string]::IsNullOrWhiteSpace($path)) {
            continue
        }
        if ($path -match '(?i)\.md$') {
            $problems.Add("Unconverted Markdown link in $($html.FullName): $value")
            continue
        }

        $path = [System.Uri]::UnescapeDataString($path.Replace('\', '/'))
        if ($path -eq $basePath -or $path -eq "$basePath/") {
            $candidateBase = $siteRootPath
        }
        elseif ($path.StartsWith("$basePath/", [System.StringComparison]::OrdinalIgnoreCase)) {
            $candidateBase = Join-Path $siteRootPath $path.Substring($basePath.Length + 1)
        }
        elseif ($path.StartsWith('/')) {
            $candidateBase = Join-Path $siteRootPath $path.TrimStart('/')
        }
        else {
            $candidateBase = Join-Path $html.DirectoryName $path
        }

        $candidateBase = [System.IO.Path]::GetFullPath($candidateBase)
        if (-not $candidateBase.StartsWith($siteRootPath, [System.StringComparison]::OrdinalIgnoreCase)) {
            $problems.Add("Link escapes Pages output in $($html.FullName): $value")
            continue
        }

        $candidates = [System.Collections.Generic.List[string]]::new()
        $candidates.Add($candidateBase)
        if ($path.EndsWith('/')) {
            $candidates.Add((Join-Path $candidateBase 'index.html'))
        }
        elseif ([string]::IsNullOrWhiteSpace([System.IO.Path]::GetExtension($candidateBase))) {
            $candidates.Add("$candidateBase.html")
            $candidates.Add((Join-Path $candidateBase 'index.html'))
        }

        $exists = $false
        foreach ($candidate in $candidates) {
            if (Test-Path -LiteralPath $candidate) {
                $exists = $true
                break
            }
        }
        if (-not $exists) {
            $relativeHtml = $html.FullName.Substring($siteRootPath.Length).TrimStart('\', '/')
            $problems.Add("Broken local link in ${relativeHtml}: $value")
        }
    }
}

if ($problems.Count -gt 0) {
    $problems | Sort-Object -Unique | ForEach-Object { [Console]::Error.WriteLine($_) }
    throw "Pages output validation failed with $($problems.Count) problem(s)."
}

Write-Output "SITE_ROOT=$siteRootPath"
Write-Output "FILES=$($files.Count) HTML=$($htmlFiles.Count) BROKEN_LINKS=0"
