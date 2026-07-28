# Workflow

Rust／ESP-IDFのbuild commandが確定するまで、それらを実行するGitHub Actions workflowをcommitしない。

理由:

- Rust workspaceと検証済みcommandがまだ存在しない。
- ESP32 toolchain versionが固定されていない。
- 再現できないbuildを検証するとworkflowで主張してはならない。

次の順でworkflowを追加する。

1. root Cargo workspace作成後のhost format、lint、unit test
2. protocol fixture test
3. toolchain固定後のESP32 cross-build
4. toolとversion選定後の文書検査

## 現在のworkflow

`pages.yml`は[GH-003 #26](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/26)で追加する公開文書専用workflowである。

- Rust、ESP-IDF、firmware、実機をbuildしない。
- Pull RequestではPages sourceの生成、Jekyll build、link検査だけを行う。
- `main`で同じcheckに成功した場合だけPages artifactをdeployする。
- 通常権限はread-onlyとし、deploy jobだけに`pages: write`と`id-token: write`を付与する。
- Actionはreview済みのcommit SHAへ固定する。

Hardware-in-the-Loop testは通常のhosted CIから分離する。
