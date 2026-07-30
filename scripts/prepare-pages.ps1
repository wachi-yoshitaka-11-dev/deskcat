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

# Pages固有のassetは、review済みmanifestが列挙したexact pathだけを公開する。
# 再帰copyにすると、許可拡張子でありreviewを経ていないfileまで公開され得る。
# 特にbinaryは下の内容scanが効かないため、hashで同一性を固定する。
$assetsSource = Join-Path $portalRoot 'assets'
$assetsDestination = Join-Path $outputRoot 'assets'
$assetManifestPath = Join-Path $portalRoot 'assets-manifest.psd1'

if (-not (Test-Path -LiteralPath $assetsSource -PathType Container)) {
    throw "Required Pages assets directory is missing: $assetsSource"
}
if (-not (Test-Path -LiteralPath $assetManifestPath -PathType Leaf)) {
    throw "Required Pages asset manifest is missing: $assetManifestPath"
}

# `Import-PowerShellDataFile`はdataだけを読み、manifest内のcodeを実行しない。
$assetManifest = Import-PowerShellDataFile -LiteralPath $assetManifestPath
if (-not $assetManifest.ContainsKey('Assets')) {
    throw "Pages asset manifest has no Assets key: $assetManifestPath"
}

# Git追跡対象だけを公開する。追跡外のfileをmanifestへ書いても公開しない。
# 比較はcase-sensitiveにする。CIのubuntu-24.04はcase-sensitive filesystemであり、
# case違いをOrdinalIgnoreCaseで「追跡済み」と誤判定すると未reviewのfileが公開される。
$trackedAssets = [System.Collections.Generic.HashSet[string]]::new(
    [System.StringComparer]::Ordinal)
$gitAssetOutput = @(& git -C $repositoryRoot ls-files 'pages/assets' 2>$null)
if ($LASTEXITCODE -ne 0) {
    throw "Unable to enumerate tracked files under pages/assets/. Run inside a Git checkout."
}
foreach ($line in $gitAssetOutput) {
    $null = $trackedAssets.Add($line)
}

$assetProblems = [System.Collections.Generic.List[string]]::new()
$declaredAssets = [System.Collections.Generic.HashSet[string]]::new(
    [System.StringComparer]::Ordinal)
$textAssetExtensions = @('.css', '.scss', '.svg', '.txt')

$null = New-Item -ItemType Directory -Path $assetsDestination
foreach ($entry in @($assetManifest.Assets)) {
    if (-not $entry.ContainsKey('Path')) {
        $assetProblems.Add('Asset manifest entry has no Path.')
        continue
    }
    $relative = [string]$entry.Path

    # Manifestの`Path`でstaging先を`assets/`の外へ逃がせないようにする。
    if (
        [string]::IsNullOrWhiteSpace($relative) -or
        $relative.Contains('..') -or
        $relative.StartsWith('/') -or
        $relative.StartsWith('\') -or
        [System.IO.Path]::IsPathRooted($relative)
    ) {
        $assetProblems.Add("Asset manifest Path is not a safe relative path: $relative")
        continue
    }

    $normalized = $relative.Replace('\', '/')
    $null = $declaredAssets.Add($normalized)
    $repoRelative = "pages/assets/$normalized"
    $source = Join-Path $assetsSource ($normalized -replace '/', [string][System.IO.Path]::DirectorySeparatorChar)

    if (-not (Test-Path -LiteralPath $source -PathType Leaf)) {
        $assetProblems.Add("Declared asset is missing: $repoRelative")
        continue
    }
    if (-not $trackedAssets.Contains($repoRelative)) {
        $assetProblems.Add("Declared asset is not tracked by Git: $repoRelative")
        continue
    }

    $extension = [System.IO.Path]::GetExtension($normalized).ToLowerInvariant()
    $isTextAsset = $extension -in $textAssetExtensions
    $hasHash = $entry.ContainsKey('Sha256')

    if ($isTextAsset -and $hasHash) {
        $assetProblems.Add("Text asset must not declare Sha256: $repoRelative")
        continue
    }
    if (-not $isTextAsset) {
        if (-not $hasHash) {
            $assetProblems.Add("Binary asset must declare Sha256: $repoRelative")
            continue
        }
        $actualHash = (Get-FileHash -LiteralPath $source -Algorithm SHA256).Hash
        if (-not $actualHash.Equals([string]$entry.Sha256, [System.StringComparison]::OrdinalIgnoreCase)) {
            $assetProblems.Add("Asset SHA-256 does not match the manifest: $repoRelative (actual $actualHash)")
            continue
        }
    }

    $target = Join-Path $assetsDestination ($normalized -replace '/', [string][System.IO.Path]::DirectorySeparatorChar)
    $targetDirectory = Split-Path -Parent $target
    if (-not (Test-Path -LiteralPath $targetDirectory)) {
        $null = New-Item -ItemType Directory -Path $targetDirectory -Force
    }
    Copy-Item -LiteralPath $source -Destination $target
}

# Manifestに無いfileを検知する。追跡状態にかかわらず失敗させ、localとCIで
# 同じ結果にする。`pages/assets/`は少数の選定済みassetだけを置く場所である。
foreach ($item in @(Get-ChildItem -LiteralPath $assetsSource -Recurse -Force -File)) {
    $onDisk = $item.FullName.Substring($assetsSource.Length).TrimStart('\', '/').Replace('\', '/')
    if (-not $declaredAssets.Contains($onDisk)) {
        $assetProblems.Add("Asset is not declared in the manifest: pages/assets/$onDisk")
    }
}

if ($assetProblems.Count -gt 0) {
    $assetProblems | ForEach-Object { [Console]::Error.WriteLine($_) }
    throw "Pages asset manifest validation failed with $($assetProblems.Count) problem(s)."
}

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
