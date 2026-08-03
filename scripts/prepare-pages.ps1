[CmdletBinding()]
param()

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

. (Join-Path $PSScriptRoot 'lib/publish-guards.ps1')

$repositoryRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$outputRoot = [System.IO.Path]::GetFullPath((Join-Path $repositoryRoot '.pages-src'))
$expectedOutput = [System.IO.Path]::GetFullPath((Join-Path $repositoryRoot '.pages-src'))

if (-not $outputRoot.Equals($expectedOutput, [System.StringComparison]::OrdinalIgnoreCase)) {
    throw 'Unexpected Pages staging path.'
}

if (Test-Path -LiteralPath $outputRoot) {
    Remove-Item -LiteralPath $outputRoot -Recurse -Force
}

$null = New-Item -ItemType Directory -Path $outputRoot

# Gitのindexにpathが存在することを確認するguardを、portal fileとroot documentにも
# 適用する。これは追跡外pathを公開対象にしないための判定であり、working treeの内容が
# commit済み・review済みであることまでは証明しない。Production deployはmainのcleanな
# CI checkoutだけから行い、内容のreview境界はそちらで担保する。
$trackedRepositoryFiles = Get-DeskCatTrackedFiles -RepositoryRoot $repositoryRoot -PathSpec '.'

# Gitがsymlinkとして記録しているpath。file属性で判定すると、`core.symlinks=false`の
# checkoutではregular fileに見えるため、複製するかどうかが環境ごとに変わる。
# indexのmodeは環境に依存しない。
$trackedSymlinks = Get-DeskCatTrackedSymlinks -RepositoryRoot $repositoryRoot

