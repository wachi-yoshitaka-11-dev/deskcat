fn main() {
    // `IDF_PATH`が設定されていると、`.cargo/config.toml`が`force = true`で固定した
    // `ESP_IDF_VERSION`（v5.5.3）より優先され、Version Recordが識別していないESP-IDFで
    // buildされる。生成物は記録と対応しなくなるが、buildは成功するため気づけない。
    //
    // `[env]`ではこの変数を保護できない。managed installを前提としており`IDF_PATH`を
    // 固定値にできないためである。開発端末のsetup手順は「build前に毎回確認する」と
    // していたが、確認は付け忘れる。ここで止める。manifestへ`unsafe_code = "forbid"`を
    // 置いたのと同じ考え方で、人の注意に依存させない。
    let allow_external = "DESKCAT_ALLOW_EXTERNAL_IDF_PATH";

    // 環境変数の変化でbuild scriptを再実行させる。これが無いと、`IDF_PATH`を設定した
    // 後の再buildでcacheが使われ、guardが評価されない。
    println!("cargo:rerun-if-env-changed=IDF_PATH");
    println!("cargo:rerun-if-env-changed={allow_external}");

    // 空文字は未設定として扱う。shellの初期化で`export IDF_PATH=`だけが走る場合がある。
    let external = std::env::var("IDF_PATH").ok().filter(|p| !p.trim().is_empty());

    if let Some(path) = external {
        assert!(
            std::env::var_os(allow_external).is_some(),
            "IDF_PATH is set to {path}. It overrides the ESP_IDF_VERSION pinned in .cargo/config.toml, so this build would not match the recorded toolchain. Unset IDF_PATH, or set {allow_external}=1 and record the override in a Version Record."
        );
        // 意図した外部SDKでも黙って通さない。build logへ残し、Version Recordの
        // `IDF_PATH present`へ転記できるようにする。
        println!("cargo:warning=IDF_PATH={path} overrides the pinned ESP-IDF. Record this override in the Version Record.");
    }

    embuild::espidf::sysenv::output();
}
