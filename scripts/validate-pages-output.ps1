[CmdletBinding()]
param(
    [string]$SiteRoot = '',
    [string]$PagesConfigPath = ''
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

. (Join-Path $PSScriptRoot 'lib/publish-guards.ps1')

function Get-DeskCatYamlScalarWithoutComment {
    param(
        [Parameter(Mandatory)][string]$Value
    )

    # YAMLのplain scalarでは、ASCII space／tabの後ろにある`#`からcommentが始まる。
    # quoted scalar内の`#`や、double quote内のescape、single quoteの二重化は
    # comment境界として扱わない。
    $quote = [char]0
    for ($i = 0; $i -lt $Value.Length; $i++) {
        $character = $Value[$i]
        if ($quote -eq [char]34) {
            if ($character -eq [char]92) {
                $i++
            }
            elseif ($character -eq [char]34) {
                $quote = [char]0
            }
            continue
        }
        if ($quote -eq [char]39) {
            if ($character -eq [char]39) {
                if ($i + 1 -lt $Value.Length -and $Value[$i + 1] -eq [char]39) {
                    $i++
                }
                else {
                    $quote = [char]0
                }
            }
            continue
        }
        if ($character -eq [char]34 -or $character -eq [char]39) {
            $quote = $character
            continue
        }
        if ($character -eq [char]35 -and
            ($i -eq 0 -or $Value[$i - 1] -eq [char]32 -or $Value[$i - 1] -eq [char]9)) {
            return $Value.Substring(0, $i).TrimEnd([char]32, [char]9)
        }
    }

    return $Value.TrimEnd([char]32, [char]9)
}

$repositoryRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
if ([string]::IsNullOrWhiteSpace($SiteRoot)) {
    $SiteRoot = Join-Path $repositoryRoot '_site'
}
$siteRootPath = [System.IO.Path]::GetFullPath($SiteRoot)

if (-not (Test-Path -LiteralPath $siteRootPath -PathType Container)) {
    # user指定pathにはusername等が含まれ得るため、local絶対pathをCI logへ出さない。
    throw 'Pages output directory does not exist.'
}

# Pages base URLの正本はJekyll設定である。validatorへrepository名を重複記述すると、
# renameやcustom baseurl変更後に古いpathを正しいものとして検査してしまう。
if ([string]::IsNullOrWhiteSpace($PagesConfigPath)) {
    $PagesConfigPath = Join-Path $repositoryRoot 'pages/_config.yml'
}
$pagesConfigFullPath = [System.IO.Path]::GetFullPath($PagesConfigPath)
if (-not (Test-Path -LiteralPath $pagesConfigFullPath -PathType Leaf)) {
    throw 'Pages config file does not exist.'
}
$baseUrlValues = [System.Collections.Generic.List[string]]::new()
foreach ($line in (Get-DeskCatFileText -Path $pagesConfigFullPath) -split '\r?\n') {
    # Jekyllのbaseurlはcase-sensitiveなtop-level keyである。indentされた同名keyや
    # `BaseUrl`のような別keyを正本として読まない。
    if ($line -cnotmatch '^baseurl[ \t]*:(?<after>.*)$') {
        continue
    }
    $afterColon = $Matches['after']
    # block mappingのcolon後が非空なら、valueとのseparatorはASCII space／tabである。
    # `baseurl:/deskcat`をkey/valueとして受理するとJekyllのYAML解釈とずれる。
    if ($afterColon.Length -gt 0 -and
        $afterColon[0] -ne [char]32 -and $afterColon[0] -ne [char]9) {
        continue
    }
    $rawBaseUrl = $afterColon.Trim([char]32, [char]9)
    $baseUrlValues.Add((Get-DeskCatYamlScalarWithoutComment -Value $rawBaseUrl))
}
if ($baseUrlValues.Count -ne 1) {
    throw "Pages config must define exactly one baseurl (found $($baseUrlValues.Count))."
}
$basePath = $baseUrlValues[0].Trim([char]32, [char]9)
if ($basePath.Length -ge 2 -and
    (($basePath[0] -eq [char]34 -and $basePath[-1] -eq [char]34) -or
    ($basePath[0] -eq [char]39 -and $basePath[-1] -eq [char]39))) {
    $basePath = $basePath.Substring(1, $basePath.Length - 2)
}
if (-not [string]::IsNullOrEmpty($basePath) -and
    (-not $basePath.StartsWith('/', [System.StringComparison]::Ordinal) -or
    $basePath.Contains('//') -or
    $basePath -match '[\x00-\x1f\x7f?#\\]' -or
    $basePath -match '\s' -or
    $basePath -match '(?:^|/)\.\.?(?:/|$)')) {
    # config値自体がsecretや個人pathである可能性があるため、値をCI logへ再掲しない。
    throw 'Pages config contains an unsafe baseurl.'
}
$basePath = $basePath.TrimEnd('/')

$requiredFiles = @(
    'index.html',
    '404.html',
    'favicon.ico',
    'assets/css/style.css',
    'assets/deskcat-concept.jpg',
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

$problems = [System.Collections.Generic.List[string]]::new()
$siteRootItem = Get-Item -LiteralPath $siteRootPath -Force
# rootがreparse pointなら、target treeを走査する前に停止する。走査後の報告では
# SiteRoot外の内容を読んだ後になり、公開境界の検査順として遅い。
if (($siteRootItem.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) {
    $relativeRoot = Get-DeskCatPathRelativeToRoot -Path $siteRootItem.FullName -Root $siteRootPath
    throw "Symbolic or reparse-point output is not allowed at root: $relativeRoot"
}
$outputItems = @($siteRootItem) + @(Get-ChildItem -LiteralPath $siteRootPath -Recurse -Force)
foreach ($item in $outputItems) {
    # fileだけでなくdirectoryとSiteRoot自身も検査する。directory reparse pointを
    # 見落とすと、lexicalには_site内でも実体が外にあるfileをlink先として受理し得る。
    if (($item.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) {
        $relativeItem = Get-DeskCatPathRelativeToRoot -Path $item.FullName -Root $siteRootPath
        $problems.Add("Symbolic or reparse-point output is not allowed: $relativeItem")
    }
}

$files = @($outputItems | Where-Object { -not $_.PSIsContainer })
$publishedFilePaths = [System.Collections.Generic.HashSet[string]]::new(
    [System.StringComparer]::Ordinal)
foreach ($file in $files) {
    $null = $publishedFilePaths.Add([System.IO.Path]::GetFullPath($file.FullName))
    $relativeFile = Get-DeskCatPathRelativeToRoot -Path $file.FullName -Root $siteRootPath

    # 拡張子の判定はcase-insensitiveのままにする。`.PDF`も拒否し、`.HTML`も
    # scan対象に含めるためであり、広く拾う方向がfail-safeになる。
    # 一方、pathのcontainment、base path、存在するfileの突き合わせはcase-sensitiveにする。
    if ($file.Extension.Equals('.pdf', [System.StringComparison]::OrdinalIgnoreCase)) {
        $problems.Add("PDF output is not allowed: $relativeFile")
    }
}

# `Test-Path`だけではWindows上でcase違いのfileを存在扱いにする。実際に列挙した
# fileのOrdinal集合と突き合わせ、LinuxのPagesと同じcase-sensitiveな結果にする。
foreach ($relativePath in $requiredFiles) {
    $candidate = [System.IO.Path]::GetFullPath((Join-Path $siteRootPath $relativePath))
    if (-not $publishedFilePaths.Contains($candidate) -or
        -not (Test-Path -LiteralPath $candidate -PathType Leaf)) {
        $problems.Add("Required output is missing: $relativePath")
    }
}

# 下限はpublish-guards.ps1が持つ。ここで再定義しない。
$minimumHtmlCount = $script:DeskCatMinimumPublishedCount
$htmlFiles = @($files | Where-Object { $_.Extension.Equals('.html', [System.StringComparison]::OrdinalIgnoreCase) })
if ($htmlFiles.Count -lt $minimumHtmlCount) {
    $problems.Add("Unexpectedly small HTML set: $($htmlFiles.Count) (minimum $minimumHtmlCount)")
}

# 公開禁止patternはHTMLだけでなく、生成CSS／SVG等を含む全text出力へ適用する。
# source側はprepare-pages.ps1が検査するが、Jekyll変換後にだけ現れる内容もあるため、
# 最終artifactを同じ共有拡張子で再検査する。
$sensitiveTextFiles = [System.Collections.Generic.HashSet[string]]::new(
    [System.StringComparer]::Ordinal)
foreach ($file in $files) {
    $isLicense = $file.Name -ceq 'LICENSE'
    if ($file.Extension.ToLowerInvariant() -notin $script:DeskCatTextExtensions -and -not $isLicense) {
        continue
    }
    $content = Get-DeskCatFileText -Path $file.FullName
    $relativeFile = Get-DeskCatPathRelativeToRoot -Path $file.FullName -Root $siteRootPath
    if (Test-DeskCatSecretLike -Content $content) {
        $problems.Add("Secret-like content detected: $relativeFile")
        $null = $sensitiveTextFiles.Add($file.FullName)
    }
    if (Test-DeskCatPersonalPath -Content $content) {
        $problems.Add("Personal absolute path detected: $relativeFile")
        $null = $sensitiveTextFiles.Add($file.FullName)
    }
}

function Test-DeskCatHtmlSpace {
    param(
        [Parameter(Mandatory)][char]$Character
    )

    # HTML tokenizerがseparatorとして扱うASCII whitespaceだけを受理する。
    # Char.IsWhiteSpaceはNBSP等も含み、browserの属性境界とずれる。
    return ([int]$Character -in 0x09, 0x0a, 0x0c, 0x0d, 0x20)
}

function Get-DeskCatDiagnosticText {
    param(
        [Parameter(Mandatory)][string]$Value
    )

    # 生成物由来の値をCI logへそのまま書かない。改行、terminal escape、bidi制御等を
    # printable placeholderへ変え、極端に長いattributeでlog容量を占有させない。
    $safe = $Value -replace '[\p{Cc}\p{Cf}\p{Zl}\p{Zp}]', '?'
    if ($safe.Length -gt 240) {
        return $safe.Substring(0, 240) + '...'
    }
    return $safe
}

function Get-DeskCatHtmlStartTagAttribute {
    param(
        [Parameter(Mandatory)][string]$StartTag
    )

    # 開始tagを先頭からtokenizeする。tag全体へのattribute regexは、quoted value内の
    # `href=`や`id=`まで実属性として拾うため使わない。quoted／unquoted／boolean属性と
    # `xlink:href`のようなnamespace付き属性名を同じ境界規則で扱う。
    if ($StartTag.Length -lt 3 -or $StartTag[0] -ne [char]60) {
        return
    }

    $seenNames = [System.Collections.Generic.HashSet[string]]::new(
        [System.StringComparer]::OrdinalIgnoreCase)
    $index = 1
    while ($index -lt $StartTag.Length -and
        -not (Test-DeskCatHtmlSpace -Character $StartTag[$index]) -and
        $StartTag[$index] -ne [char]'/' -and
        $StartTag[$index] -ne [char]'>') {
        $index++
    }

    while ($index -lt $StartTag.Length) {
        while ($index -lt $StartTag.Length -and
            (Test-DeskCatHtmlSpace -Character $StartTag[$index])) {
            $index++
        }
        if ($index -ge $StartTag.Length -or $StartTag[$index] -eq [char]'>') {
            break
        }
        if ($StartTag[$index] -eq [char]'/' -and
            $index + 1 -lt $StartTag.Length -and $StartTag[$index + 1] -eq [char]'>') {
            break
        }

        $nameStart = $index
        while ($index -lt $StartTag.Length -and
            -not (Test-DeskCatHtmlSpace -Character $StartTag[$index]) -and
            $StartTag[$index] -ne [char]'=' -and
            $StartTag[$index] -ne [char]'/' -and
            $StartTag[$index] -ne [char]'>' -and
            $StartTag[$index] -ne [char]60 -and
            $StartTag[$index] -ne [char]'"' -and
            $StartTag[$index] -ne [char]39) {
            $index++
        }
        if ($index -eq $nameStart) {
            # malformedな1文字で停止せず、次の境界を検査する。開始tag抽出側が
            # quoteを閉じたtagだけを渡すため、ここでquoted valueへ入ることはない。
            $index++
            continue
        }

        $name = $StartTag.Substring($nameStart, $index - $nameStart)
        while ($index -lt $StartTag.Length -and
            (Test-DeskCatHtmlSpace -Character $StartTag[$index])) {
            $index++
        }

        $value = ''
        if ($index -lt $StartTag.Length -and $StartTag[$index] -eq [char]'=') {
            $index++
            while ($index -lt $StartTag.Length -and
                (Test-DeskCatHtmlSpace -Character $StartTag[$index])) {
                $index++
            }
            if ($index -lt $StartTag.Length -and
                ($StartTag[$index] -eq [char]'"' -or $StartTag[$index] -eq [char]39)) {
                $quote = $StartTag[$index]
                $index++
                $valueStart = $index
                while ($index -lt $StartTag.Length -and $StartTag[$index] -ne $quote) {
                    $index++
                }
                $value = $StartTag.Substring($valueStart, $index - $valueStart)
                if ($index -lt $StartTag.Length) {
                    $index++
                }
            }
            else {
                $valueStart = $index
                while ($index -lt $StartTag.Length -and
                    -not (Test-DeskCatHtmlSpace -Character $StartTag[$index]) -and
                    $StartTag[$index] -ne [char]'>') {
                    $index++
                }
                $value = $StartTag.Substring($valueStart, $index - $valueStart)
            }
        }

        # HTML tokenizerは同じ開始tag内の後続duplicate属性をdropする。すべてを返すと、
        # browserが採用しない2個目のidをanchor集合へ加えてbroken linkを通してしまう。
        if ($seenNames.Add($name)) {
            [pscustomobject]@{
                Name  = $name
                Value = $value
            }
        }
    }
}

function Get-DeskCatHtmlStartTag {
    param(
        [Parameter(Mandatory)][string]$Content
    )

    # comment、quoted属性、raw-text／RCDATAを同じ走査状態で扱う。これらを別々の
    # global regexで除去すると、comment内の偽scriptや属性値内の`<!--`が後続の
    # 実tagまで飲み込み、navigation属性を無検査にし得る。
    $index = 0
    while ($index -lt $Content.Length) {
        $openIndex = $Content.IndexOf([char]60, $index)
        if ($openIndex -lt 0) {
            break
        }

        if ($openIndex + 3 -lt $Content.Length -and
            $Content.Substring($openIndex, 4) -ceq '<!--') {
            $commentBodyStart = $openIndex + 4
            $commentEnd = -1
            # `<!-->`と`<!--->`はparse errorだが、その`>`でcommentが終了する。
            # 通常の`-->`だけを探すと、後続の実tagをcommentとして飲み込む。
            if ($commentBodyStart -lt $Content.Length -and
                $Content[$commentBodyStart] -eq [char]'>') {
                $commentEnd = $commentBodyStart
            }
            elseif ($commentBodyStart + 1 -lt $Content.Length -and
                $Content[$commentBodyStart] -eq [char]'-' -and
                $Content[$commentBodyStart + 1] -eq [char]'>') {
                $commentEnd = $commentBodyStart + 1
            }
            $normalCommentEnd = $Content.IndexOf(
                '-->',
                $commentBodyStart,
                [System.StringComparison]::Ordinal)
            if ($normalCommentEnd -ge 0) {
                $normalCommentEnd += 2
                if ($commentEnd -lt 0 -or $normalCommentEnd -lt $commentEnd) {
                    $commentEnd = $normalCommentEnd
                }
            }
            # `--!>`もHTML tokenizerではcomment終端になる。
            $bangCommentEnd = $Content.IndexOf(
                '--!>',
                $commentBodyStart,
                [System.StringComparison]::Ordinal)
            if ($bangCommentEnd -ge 0) {
                $bangCommentEnd += 3
                if ($commentEnd -lt 0 -or $bangCommentEnd -lt $commentEnd) {
                    $commentEnd = $bangCommentEnd
                }
            }
            if ($commentEnd -lt 0) {
                # 閉じていないcommentでは残り全体がmarkupではない。
                break
            }
            $index = $commentEnd + 1
            continue
        }

        $nameStart = $openIndex + 1
        if ($nameStart -ge $Content.Length) {
            break
        }
        $first = [int]$Content[$nameStart]
        $isAsciiLetter = (($first -ge 0x41 -and $first -le 0x5a) -or
            ($first -ge 0x61 -and $first -le 0x7a))
        if (-not $isAsciiLetter) {
            $marker = $Content[$nameStart]
            if ($marker -ne [char]'/' -and $marker -ne [char]'!' -and $marker -ne [char]'?') {
                # `1 < 2`のようなtextは次の`>`までを構文要素にしない。1文字だけ進め、
                # その後ろにある実開始tagを引き続き探索する。
                $index = $openIndex + 1
                continue
            }

            if ($marker -eq [char]'!' -or $marker -eq [char]'?') {
                # comment以外のmarkup declarationとprocessing instructionは、HTMLでは
                # bogus comment等として最初の`>`でdataへ戻る。quoteに特別な意味はない。
                $bogusEnd = $Content.IndexOf([char]'>', $nameStart + 1)
                if ($bogusEnd -lt 0) { break }
                $index = $bogusEnd + 1
                continue
            }

            # `</`の直後がASCII letterでなければclosing tag tokenは始まらない。
            # textとして1文字だけ進め、後続の実開始tagを探索する。
            $closingNameStart = $nameStart + 1
            if ($closingNameStart -ge $Content.Length) { break }
            $closingFirst = [int]$Content[$closingNameStart]
            $isClosingAsciiLetter = (($closingFirst -ge 0x41 -and $closingFirst -le 0x5a) -or
                ($closingFirst -ge 0x61 -and $closingFirst -le 0x7a))
            if (-not $isClosingAsciiLetter) {
                if ($Content[$closingNameStart] -eq [char]'>') {
                    $index = $closingNameStart + 1
                }
                else {
                    $index = $openIndex + 1
                }
                continue
            }

            # closing tagのattribute-like部分ではquoted value中の`>`を終端にしない。
            $quote = [char]0
            $declarationEnd = -1
            for ($scan = $closingNameStart; $scan -lt $Content.Length; $scan++) {
                $character = $Content[$scan]
                if ($quote -ne [char]0) {
                    if ($character -eq $quote) { $quote = [char]0 }
                    continue
                }
                if ($character -eq [char]'"' -or $character -eq [char]39) {
                    $quote = $character
                }
                elseif ($character -eq [char]'>') {
                    $declarationEnd = $scan
                    break
                }
            }
            if ($declarationEnd -lt 0) { break }
            $index = $declarationEnd + 1
            continue
        }

        $tagNameEnd = $nameStart
        while ($tagNameEnd -lt $Content.Length -and
            -not (Test-DeskCatHtmlSpace -Character $Content[$tagNameEnd]) -and
            $Content[$tagNameEnd] -ne [char]'/' -and
            $Content[$tagNameEnd] -ne [char]'>') {
            $tagNameEnd++
        }
        $tagName = $Content.Substring($nameStart, $tagNameEnd - $nameStart)

        $quote = [char]0
        $tagEnd = -1
        for ($scan = $tagNameEnd; $scan -lt $Content.Length; $scan++) {
            $character = $Content[$scan]
            if ($quote -ne [char]0) {
                if ($character -eq $quote) { $quote = [char]0 }
                continue
            }
            if ($character -eq [char]'"' -or $character -eq [char]39) {
                $quote = $character
            }
            elseif ($character -eq [char]'>') {
                $tagEnd = $scan
                break
            }
        }
        if ($tagEnd -lt 0) {
            # 閉じていない開始tagの後ろをtextとして再解釈しない。
            break
        }

        $selfClosingProbe = $tagEnd - 1
        while ($selfClosingProbe -gt $openIndex -and
            (Test-DeskCatHtmlSpace -Character $Content[$selfClosingProbe])) {
            $selfClosingProbe--
        }
        $hasSelfClosingMarker = ($selfClosingProbe -gt $openIndex -and
            $Content[$selfClosingProbe] -eq [char]'/')

        $Content.Substring($openIndex, $tagEnd - $openIndex + 1)
        $index = $tagEnd + 1

        if ($tagName -eq 'plaintext') {
            if ($hasSelfClosingMarker) {
                # scannerはHTML／foreign-contentのnamespace stackを持たない。`/>`を
                # text開始として残り全体を隠すより、後続tagを検査する安全側へ倒す。
                continue
            }
            # plaintext要素の開始後はEOFまでtextであり、終了tagも認識されない。
            break
        }
        if ($tagName -in 'script', 'style', 'textarea', 'title', 'xmp', 'iframe', 'noembed', 'noframes') {
            if ($hasSelfClosingMarker) {
                # foreign contentではself-closingになる一方、HTML namespaceではparse errorに
                # なり得る。namespace非追跡のscannerでは後続の実linkを隠さない方を選ぶ。
                continue
            }
            # 対応する終了tagまでをtextとしてskipし、終了tagが無ければ残りを再走査しない。
            $closingPrefix = "</$tagName"
            $closingEnd = -1
            while ($index -lt $Content.Length) {
                $closingStart = $Content.IndexOf(
                    $closingPrefix,
                    $index,
                    [System.StringComparison]::OrdinalIgnoreCase)
                if ($closingStart -lt 0) { break }
                $afterName = $closingStart + $closingPrefix.Length
                if ($afterName -lt $Content.Length -and
                    ((Test-DeskCatHtmlSpace -Character $Content[$afterName]) -or
                    $Content[$afterName] -eq [char]'/' -or
                    $Content[$afterName] -eq [char]'>')) {
                    $closingEnd = $Content.IndexOf([char]'>', $afterName)
                    break
                }
                $index = $afterName
            }
            if ($closingEnd -lt 0) { break }
            $index = $closingEnd + 1
        }
    }
}

# base pathの比較はcase-sensitiveにする。GitHub PagesのURLはcase-sensitiveであり、
# 設定とcaseが異なるpathは実際には404になる。IgnoreCaseで比較すると、そのlinkを
# site root基準で解決して「有効」と誤判定する。

# 生成HTMLのid属性。fragment付きlinkの検証に使う。
# 元Markdownのanchorはvalidate-doc-links.ps1が検査するが、生成側のid生成規則
# （kramdownのauto_ids）がGitHubのanchor規則と一致する保証はない。
# 食い違えば、sourceで通ったlinkが生成siteで解決しない。
$idsByFile = @{}
$attributesByFile = @{}
$urlTrimCharacters = [char[]](0x00..0x20)
$scannableHtmlCount = 0
foreach ($html in $htmlFiles) {
    if ($sensitiveTextFiles.Contains($html.FullName)) {
        $attributesByFile[$html.FullName] = [System.Collections.Generic.List[object]]::new()
        $idsByFile[$html.FullName] = [System.Collections.Generic.HashSet[string]]::new(
            [System.StringComparer]::Ordinal)
        continue
    }
    $scannableHtmlCount++
    $content = Get-DeskCatFileText -Path $html.FullName
    # text nodeに表示された`href=&quot;...&quot;`やcomment内の例示を、実際の
    # navigation属性として扱わない。scannerはcomment、raw-text／RCDATA、quoted属性を
    # 一度の線形走査で区別し、raw-text要素自身の`src`等は開始tagとして保持する。
    $tags = [System.Collections.Generic.List[string]]::new()
    foreach ($tag in (Get-DeskCatHtmlStartTag -Content $content)) {
        $tags.Add($tag)
    }

    $set = [System.Collections.Generic.HashSet[string]]::new([System.StringComparer]::Ordinal)
    $attributes = [System.Collections.Generic.List[object]]::new()
    foreach ($tag in $tags) {
        foreach ($attribute in (Get-DeskCatHtmlStartTagAttribute -StartTag $tag)) {
            $attributes.Add($attribute)
            if ($attribute.Name.Equals('id', [System.StringComparison]::OrdinalIgnoreCase) -and
                -not [string]::IsNullOrEmpty($attribute.Value)) {
                $null = $set.Add([System.Net.WebUtility]::HtmlDecode($attribute.Value))
            }
        }
    }
    $attributesByFile[$html.FullName] = $attributes
    $idsByFile[$html.FullName] = $set
}

# idが1件も取れない場合、生成物ではなく検査側が壊れている可能性が高い。
# 実際に一度、patternへ制御文字が混入して全fileで0件になり、
# 25件のanchorをすべて誤検出した。0件を正常として通さない。
if ($scannableHtmlCount -gt 0 -and
    ($idsByFile.Values | ForEach-Object { $_.Count } | Measure-Object -Sum).Sum -eq 0) {
    throw "No id attributes found in any generated HTML. The anchor check is not working."
}

foreach ($html in $htmlFiles) {
    $relativeHtml = Get-DeskCatPathRelativeToRoot -Path $html.FullName -Root $siteRootPath
    # secret／個人pathを検出済みのfileは、raw URLを含むlink診断を追加で出さない。
    # file pathだけを報告して停止し、機密値そのものをCI logへ二次露出させない。
    if ($sensitiveTextFiles.Contains($html.FullName)) { continue }

    foreach ($attribute in $attributesByFile[$html.FullName]) {
        $attributeName = $attribute.Name.ToLowerInvariant()
        if ($attributeName -notin 'href', 'src', 'xlink:href') {
            continue
        }
        $rawValue = $attribute.Value
        $value = [System.Net.WebUtility]::HtmlDecode($rawValue).Trim($urlTrimCharacters)
        if ([string]::IsNullOrWhiteSpace($value)) {
            if ($attributeName -eq 'src') {
                $problems.Add("Source attribute has no resource path in ${relativeHtml}")
            }
            continue
        }
        $diagnosticValue = Get-DeskCatDiagnosticText -Value $value
        # data URIはasset manifest／size guardを迂回し、javascript URIは公開pageで
        # codeを実行できる。browserがscheme中のASCII制御文字／空白を正規化する場合も
        # あるため、それらを除いたprobeで判定し、外部linkとしてskipしない。
        $schemeProbe = $value -replace '[\x00-\x20\x7f]+', ''
        if ($schemeProbe.StartsWith('data:', [System.StringComparison]::OrdinalIgnoreCase) -or
            $schemeProbe.StartsWith('javascript:', [System.StringComparison]::OrdinalIgnoreCase)) {
            $problems.Add("Unsafe URL scheme in ${relativeHtml}: $diagnosticValue")
            continue
        }
        if ($schemeProbe.StartsWith('//', [System.StringComparison]::Ordinal) -or
            $schemeProbe.StartsWith('http:', [System.StringComparison]::OrdinalIgnoreCase) -or
            $schemeProbe.StartsWith('https:', [System.StringComparison]::OrdinalIgnoreCase) -or
            $schemeProbe.StartsWith('mailto:', [System.StringComparison]::OrdinalIgnoreCase) -or
            $schemeProbe.StartsWith('tel:', [System.StringComparison]::OrdinalIgnoreCase)) {
            continue
        }
        # URI schemeを持つ値は上のallowlistだけを外部linkとして受理する。file:、
        # vbscript:、custom scheme等をlocal pathへ落とすと、同名fileの存在だけで
        # browser上の別解釈を有効と誤判定する。
        if ($schemeProbe -match '^[A-Za-z][A-Za-z0-9+.-]*:') {
            $problems.Add("Unsafe URL scheme in ${relativeHtml}: $diagnosticValue")
            continue
        }

        # fragment内では`?`も通常の文字である。`[?#]`で同時にsplitすると
        # `#missing?part`のfragmentを空として扱い、anchor検査をskipしてしまう。
        # 先に最初の`#`でfragmentを分離し、その手前だけからqueryを除く。
        $fragmentIndex = $value.IndexOf([char]'#')
        $beforeFragment = if ($fragmentIndex -ge 0) { $value.Substring(0, $fragmentIndex) } else { $value }
        $fragment = if ($fragmentIndex -ge 0) { $value.Substring($fragmentIndex + 1) } else { '' }
        $path = ($beforeFragment -split '\?', 2)[0]
        $isSamePageFragment = (
            [string]::IsNullOrWhiteSpace($path) -and
            -not [string]::IsNullOrWhiteSpace($fragment)
        )
        if ($attributeName -eq 'src' -and [string]::IsNullOrWhiteSpace($path)) {
            $problems.Add("Source attribute has no resource path in ${relativeHtml}: $diagnosticValue")
            continue
        }
        if ([string]::IsNullOrWhiteSpace($path) -and -not $isSamePageFragment) {
            continue
        }
        if (-not $isSamePageFragment) {
            # encoded拡張子やseparatorも判定対象へ含める。decode前に`.md`を調べると
            # `source%2Emd`が禁止を迂回し、decode前だけ`\`を置換すると`%5C`の
            # path解釈がWindowsとLinuxで食い違う。
            $path = [System.Uri]::UnescapeDataString($path).Replace('\', '/')
        }
        $pathExtension = if ($isSamePageFragment) {
            ''
        }
        else {
            [System.IO.Path]::GetExtension($path).ToLowerInvariant()
        }
        if ($pathExtension -in $script:DeskCatMarkdownExtensions) {
            $problems.Add("Unconverted Markdown link in ${relativeHtml}: $diagnosticValue")
            continue
        }

        if ($isSamePageFragment) {
            $candidateBase = $html.FullName
        }
        else {
            # `-eq`はPowerShellでcase-insensitiveのため`-ceq`を使う。
            # 下の`StartsWith`の`Ordinal`と判定を揃える。
            if ($path -ceq $basePath -or $path -ceq "$basePath/") {
                $candidateBase = $siteRootPath
            }
            elseif ($path.StartsWith("$basePath/", [System.StringComparison]::Ordinal)) {
                $candidateBase = Join-Path $siteRootPath $path.Substring($basePath.Length + 1)
            }
            elseif ($path.StartsWith('/', [System.StringComparison]::Ordinal)) {
                # project Pagesで`/docs/...`はdomain rootを指し、`/deskcat/docs/...`とは
                # 別URLである。site内の同名fileへ読み替えると404を見逃す。
                $problems.Add("Root-absolute link is outside Pages base path in ${relativeHtml}: $diagnosticValue")
                continue
            }
            else {
                $candidateBase = Join-Path $html.DirectoryName $path
            }
        }

        $candidateBase = [System.IO.Path]::GetFullPath($candidateBase)
        if (-not (Test-DeskCatPathWithinRoot -Path $candidateBase -Root $siteRootPath)) {
            $problems.Add("Link escapes Pages output in ${relativeHtml}: $diagnosticValue")
            continue
        }

        $candidates = [System.Collections.Generic.List[string]]::new()
        $candidates.Add($candidateBase)
        if (-not $isSamePageFragment -and $path.EndsWith('/', [System.StringComparison]::Ordinal)) {
            $candidates.Add((Join-Path $candidateBase 'index.html'))
        }
        elseif (-not $isSamePageFragment -and [string]::IsNullOrWhiteSpace([System.IO.Path]::GetExtension($candidateBase))) {
            $candidates.Add("$candidateBase.html")
            $candidates.Add((Join-Path $candidateBase 'index.html'))
        }

        $resolvedCandidates = [System.Collections.Generic.List[string]]::new()
        foreach ($candidate in $candidates) {
            # directory自体をlink先として受理しない。末尾`/`などのdirectory URLは
            # 別candidateとして追加した`index.html`が存在する場合だけ解決する。
            # Ordinal集合との突き合わせで、Windowsでもcase違いを存在扱いにしない。
            $candidateFull = [System.IO.Path]::GetFullPath($candidate)
            if ($publishedFilePaths.Contains($candidateFull) -and
                (Test-Path -LiteralPath $candidateFull -PathType Leaf)) {
                $resolvedCandidates.Add($candidateFull)
            }
        }
        if ($resolvedCandidates.Count -eq 0) {
            $problems.Add("Broken local link in ${relativeHtml}: $diagnosticValue")
            continue
        }
        if ($resolvedCandidates.Count -gt 1) {
            $relativeTargets = @($resolvedCandidates | ForEach-Object {
                    Get-DeskCatPathRelativeToRoot -Path $_ -Root $siteRootPath
                }) -join ', '
            $problems.Add("Ambiguous local link in ${relativeHtml}: $diagnosticValue (matches: $relativeTargets)")
            continue
        }
        $resolved = $resolvedCandidates[0]

        # fragmentが生成HTMLのidに存在するか。
        if (-not [string]::IsNullOrWhiteSpace($fragment)) {
            $resolvedFull = $resolved
            # 非HTMLのLeafへのfragmentは、この検査の対象外として通す。
            # `sprite.svg#icon`や`file.pdf#page=3`のfragmentはasset内部への参照であり、
            # HTMLのid検査では判定できない。fail-openではなく検査領域の境界である。
            # HTML拡張子なのにid集合へ無い場合だけを、検査できない事実として報告する。
            $resolvedExtension = [System.IO.Path]::GetExtension($resolvedFull).ToLowerInvariant()
            if ($resolvedExtension -ne '.html') {
                continue
            }
            if ($sensitiveTextFiles.Contains($resolvedFull)) {
                $problems.Add("Unverifiable anchor in ${relativeHtml}: target HTML contains sensitive content")
                continue
            }
            if (-not $idsByFile.ContainsKey($resolvedFull)) {
                $problems.Add("Unverifiable anchor in ${relativeHtml}: $diagnosticValue (resolved file is not a scanned HTML)")
                continue
            }
            $wanted = [System.Uri]::UnescapeDataString($fragment)
            if (-not $idsByFile[$resolvedFull].Contains($wanted)) {
                $available = @($idsByFile[$resolvedFull] | Sort-Object | Select-Object -First 6 |
                    ForEach-Object { Get-DeskCatDiagnosticText -Value $_ }) -join ', '
                $problems.Add("Broken anchor in generated site: ${relativeHtml} -> $diagnosticValue (ids present: $available)")
            }
        }
    }
}

if ($problems.Count -gt 0) {
    $problems | Sort-Object -Unique | ForEach-Object { [Console]::Error.WriteLine($_) }
    throw "Pages output validation failed with $($problems.Count) problem(s)."
}

Write-Output 'SITE_ROOT=.'
Write-Output "FILES=$($files.Count) HTML=$($htmlFiles.Count) BROKEN_LINKS=0"
