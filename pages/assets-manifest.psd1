# GitHub Pagesへ公開する`pages/assets/`配下のasset manifest。
#
# `scripts/prepare-pages.ps1`はここに列挙したexact pathだけをstagingする。
# 列挙外のfileは、`pages/assets/`へ置いてもstagingせず、buildを失敗させる。
#
# `Path`は`pages/assets/`からの相対pathとする。`..`、絶対path、rootedな
# pathは受け付けない。列挙したfileはGitの追跡対象でなければならない。
#
# `Sha256`はbinary assetにのみ指定する。Binaryは内容scanが効かないため、
# hashで同一性を固定する。差し替え時は本fileと
# `docs/governance/published-asset-register.md`の両方を更新する。
# Text asset（stylesheet等）はdiff reviewと内容scanの対象であり、
# 編集ごとにhashが変わるだけなので指定しない。
#
# 出所と再配布許諾は`docs/governance/published-asset-register.md`が正本。

@{
    Assets = @(
        @{
            Path = 'css/style.scss'
            Note = 'Cayman themeのoverride stylesheet。配色とtypographyをconcept資料へ合わせる'
        }
        @{
            Path   = 'deskcat-concept.jpg'
            Sha256 = '615063ED60596F55066D602E1C44ACFB46D6D3103B9D234AEB8170E864FBB5B2'
            Note   = '入口pageのconcept image。720 x 720。出所と許諾は公開asset registerを参照'
        }
    )
}