$portalRoot = Join-Path $repositoryRoot 'pages'
$portalFiles = @('_config.yml', 'index.md', '404.md')
foreach ($name in $portalFiles) {
    $source = Join-Path $portalRoot $name
    if (-not (Test-Path -LiteralPath $source -PathType Leaf)) {
        throw "Required Pages source is missing: pages/$name"
    }
    if (-not $trackedRepositoryFiles.Contains("pages/$name")) {
        throw "Required Pages source is not tracked by Git: pages/$name"
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
    throw 'Required Pages assets directory is missing: pages/assets'
}
if (-not (Test-Path -LiteralPath $assetManifestPath -PathType Leaf)) {
    throw 'Required Pages asset manifest is missing: pages/assets-manifest.psd1'
}

# `Import-PowerShellDataFile`はdataだけを読み、manifest内のcodeを実行しない。
$assetManifest = Import-PowerShellDataFile -LiteralPath $assetManifestPath
if (-not $assetManifest.ContainsKey('Assets')) {
    throw 'Pages asset manifest has no Assets key: pages/assets-manifest.psd1'
}

# Git追跡対象だけを公開する。追跡外のfileをmanifestへ書いても公開しない。
# manifestのPathはslash区切りで比較するため、区切り文字を変換しない。
$trackedAssets = Get-DeskCatTrackedFiles -RepositoryRoot $repositoryRoot -PathSpec 'pages/assets'

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
    $onDisk = Get-DeskCatPathRelativeToRoot -Path $item.FullName -Root $assetsSource
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

$rootDocuments = $script:DeskCatRootDocuments
foreach ($name in $rootDocuments) {
    $source = Join-Path $repositoryRoot $name
    if (-not (Test-Path -LiteralPath $source -PathType Leaf)) {
        throw "Required root document is missing: $name"
    }
    if (-not $trackedRepositoryFiles.Contains($name)) {
        throw "Required root document is not tracked by Git: $name"
    }
    Copy-Item -LiteralPath $source -Destination (Join-Path $outputRoot $name)
}

# `docs/` はMarkdownだけを複製する。
# 再帰的な一括copyにすると、docs/へ置いたfileが人手のreviewを経ずに公開される。
# 特に画像はbinaryのため下の内容scanが効かず、EXIFや写り込みを検出できない。
$docsSource = Join-Path $repositoryRoot 'docs'
$docsDestination = Join-Path $outputRoot 'docs'
if (-not (Test-Path -LiteralPath $docsSource -PathType Container)) {
    throw 'Required docs directory is missing: docs'
}

$null = New-Item -ItemType Directory -Path $docsDestination
$skipped = [System.Collections.Generic.List[string]]::new()
$copied = 0

# Gitが追跡しているfileだけを公開する。
# CIはclean checkoutのため差は出ないが、local実行では未追跡の下書きがdocs/に
# 残っていることがある。追跡状態で絞り、localとCIのstaging結果を一致させる。
$trackedDocs = Get-DeskCatTrackedFiles -RepositoryRoot $repositoryRoot -PathSpec 'docs'

foreach ($item in @(Get-ChildItem -LiteralPath $docsSource -Recurse -Force -File)) {
    $relative = Get-DeskCatPathRelativeToRoot -Path $item.FullName -Root $docsSource
    $repoRelative = Get-DeskCatPathRelativeToRoot -Path $item.FullName -Root $repositoryRoot

    # Gitのmodeを先に見る。属性だけだと`core.symlinks=false`のcheckoutで
    # symlinkがregular fileとして複製され、環境ごとに公開物が変わる。
    if ($trackedSymlinks.Contains($repoRelative)) {
        $skipped.Add("$relative (symlink in Git)")
        continue
    }

    if (($item.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) {
        $skipped.Add("$relative (reparse point)")
        continue
    }

    if (-not $trackedDocs.Contains($repoRelative)) {
        $skipped.Add("$relative (not tracked by Git)")
        continue
    }

    if ($item.Extension.ToLowerInvariant() -notin $script:DeskCatDocsCopyExtensions) {
        $skipped.Add("$relative (not Markdown)")
        continue
    }

    $target = Join-Path $docsDestination $relative
    $targetDirectory = Split-Path -Parent $target
    if (-not (Test-Path -LiteralPath $targetDirectory)) {
        $null = New-Item -ItemType Directory -Path $targetDirectory -Force
    }
    Copy-Item -LiteralPath $item.FullName -Destination $target
    $copied++
}

# Staging対象の全fileへ適用するsize上限。extension条件を付けない。
# `.svg`はtext扱いだがimage同様に大きくなり得るため、除外すると検査から漏れる。
$fileSizeLimit = 1MB
$problems = [System.Collections.Generic.List[string]]::new()

$files = @(Get-ChildItem -LiteralPath $outputRoot -Recurse -Force -File)
foreach ($file in $files) {
    $relativeFile = Get-DeskCatPathRelativeToRoot -Path $file.FullName -Root $outputRoot
    if (($file.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) {
        $problems.Add("Symbolic or reparse-point file is not allowed: $relativeFile")
    }

    $extension = $file.Extension.ToLowerInvariant()
    # 拡張子allowlistの例外はroot直下の`LICENSE`1 fileに限る。file名だけで判定すると、
    # manifestが宣言した`assets/...license`のような拡張子なしpathも例外を通る。
    $isLicense = $file.FullName.Equals(
        (Join-Path $outputRoot 'LICENSE'),
        [System.StringComparison]::Ordinal)
    if ($extension -notin $script:DeskCatAllowedExtensions -and -not $isLicense) {
        $problems.Add("File type is not approved for Pages: $relativeFile")
        continue
    }

    if ($file.Length -gt $fileSizeLimit) {
        $problems.Add("File exceeds the Pages size limit: $relativeFile")
    }

    if ($extension -in $script:DeskCatTextExtensions -or $isLicense) {
        $content = Get-DeskCatFileText -Path $file.FullName
        if (Test-DeskCatSecretLike -Content $content) {
            $problems.Add("Secret-like content detected: $relativeFile")
        }
        if (Test-DeskCatPersonalPath -Content $content) {
            $problems.Add("Personal absolute path detected: $relativeFile")
        }
    }
}

# 大量削除に気付くための下限。stagingするMarkdownが減ったら検知する。
# 実際の件数から余裕を取った値であり、増えた分に追従して上げる必要はない。
$minimumMarkdownCount = $script:DeskCatMinimumPublishedCount
# `docs/`の複製条件と同じ拡張子集合を使う。Jekyllがrenderするのはこの集合であり、
# 二箇所で列挙すると、拡張子を増やしたときに件数checkだけ追従しない。
$markdownCount = @($files | Where-Object { $_.Extension -in $script:DeskCatDocsCopyExtensions }).Count
if ($markdownCount -lt $minimumMarkdownCount) {
    $problems.Add("Unexpectedly small Markdown set: $markdownCount (minimum $minimumMarkdownCount)")
}

if ($skipped.Count -gt 0) {
    [Console]::Error.WriteLine("Skipped files under docs/ (not published):")
    $skipped | ForEach-Object { [Console]::Error.WriteLine("  $_") }
}

if ($problems.Count -gt 0) {
    $problems | ForEach-Object { [Console]::Error.WriteLine($_) }
    throw "Pages staging validation failed with $($problems.Count) problem(s)."
}

Write-Output 'PAGES_SOURCE=.pages-src'
Write-Output "FILES=$($files.Count) MARKDOWN=$markdownCount DOCS_COPIED=$copied DOCS_SKIPPED=$($skipped.Count)"
