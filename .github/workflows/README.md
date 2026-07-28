# Workflow

基盤整備段階では、実行可能なGitHub Actions workflowをcommitしない。

理由:

- Rust workspaceと検証済みcommandがまだ存在しない。
- ESP32 toolchain versionが固定されていない。
- 再現できないbuildを検証するとworkflowで主張してはならない。

次の順でworkflowを追加する。

1. root Cargo workspace作成後のhost format、lint、unit test
2. protocol fixture test
3. toolchain固定後のESP32 cross-build
4. toolとversion選定後の文書検査

Hardware-in-the-Loop testは通常のhosted CIから分離する。
