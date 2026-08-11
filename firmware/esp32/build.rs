fn main() {
    // `IDF_PATH`が設定されていると、`.cargo/config.toml`が`force = true`で固定した
    // `ESP_IDF_VERSION`（v5.5.3）より優先され、Version Recordが識別していないESP-IDFで
    // buildされる。生成物は記録と対応しなくなるが、buildは成功するため気づけない。
    //
    // `[env]`ではこの変数を保護できない。managed installを前提としており`IDF_PATH`を
    // 固定値にできないためである。開発端末のsetup手順は「build前に毎回確認する」と
    // していたが、確認は付け忘れる。ここで止める。manifestへ`unsafe_code = "forbid"`を
    // 置いたのと同じ考え方で、人の注意に依存させない。
    //
    // **このguardが効くのは`IDF_PATH`が実在するESP-IDFを指す場合である**（#102で実測）。
    // 依存の`esp-idf-sys`のbuild scriptが先に走り、`IDF_PATH`を検出すると
    // `ESP_IDF_VERSION`のpinを破棄する。指す先が実在しなければ`esp-idf-sys`が
    // 先に失敗するため、ここへは到達しない。**そちらはbuildが止まるので安全側であり、
    // 差は診断messageの分かりやすさだけである。**実害があるのは「実在する別のESP-IDFで
    // 黙ってbuildが通る」経路であり、それはこのguardが止める。
    // 依存のbuild scriptの実行順はCargoが決めるため、ここでは変えられない。
    let allow_external = "DESKCAT_ALLOW_EXTERNAL_IDF_PATH";

    // 環境変数の変化でbuild scriptを再実行させる。これが無いと、`IDF_PATH`を設定した
    // 後の再buildでcacheが使われ、guardが評価されない。
    println!("cargo:rerun-if-env-changed=IDF_PATH");
    println!("cargo:rerun-if-env-changed={allow_external}");

    // 空文字は未設定として扱う。ただし**この分岐へ実buildで到達しない**（#102で実測）。
    // `export IDF_PATH=`だけが走った端末では、ここより先に`esp-idf-sys`が空文字を
    // 「空pathのcustom repository」（`idf_path: Some("")`）と解釈して失敗する。
    // どのみちbuildできないため、この`filter`はbuild script単体での意味づけに留まる。
    // 外さないのは、guard単体の意味（未設定と空文字を同じに扱う）を保つためである。
    let external = std::env::var("IDF_PATH")
        .ok()
        .filter(|p| !p.trim().is_empty());

    if let Some(path) = external {
        // 値が`1`のときだけ通す。存在だけを見ると`=0`や`=false`でも通ってしまい、
        // 「無効にしたつもり」の設定が外部SDKでのbuildを許してしまう。
        let allowed = std::env::var(allow_external).is_ok_and(|v| v == "1");
        assert!(
            allowed,
            "IDF_PATH is set to {path}. It overrides the ESP_IDF_VERSION pinned in .cargo/config.toml, so this build would not match the recorded toolchain. Unset IDF_PATH, or set {allow_external}=1 (exactly \"1\") and record the override in a Version Record."
        );
        // 意図した外部SDKでも黙って通さない。build logへ残し、Version Recordの
        // `IDF_PATH present`へ転記できるようにする。
        println!("cargo:warning=IDF_PATH={path} overrides the pinned ESP-IDF. Record this override in the Version Record.");
    }

    embuild::espidf::sysenv::output();
}
