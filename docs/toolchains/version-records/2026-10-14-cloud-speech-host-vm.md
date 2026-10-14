# #522 Cloud Speech Host Rust 検証記録

```text
Record ID: 2026-10-14-cloud-speech-host-vm
Date: 2026-10-14
Machine profile: Host Rust Development
Operator role: 作業セッション
Repository commit: `origin/develop` `df6a6956bffa`からの未commit差分
Working tree clean: no

OS name: Ubuntu
OS version: 22.04.5 LTS
CPU architecture: x86_64
Userspace bitness: 64
Container / VM / native: VM（`systemd-detect-virt`: `microsoft`）

Rustup version: 1.29.0
Rust channel: stable
Rust compiler version: 1.97.1
Rust host: x86_64-unknown-linux-gnu
Installed Rust targets: x86_64-unknown-linux-gnu
Cargo version: 1.97.1
rustfmt version: 1.9.0-stable
Clippy version: 0.1.97
Linker identity and version: GNU ld 2.38
Host OpenSSL (pkg-config): 3.0.2

ESP32 only: 対象外。共通crateの変更なし
Raspberry Pi only: 対象外。Pi Zero WでのbuildとHTTPSは未実施

Commands run: `cargo fmt --all -- --check`、`cargo clippy --workspace --all-targets --locked`、`cargo test --workspace --locked`、`python3 scripts/validate_doc_links.py`
Expected result: format差分なし、lint・test・link検査成功
Actual result: 上記4 command成功。実APIへのrequestは行っていない。
Build duration: 未測定
Peak memory if measured: 未測定
Storage delta if measured: 未測定
Generated artifact identity: host test binaries（配布対象ではない）
Log or evidence path: この記録とPull Requestの検証欄
Known differences from documented profile: Pi Zero Wのarmv6、native OpenSSL環境ではない
Conclusion: Verified（Host Rust Developmentのformat・lint・unit/integration testのみ）
Next action: Pi Zero Wでの`deskcatd`直接build、systemd credentialとHTTPS接続、常駐時memory・時間の確認。Workspace月額上限と専用キーのscopeは人間が確認する。
```
