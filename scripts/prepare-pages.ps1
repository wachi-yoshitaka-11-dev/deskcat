[CmdletBinding()]
param()

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$repositoryRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$outputRoot = [System.IO.Path]::GetFullPath((Join-Path $repositoryRoot '.pages-src'))
$expectedOutput = [System.IO.Path]::GetFullPath((Join-Path $repositoryRoot '.pages-src'))

if (-not $outputRoot.Equals($expectedOutput, [System.StringComparison]::OrdinalIgnoreCase)) {
    throw "Unexpected Pages staging path: $outputRoot"
}

if (Test-Path -LiteralPath $outputRoot) {
    Remove-Item -LiteralPath $outputRoot -Recurse -Force
}

$null = New-Item -ItemType Directory -Path $outputRoot

$portalRoot = Join-Path $repositoryRoot 'pages'
$portalFiles = @('_config.yml', 'index.md', '404.md')
foreach ($name in $portalFiles) {
    $source = Join-Path $portalRoot $name
    if (-not (Test-Path -LiteralPath $source -PathType Leaf)) {
        throw "Required Pages source is missing: $source"
    }
    Copy-Item -LiteralPath $source -Destination (Join-Path $outputRoot $name)
}

# Pages固有のimageとstylesheetだけを`assets/`から公開する。
# 技術文書用のimageはここへ置かない。
$assetsSource = Join-Path $portalRoot 'assets'
$assetsDestination = Join-Path $outputRoot 'assets'
if (-not (Test-Path -LiteralPath $assetsSource -PathType Container)) {
    throw "Required Pages assets directory is missing: $assetsSource"
}
Copy-Item -LiteralPath $assetsSource -Destination $assetsDestination -Recurse

# GitHub Pagesのthemeが各pageから参照するfaviconを、依存toolなしで生成する。
# 1 x 1 pixel、32-bit BGRAの最小ICOであり、公開文書のbuild成否だけに影響する。
[byte[]]$faviconBytes = @(
    0x00, 0x00, 0x01, 0x00, 0x01, 0x00,
    0x01, 0x01, 0x00, 0x00, 0x01, 0x00, 0x20, 0x00,
    0x30, 0x00, 0x00, 0x00, 0x16, 0x00, 0x00, 0x00,
    0x28, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00,
    0x02, 0x00, 0x00, 0x00, 0x01, 0x00, 0x20, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x66, 0x99, 0xCC, 0xFF, 0x00, 0x00, 0x00, 0x00
)
[System.IO.File]::WriteAllBytes((Join-Path $outputRoot 'favicon.ico'), $faviconBytes)

$rootDocuments = @('README.md', 'AGENTS.md', 'CONTRIBUTING.md', 'SECURITY.md', 'LICENSE')
foreach ($name in $rootDocuments) {
    $source = Join-Path $repositoryRoot $name
    if (-not (Test-Path -LiteralPath $source -PathType Leaf)) {
        throw "Required root document is missing: $source"
    }
    Copy-Item -LiteralPath $source -Destination (Join-Path $outputRoot $name)
}

$docsSource = Join-Path $repositoryRoot 'docs'
$docsDestination = Join-Path $outputRoot 'docs'
if (-not (Test-Path -LiteralPath $docsSource -PathType Container)) {
    throw "Required docs directory is missing: $docsSource"
}
Copy-Item -LiteralPath $docsSource -Destination $docsDestination -Recurse

$allowedExtensions = @(
    '.css', '.gif', '.html', '.ico', '.jpeg', '.jpg', '.markdown', '.md',
    '.png', '.scss', '.svg', '.txt', '.webp', '.yaml', '.yml'
)
$textExtensions = @('.css', '.html', '.markdown', '.md', '.scss', '.svg', '.txt', '.yaml', '.yml')
# Extensionを問わず全fileへ適用する。`.svg`はtext扱いだが、image同様に
# 大きくなり得るため除外しない。現時点の最大fileは76 KiBである。
$fileSizeLimit = 1MB
$secretPattern = 'ghp_[A-Za-z0-9]{20,}|github_pat_[A-Za-z0-9_]{20,}|AKIA[0-9A-Z]{16}|-----BEGIN (?:RSA |EC |OPENSSH )?PRIVATE KEY-----'
$personalPathPattern = 'C:\\Users\\[^\\\s]+|/home/[^/\s]+|file://'
$problems = [System.Collections.Generic.List[string]]::new()

$files = @(Get-ChildItem -LiteralPath $outputRoot -Recurse -Force -File)
foreach ($file in $files) {
    if (($file.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) {
        $problems.Add("Symbolic or reparse-point file is not allowed: $($file.FullName)")
    }

    $extension = $file.Extension.ToLowerInvariant()
    $isLicense = $file.Name.Equals('LICENSE', [System.StringComparison]::OrdinalIgnoreCase)
    if ($extension -notin $allowedExtensions -and -not $isLicense) {
        $problems.Add("File type is not approved for Pages: $($file.FullName)")
        continue
    }

    if ($file.Length -gt $fileSizeLimit) {
        $problems.Add("File exceeds the Pages size limit: $($file.FullName)")
    }

    if ($extension -in $textExtensions -or $isLicense) {
        $content = Get-Content -LiteralPath $file.FullName -Raw
        if ([regex]::IsMatch($content, $secretPattern)) {
            $problems.Add("Secret-like content detected: $($file.FullName)")
        }
        if ([regex]::IsMatch($content, $personalPathPattern, [System.Text.RegularExpressions.RegexOptions]::IgnoreCase)) {
            $problems.Add("Personal absolute path detected: $($file.FullName)")
        }
    }
}

$markdownCount = @($files | Where-Object { $_.Extension -in @('.md', '.markdown') }).Count
if ($markdownCount -lt 35) {
    $problems.Add("Unexpectedly small Markdown set: $markdownCount")
}

if ($problems.Count -gt 0) {
    $problems | ForEach-Object { [Console]::Error.WriteLine($_) }
    throw "Pages staging validation failed with $($problems.Count) problem(s)."
}

Write-Output "PAGES_SOURCE=$outputRoot"
Write-Output "FILES=$($files.Count) MARKDOWN=$markdownCount"
