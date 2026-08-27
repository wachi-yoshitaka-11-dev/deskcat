# Version Record: Raspberry Pi Direct Build (実機 Raspberry Pi Zero W)

様式は [Version Record Template](../version-record-template.md) に従う。

- Record ID: `2026-08-17-pi-direct-build-native`
- 判定: `Partial`
- 初回検証日: 2026-08-17
- 最終有効な検証日時: 2026-08-26（[Issue #11](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/11)
  の前半。依存を持つ crate の build、lint、test。下の[2026-08-26 再検証](#2026-08-26-再検証issue-11-の前半依存を持つ-crate-の-build-と-test)）

**Raspberry Pi profile の version record は、これが初めてである。**既存 6 記録はいずれも
x86_64 の開発端末または CI の記録であり、Raspberry Pi 実機の記録は存在しなかった。
本作業の前は [Raspberry Pi Rust Toolchain](../raspberry-pi-rust-toolchain.md) が
「調査済み。Raspberry Pi実機は未検証」で、候補 target
`arm-unknown-linux-gnueabihf` の状態も `ABI 確認待ち` だった。
**同文書の状態欄は、本記録を根拠に同じ Pull Request で更新している。**

**この記録は開発端末の記録を置き換えない。**別端末・別 profile であり、
[README](README.md) の「一つの記録は、一台の端末と一つの profile に対応させる」に従う。

**この記録は SSH 経由で取得した。**対象機の shell へ到達する手段として SSH を使った。
実行場所は Raspberry Pi 実機であり、`Container / VM / native:` は `native（実機）` である。
Pi の shell 内で実行した command だけを記録の根拠にしている。
**access method としての SSH は、profile 文書と runbook のいずれにも記述が無い。**
詳細は[補足](#ssh-を-access-method-として使ったことの位置づけ)に書く。

## 記録

```text
Record ID: 2026-08-17-pi-direct-build-native
Date: 2026-08-17
Machine profile: Raspberry Pi Direct Build
Operator role: 開発者（human）の監督下でのAI agent作業。SSH経由で実行した。
  rustupの導入とrebootはhumanの確認を得た
Repository commit: 3a34aeffe6f441738cba14f9a4ef8ac9616cf15e
  （**測定を行った時点の commit である。**本記録を載せた branch はその後
  develop の更新へ追従して rebase したが、**測定はやり直していない。**
  Pi 上の作業はこのrepositoryのcrateを一切使わないため、rebase は測定値に影響しない）
Working tree clean: no（本記録の追加分を含む）

OS name: Raspbian GNU/Linux
OS version: 13 (trixie)。DEBIAN_VERSION_FULL=13.4
Kernel: 6.18.34+rpt-rpi-v6（Raspbian 1:6.18.34-1+rpt1、2026-06-09）
CPU architecture: armv6l
Userspace bitness: 32-bit（getconf LONG_BIT が 32）
Container / VM / native: native（実機）。systemd-detect-virt: none。
  containerでもVMでもない

Rustup version: rustup 1.29.0 (28d1352db 2026-03-05)
Rust channel: stable（Pi 上に rust-toolchain.toml は置いていない）
Rust compiler version: rustc 1.97.1 (8bab26f4f 2026-07-14)。LLVM version 22.1.6
Rust host: arm-unknown-linux-gnueabihf
Installed Rust targets: arm-unknown-linux-gnueabihf
Cargo version: cargo 1.97.1 (c980f4866 2026-06-30)
rustfmt version: rustfmt 1.9.0-stable (8bab26f4f6 2026-07-14)
Clippy version: clippy 0.1.97 (8bab26f4f6 2026-07-14)
Linker identity and version: cc (Raspbian 14.2.0-19+rpi1) 14.2.0。
  GNU ld (GNU Binutils for Raspbian) 2.44

Raspberry Pi only:
  Board model: Raspberry Pi Zero W Rev 1.1
    （/sys/firmware/devicetree/base/model）。Hardware は BCM2835
  Board revision: 9000c1（/proc/cpuinfo の Revision）。
    現物裏面 silkscreen は Raspberry Pi Zero W V1.1
  Kernel architecture: armv6l（uname -m）
  libc identity and version: Debian GLIBC 2.41-12+rpt1+deb13u3。
    getconf GNU_LIBC_VERSION は glibc 2.41
  Available memory before build: MemTotal 437156 kB（約 426 MiB）のうち
    available 約 326 MiB。zram swap（/dev/zram0）436220 kB が既に有効
  Available storage before build: /dev/mmcblk0p2（約 29 GiB）の
    available 26800272 kB（約 25.6 GiB）。これは toolchain 導入前の値

Commands run:
  # 対象機の確定（install より前）
  tr -d '\0' < /sys/firmware/devicetree/base/model
  # 1節 環境記録
  uname -a
  uname -m
  uname -r
  getconf LONG_BIT
  cat /etc/os-release
  ldd --version
  getconf GNU_LIBC_VERSION
  free -h
  df -h
  systemd-detect-virt
  # float ABI 判定（手段A・B・Cすべて）
  dpkg --print-architecture
  dpkg --print-foreign-architectures
  ls -l /lib/ld-linux-armhf.so.3 /lib/ld-linux.so.3
  readelf -h /bin/sh
  readelf -A /bin/sh
  readelf -l /bin/sh
  file -L /bin/sh
  od -An -tx1 -j 36 -N 4 /bin/sh
  # toolchain 導入
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs -o /tmp/rustup-init.sh
  sh /tmp/rustup-init.sh -y --no-modify-path --profile minimal --default-toolchain stable
  # **`--no-modify-path` で導入したため、PATH を通さないと以降の rustup と cargo は動かない。**
  # shell の起動 file を書き換えていないので、以降の各 session と各 script の先頭で
  # 都度これを実行した（下の `rustup component add` 以降のすべての command が対象）。
  export PATH="$HOME/.cargo/bin:$PATH"
  rustup component add rustfmt clippy
  # 最小 program（このrepositoryのcrateは使わない）
  export PATH="$HOME/.cargo/bin:$PATH"   # 別 session のため再度実行した
  WORK="$HOME/deskcat-issue8"
  cargo new "$WORK/hello" --name hello
  cd "$WORK/hello"
  # build 計測。実行した順序と内容を再現した形で示す（原文は関数へ切り出していた）。
  # debug の 3 回を先に回し、続けて release の 3 回を回した。時間は
  # date +%s.%N の差分を awk で算出した（この機体に bc は無い）。
  for FLAG in "" "--release"; do            # debug の 3 回 → release の 3 回
    for i in 1 2 3; do
      cargo clean >/dev/null 2>&1; sync
      t0=$(date +%s.%N); cargo build $FLAG > /tmp/b.log  2>&1; rc=$?; t1=$(date +%s.%N)
      awk -v a="$t0" -v b="$t1" 'BEGIN{printf "clean  %.2f s", b-a}'; echo " (rc=$rc)"
      [ $rc -ne 0 ] && tail -5 /tmp/b.log
      t0=$(date +%s.%N); cargo build $FLAG > /tmp/b2.log 2>&1; rc=$?; t1=$(date +%s.%N)
      awk -v a="$t0" -v b="$t1" 'BEGIN{printf "cached %.2f s", b-a}'; echo " (rc=$rc)"
    done
  done
  # **この loop の限界を 2 点記録する。**どちらも実行時の実態である。
  # (1) rc を出力するが、失敗しても loop を止めない（fail closed ではない）。
  #     今回は clean・cached とも全 12 回が rc=0 だったため、失敗した回が
  #     計測値へ混ざる余地は無かった。**再実行するなら rc!=0 で停止させるべきである。**
  # (2) loop 内で生成物を hash していない。hash は loop の外で 3 回取った
  #     （下記 `Generated artifact identity` に取得点を書いた）。
  # peak memory は上記とは別に 1 回ずつ測った。/proc を 0.2 秒間隔で読む sampler を
  # build と並走させる方式で、内容は `Peak memory if measured` 欄に書いた。
  # 生成物の同一性
  file target/debug/hello
  readelf -h target/debug/hello
  readelf -A target/debug/hello
  ldd target/debug/hello
  sha256sum target/debug/hello target/release/hello
  ./target/debug/hello; ./target/release/hello
  # 4節の reboot 後再実行（session が切れる前に応答を返すため timer 経由で起動した）
  sudo -n systemd-run --on-active=2 --timer-property=AccuracySec=100ms /sbin/reboot
  uptime -s; uname -r                  # reboot したことと kernel が同一であることの確認
  sha256sum target/debug/hello target/release/hello  # reboot 後（再 build しない）
  ./target/debug/hello; ./target/release/hello       # reboot 後

Expected result: 候補 target の適用条件が実機で満たされ、最小 Rust program が
  Pi 上で build でき、同じ Pi で実行でき、reboot 後も実行できる。

Actual result:
  すべて成功した。内訳は次のとおり。

  対象機の確定  /sys/firmware/devicetree/base/model が
                `Raspberry Pi Zero W Rev 1.1`。silkscreen の V1.1 と一致

  float ABI     hard-float。手段6つすべてが一致した（詳細は下の表）
  rustup host   arm-unknown-linux-gnueabihf（rustup が推論した default host triple）
  最小 program  cargo new の hello world。依存 0 件。edition は cargo の既定で 2024
  build         clean / cached とも debug・release 全 6 回が rc=0
  実行          Hello, world!（debug rc=0、release rc=0）
  reboot 後     再 build せずに両 binary が実行でき、sha256 も一致した
                （uptime -s が 2026-08-17 23:03:56 に更新されたことで reboot を確認）

  float ABI 判定の実出力（手段と結果）
  ┌────────────────────────────────┬──────────────────────────────────────┬──────┐
  │ 手段                           │ 出力                                 │ 判定 │
  ├────────────────────────────────┼──────────────────────────────────────┼──────┤
  │ A dpkg --print-architecture    │ armhf                                │ hard │
  │ A ls -l /lib/ld-linux*         │ ld-linux-armhf.so.3 あり             │ hard │
  │                                │ ld-linux.so.3 は No such file        │      │
  │ B readelf -h の Flags:         │ 0x5000400, Version5 EABI,            │ hard │
  │                                │ hard-float ABI                       │      │
  │ B readelf -A Tag_ABI_VFP_args  │ VFP registers                        │ hard │
  │ B interpreter (readelf -l,     │ /lib/ld-linux-armhf.so.3             │ hard │
  │   file -L)                     │                                      │      │
  │ C od -An -tx1 -j 36 -N 4       │ 00 04 00 05（2 byte 目が 04）        │ hard │
  └────────────────────────────────┴──────────────────────────────────────┴──────┘
  **手段 C を実行したのは runbook の手順からの意図的な逸脱である。**runbook は
  「C を使うのは B を実行できないときだけである」と定めるが、この機体では
  `readelf` が最初から使えたため B を実行できた。**それでも C を追加の
  cross-check として実行した。**理由は、#8 が判定手順そのものの判別可能性を
  確かめる Issue であり、C（`od` による `e_flags` の直読み）が B と同じ結論を
  出すかを実機で確認する価値があったためである。
  **したがって「6 手段が一致」は、runbook が要求する手段より多く実行した結果である。**
  runbook が要求する範囲（A と B）だけでも hard-float と判定できた。
  **runbook の条件文を書き換えていない。**C を常時実行してよいかは
  手順の変更にあたるため、人間の判断に委ねる。

  検体は /bin/sh（dash への symlink。file には -L を付けた）。
  dpkg --print-foreign-architectures は arm64 を返したが、native architecture は
  armhf であり、32-bit loader は armhf 版だけが存在するため多重解釈は生じない。
  **Tag_ABI_VFP_args は判定表が hard と定める `VFP registers` そのものであり、
  「判定不能」に当たる compatible / toolchain-specific / 行なしのいずれでもない。**

  Tag_CPU_arch: v6（/bin/sh、および生成した hello の両方）
  Tag_FP_arch: VFPv2

  候補 target の適用条件（raspberry-pi-rust-toolchain.md の 8 条件）
  1 board が Zero W          満たす（silkscreen と devicetree model）
  2 userspace が 32-bit      満たす（getconf LONG_BIT = 32）
  3 hard-float               満たす（上記 6 手段が一致）
  4 物理 CPU が Armv6        満たす（board model と BCM2835 の公式情報。
                             /proc/cpuinfo の CPU part 0xb76 = ARM1176JZF-S）
  5 ELF が Armv6 超を要求しない 満たす（Tag_CPU_arch: v6）
  6 glibc 2.17 以上          満たす（2.41）
  7 kernel 3.2 以上          満たす（6.18.34）
  8 rustc host が一致        満たす（arm-unknown-linux-gnueabihf）

Build duration:
  toolchain 導入
    rustup（minimal、stable）        247 秒
    rustup component add rustfmt clippy  24 秒
    合計                             271 秒

  最小 program（cargo clean 後の clean build と、その直後の cache 有り再build。
  各 3 回。sync を挟み、1 コアの機体で逐次実行した）
  ┌─────────┬──────────────────────────┬────────────────────────┐
  │ profile │ clean build              │ cache 有り再build      │
  ├─────────┼──────────────────────────┼────────────────────────┤
  │ debug   │ 11.39 / 4.35 / 5.37 秒   │ 0.48 / 0.50 / 0.47 秒  │
  │ release │  3.96 / 3.52 / 3.50 秒   │ 0.50 / 0.50 / 0.49 秒  │
  └─────────┴──────────────────────────┴────────────────────────┘
  **1 回目の debug 11.39 秒だけが外れ値である。**toolchain 導入直後で
  page cache が温まっていない状態の測定であり、2 回目以降は 4〜5 秒台に収まる。
  この差を機体の性能変動と読まない。
  peak memory 測定時の追加測定は debug 6.09 秒、release 4.96 秒（sampling の負荷を含む）。

Peak memory if measured: 測定した。ただし sampling による近似値である。
  GNU time（/usr/bin/time）が未導入で、この Issue の範囲で新規 package を
  増やさない判断をしたため、/proc を 0.2 秒間隔で読む方式で代替した。
  ┌──────────────────────────────┬─────────────┬─────────────┐
  │ 指標                         │ clean debug │ clean release│
  ├──────────────────────────────┼─────────────┼─────────────┤
  │ sample 数                    │ 15          │ 12          │
  │ baseline（MemTotal-MemAvail）│ 108280 kB   │ 109080 kB   │
  │ peak（MemTotal-MemAvail）    │ 125408 kB   │ 119644 kB   │
  │ build 起因の増分             │ 17128 kB    │ 10564 kB    │
  │ peak Σ RSS（cargo + rustc）  │ 149092 kB   │ 134432 kB   │
  │ peak swap used               │ 9128 kB     │ 9128 kB     │
  └──────────────────────────────┴─────────────┴─────────────┘
  **Σ RSS が system 全体の増分より大きいのは二重計上のためである。**rustc の RSS には
  共有 file-backed page が含まれる。両者を足したり比べたりしない。
  **build 起因の swap 増加は観測していない。**build 中の peak が 9128 kB であるのに対し、
  この測定より前の時点で既に 9888 kB が使われていた（build 前の観測値）。
  **つまり build 中に swap 使用量が既存の水準を上回っていない。**reboot 後は 0 である。
  **なお「swap を使っていない」ことの証明ではない。**0.2 秒間隔の sampling であり、
  /proc/swaps の `Used` は瞬間値である。
  **0.2 秒間隔の sampling であり、これより短い spike は取り落としうる。**

Storage delta if measured:
  toolchain 導入   839420 kB 増（約 820 MiB）
    うち rustup minimal    807064 kB（2144504 → 2951568 kB used）
    うち rustfmt + clippy   32356 kB（2951568 → 2983924 kB used）
    導入後の内訳  ~/.rustup 823656 kB、~/.cargo 15760 kB（合計 839416 kB。
      df の差分 839420 kB との 4 kB 差は block 単位の丸めである）
    **delta の baseline は導入 script 自身が導入直前に採った used 2144504 kB である。**
      上の `Available storage before build` の 26800272 kB は、その数分前に
      前提確認で採った別の snapshot（used 2144488 kB）であり、16 kB ずれる。
      **同一時点の値ではないため、両者を足し引きしない。**
  最小 program の target/   4708 kB（debug と release の両方を含む）
  build 後の available     25955984 kB（約 24.8 GiB）。reboot 後は 25955916 kB。
    空き容量に問題は無い

Generated artifact identity:
  target/debug/hello    4155324 byte
    sha256 41afa75673326a20cd9a6b24c2823f492839b367ac8971e275c12edc44aa36eb
    ELF 32-bit LSB pie executable, ARM, EABI5 version 1 (SYSV), dynamically linked,
    interpreter /lib/ld-linux-armhf.so.3, for GNU/Linux 3.2.0, with debug_info
    Flags: 0x5000400, Version5 EABI, hard-float ABI / Tag_CPU_arch: v6
  target/release/hello   523204 byte
    sha256 d1b064383f8d38ae74dde3046c68a8bb8ab039231b9c0f79c1de18424e648213
    Flags: 0x5000400, Version5 EABI, hard-float ABI
  dynamic 依存（debug、ldd）
    /usr/lib/arm-linux-gnueabihf/libarmmem-v6l.so
    libgcc_s.so.1、libc.so.6、/lib/ld-linux-armhf.so.3
  **生成物の float ABI は system と一致する。**cross build ではなく native build
  なので build host の library へ誤って link する余地は無い。
  **同じ sha256 が別々の clean build で再現した。**取得点は次の 3 つで、
  いずれも `cargo clean` を挟んだ独立した build の生成物である。
    1 回目  build 計測 loop の最後の回（debug・release とも）
    2 回目  peak memory 測定のために測り直した clean build（同上）
    3 回目  reboot 後（**再 build せず、2 回目の生成物をそのまま hash した**）
  1 回目と 2 回目が bit 単位で一致し、3 回目で reboot をまたいでも同一であることを確認した。
  **loop 内で毎回 hash したわけではない。**上の `Commands run` の限界 (2) を参照する。

Log or evidence path: この記録本文

Known differences from documented profile:
  - この端末は導入時点で Rust を持っていなかった。**事前に rustup も distribution
    package の rustc も入っていない。**本 Issue の作業で human の確認を得て
    rustup 経由の stable を導入した（AGENTS.md「ツール導入は、対象 Issue、端末 profile、
    人間の確認が揃った開発端末だけで行う」）。
  - **install log に現れる rustup の warning を、既存 rustup の痕跡と読まないこと。**
    `It looks like you have an existing rustup settings file` が出るが、この
    `~/.rustup/settings.toml` は**同じ installer 自身がこの実行中に作った file** である
    （timestamp が install 開始時刻と一致する）。導入前に `command -v rustup` は
    NOT FOUND、`apt` の rustc は `Installed: (none)` であった。
  - **distribution package の rustc は採用しなかった。**apt の候補版が
    1.85.0+dfsg3-1+rpi1 であり、root Cargo.toml の rust-version = "1.97" を満たさない。
    rustup の stable は 1.97.1 でこれを満たす。
  - **rustup は --no-modify-path で導入した。**shell の起動 file を書き換えていないため、
    login shell では rustc が PATH に無い。使うには
    `. "$HOME/.cargo/env"` か PATH への追加が必要である。
    既存のユーザー設定を変更しない判断であり、恒久化するかは human の判断に委ねる。
  - **profile は minimal を選び、rustfmt と clippy だけを後から足した。**
    rust-docs は入れていない。1 コアの機体で download と展開の時間を減らすためである。
  - **git は導入していない。**この Issue の範囲は最小 Rust program であり、
    repository を clone しないため不要である。runbook 2 節は前提に Git を挙げているが、
    Raspberry Pi Runtime profile として配布物を扱う段で必要になる。
  - 2 節のその他の前提は導入前から満たしていた（build-essential 12.12、
    gcc 4:14.2.0-1+rpi1、binutils 2.44、libc6-dev:armhf 2.41-12+rpt1+deb13u3、
    curl 8.14.1-2+deb13u3、ca-certificates 20250419）。**readelf が最初から
    使えたため、ABI 判定の手段 B を代替せずに実行できた。**
  - swap は OS 既定の zram（/dev/zram0、436220 kB）である。**この作業で swap 設定を
    変更していない。**
  - **/proc/cpuinfo の `CPU architecture: 7` を Armv7 と読まないこと。**同 file の
    `CPU part: 0xb76` は ARM1176JZF-S であり Armv6 である。物理 CPU の根拠は
    board model と SoC の公式情報に置く（raspberry-pi-rust-toolchain.md の条件 4）。
    この機体の実出力にこの罠が現れたため記録する。

Conclusion: Partial。**この記録の対象は、この 1 台・Raspberry Pi Direct Build profile に
  限る。**その範囲では profile の必須項目（Rust stable、Cargo、native linker、空き容量）が
  すべて成功し、最小 Rust program の build・実行・reboot 後の再実行まで確認できた。
  候補 target `arm-unknown-linux-gnueabihf` は 8 条件すべてを実機で満たし、確定した。
  **`ABI 確認待ち` は解消した。**判定手順そのものも実機で判別可能であることを確認した
  （6 手段が一致し、「判定不能」に落ちなかった）。

  **`Verified` にしないのは、次が未実行のためである。**
  - **このrepositoryのcrateとworkspaceを Pi 上で build していない。**#8 の範囲は
    最小 program であり、意図して実行していない。したがって
    **host workspace の検証済み command（cargo fmt / clippy / test）が Pi で通るかは
    不明である。**この記録を根拠にしない。
  - **依存を持つ build を測っていない。**計測した最小 program の依存は 0 件である。
    cross compilation へ移る条件のうち「dependency build が memory 不足で安定しない」は、
    **この記録では評価できない。**426 MiB の機体で依存を伴う build が通るかは別問題である。
  - **/dev/ttyUSB* の device 名を確認していない。**USB OTG 変換 cable が未購入で
    ESP32 と接続できない。#8 の受け入れ条件にも入っていない。
  - **flash、serial monitor、ESP32 との通信は一切行っていない。**この記録は
    Raspberry Pi Direct Build profile だけを対象とする。

  **cross compilation は保留を維持する。**raspberry-pi-rust-toolchain.md の
  「Cross compilationへ移る条件」4 つのうち、**評価できたのは 3 つで、いずれも
  当たらなかった。残る 1 つは未評価である。**
  - clean build が許容できない → 評価した。当たらない。debug 4〜5 秒台、release 3.5 秒台
  - dependency build の memory 不足 → **未評価**（依存 0 件のため。上記のとおり）。
    **「当たらない」とは言えない。**
  - storage 消費や書込み負荷 → 評価した。当たらない。toolchain 820 MiB、
    target 4.6 MiB、空き 24.8 GiB
  - 複数 Pi への配布の自動化 → 評価した。当たらない。現時点で対象は 1 台

  **したがって保留の維持は、4 条件すべてを否定した結論ではない。**評価できた 3 条件が
  当たらず、残る 1 条件が未評価であるため、**現時点で移行する根拠が無い**という判断である。

Next action:
  - Raspberry Pi Runtime profile（deskcatd の実行）の記録は別途必要である（本記録の対象外）。
  - **依存を持つ crate の build を Pi 上で測ることは、この記録の範囲外である。**
    cross compilation の判断を確定させるにはその測定が必要になる。
  - PATH 設定を恒久化するか（`. "$HOME/.cargo/env"` の .bashrc 追記など）は human の判断。
  - /dev/ttyUSB* の device 名の確認は USB OTG 変換 cable の入手後に行う。
```

## 2026-08-26 再検証（Issue #11 の前半。依存を持つ crate の build と test）

同じ端末、同じ profile である。**環境の値は上の記録から変わっていない。**
OS、kernel、board、libc、rustup、rustc、cargo、rustfmt、clippy、linker のいずれも同一である。
**上の記録が「未実行」として残した項目のうち 2 つに答えるために行った測定だけを書く。**

上の `Conclusion` が `Verified` にしない理由として挙げていたもののうち、この節が扱うのは次の 2 つである。

- **このrepositoryのcrateとworkspaceを Pi 上で build していない**
  → **crate は build した。workspace は build していない。**したがって**この項目は半分しか埋まっていない。**
- **依存を持つ build を測っていない**（測定した最小 program の依存は 0 件だった）
  → **埋めた。**

**残りは埋まっていない。**何が残るかは末尾の[この再検証が主張しないこと](#この再検証が主張しないこと)に書く。

### 測定をやり直したことの記録

**1 回目の測定は完走していない。**外部 crate 14 件を通過し、`deskcat-protocol` を compile 中に、
**Pi の電源が落ちて中断した。**原因はこの profile の測定ではなく、同じ作業の別手順で
ESP32 を Pi の USB OTG port へ接続したことである。**接続した瞬間に Pi が network から消えた。**

**中断した測定の値は採らない。**`target/` を削除したうえで測定をやり直した。
**この節に載せる値は、すべてやり直した側の 1 回の連続実行から採っている。**

file system は再起動時の journal 復旧（`EXT4-fs: orphan cleanup`）だけで済み、
**破損は無かった**（`EXT4-fs error` と `I/O error` はいずれも 0 件、書き込み test も成功）。
再起動後の連続稼働で異常は出ていない。

**電源が落ちた原因はこの記録では切り分けていない。**低電圧 flag（`vcgencmd get_throttled`）は
再起動で消えるため事後に確認できず、電圧も測っていない。**この記録は電源経路について何も主張しない。**
扱いは [Issue #11](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/11) と
[power-budget.md](../../hardware/power-budget.md) の側に残す。

### 記録

```text
Record ID: 2026-08-17-pi-direct-build-native（2026-08-26 再検証）
Date: 2026-08-26（JST。測定は 23:03–23:43）
Machine profile: Raspberry Pi Direct Build（上の記録と同一）
Operator role: 開発者（human）の監督下でのAI agent作業。SSH経由で実行した
Repository commit: 4a91ee18fa41ce0398ca0e710fb586cfde0c590c
  （**測定を行った時点の commit である。**本記録を載せた branch はその後 develop の
  更新へ追従したが、**測定はやり直していない。**追従で入った差分は文書と script であり、
  測定対象の `crates/deskcat-serial` と `crates/deskcat-protocol` と `Cargo.lock` を
  変更していない。上の 2026-08-17 の記録と同じ扱いである）
Working tree clean: yes（Pi 側は git checkout ではない。開発端末から
  `git archive` で同 commit の tree を転送した。**Pi へ git を導入していない**）

環境: 上の記録から変更なし。再確認した値だけを挙げる。
  rustc 1.97.1 (8bab26f4f 2026-07-14) / cargo 1.97.1 (c980f4866 2026-06-30)
  rustfmt 1.9.0-stable (8bab26f4f6 2026-07-14) / clippy 0.1.97 (8bab26f4f6 2026-07-14)
  nproc: 1
  MemTotal: 437156 kB（約 426.9 MiB）
  swap: zram（/dev/zram0）。**この作業で swap 設定を変更していない**

Available memory before build: MemTotal 437156 kB のうち MemAvailable 約 321888 kB
  （約 314 MiB）。sampler の baseline（MemTotal-MemAvailable）は 115268 kB、swap used 0
Available storage before build: /dev/mmcblk0p2 の used 3028124 kB、available 25916636 kB
  （約 24.7 GiB）。~/.cargo は 56340 kB

Commands run:
  . "$HOME/.cargo/env"
  cd "$HOME/deskcat"
  rm -rf target                                              # clean 状態を作る
  cargo build  --locked -p deskcat-serial                    # P1 clean build
  cargo build  --locked -p deskcat-serial                    # P2 cache 有り（no-op）
  cargo fmt --all -- --check                                 # P3
  cargo clippy --locked -p deskcat-serial --all-targets      # P4
  cargo test   --locked -p deskcat-serial                    # P5
  cargo test   --locked -p deskcat-protocol                  # P6
  cargo build  --locked -p deskcat-serial --example serial_link   # P7（buildのみ。実行しない）

  **`cargo test --workspace` を実行していない。**426 MiB の機体で workspace 全体を
  一度に回す前に 1 crate ずつ確認する方針を採り、`-p` で絞った。
  **registry cache は測定開始前から埋まっている。**1 回目（中断した回）の
  `cargo fetch --locked` が 20 秒で 17 crate を download 済みであり、
  ~/.cargo は 15816 kB から 56340 kB へ増えていた（+40524 kB、約 39.6 MiB）。
  **したがって下の P1 は compile だけの時間であり、download を含まない。**

Expected result: 依存を持つこのrepositoryの crate が Pi 上で build でき、
  lint と test が通り、426 MiB の機体で OOM せずに完走する。

Actual result: **全 phase が rc=0 で成功した。OOM は発生していない。**

  ┌─────┬──────────────────────────────────────────────┬──────┬───────────┐
  │ 相  │ 内容                                         │ rc   │ 所要      │
  ├─────┼──────────────────────────────────────────────┼──────┼───────────┤
  │ P1  │ clean build（-p deskcat-serial）             │ 0    │ 1344 秒   │
  │ P2  │ cache 有り再build（no-op）                   │ 0    │    3 秒   │
  │ P3  │ cargo fmt --all -- --check                   │ 0    │    5 秒   │
  │ P4  │ clippy（-p、--all-targets）                  │ 0    │  569 秒   │
  │ P5  │ test -p deskcat-serial                       │ 0    │  190 秒   │
  │ P6  │ test -p deskcat-protocol                     │ 0    │  339 秒   │
  │ P7  │ example serial_link の build                 │ 0    │    3 秒   │
  └─────┴──────────────────────────────────────────────┴──────┴───────────┘

  P1 の 1344 秒は 22 分 24 秒である。**`deskcat-serial` 自身を含めて 17 unit を compile した。**
  内訳は依存 16 件と対象 crate 1 件である。**依存 16 件のうち 15 件が crates.io の外部 crate**
  （proc-macro2、unicode-ident、quote、serde_core、zmij、syn、serde_json、libc、serde、
  serde_derive、itoa、memchr、cfg-if、serial2、log）**であり、残る 1 件は path dependency の
  `deskcat-protocol` である。**
  **download した 17 crate のうち 2 件（windows-link、windows-sys）は compile されていない。**
  Windows 専用であり、Linux では `cfg` で外れる。**`cargo fetch` の件数と compile 件数を
  同じ数として扱わない。**

  test の内訳（合計 138 tests passed、0 failed、0 ignored）
    deskcat-serial   unittests src/lib.rs      24 passed
                     tests/simulator.rs        29 passed
                     Doc-tests                  2 passed
    deskcat-protocol unittests src/lib.rs      52 passed
                     tests/conformance.rs      11 passed
                     tests/error_codes.rs       3 passed
                     tests/framing.rs           5 passed
                     tests/limits.rs            9 passed
                     Doc-tests                  3 passed

  **警告は 0 件である。**root Cargo.toml の [workspace.lints] が
  `clippy::all = deny` と `unsafe_code = "forbid"` を課すため、
  警告が出ていれば P4 は rc!=0 で失敗する。**`-D warnings` を付けていないのは
  この理由による**（AGENTS.md の host workspace の検証 command と同じ扱い）。

Build duration: 上表のとおり。**clean build 1344 秒、cache 有り 3 秒。**
  **上の 2026-08-17 の記録（依存 0 件の最小 program で clean 4〜5 秒台）とは
  比較の対象が違う。**同じ「clean build」の語で並べない。

Peak memory if measured: 測定した。**sampling による近似値である。**
  GNU time（/usr/bin/time）はこの機体に無く、この作業で package を増やさない判断をしたため、
  /proc を 1 秒間隔で読む sampler を全 phase と並走させた（686 sample）。
  ┌────────────────────────────────────┬───────────────┐
  │ 指標                               │ 値            │
  ├────────────────────────────────────┼───────────────┤
  │ sample 数（1 秒間隔）              │ 686           │
  │ baseline（MemTotal-MemAvailable）  │ 115268 kB     │
  │ peak（MemTotal-MemAvailable）      │ 216332 kB     │
  │ build 起因の増分                   │ 101064 kB     │
  │ peak 単一 process RSS（rustc）     │ 247364 kB     │
  │ baseline swap used                 │      0 kB     │
  │ peak swap used                     │  53680 kB     │
  └────────────────────────────────────┴───────────────┘
  **peak 単一 process RSS 247364 kB は MemTotal 437156 kB の 56.6% である。**
  **これは「単一 process の最大 RSS」であり、process の合計ではない。**
  上の 2026-08-17 の記録が採った `Σ RSS（cargo + rustc）` とは指標が違うため、
  両者を直接比べない。
  **MemTotal-MemAvailable が単一 process の RSS より小さいのは二重計上ではなく、
  MemAvailable が回収可能な page cache を含むためである。**rustc の RSS には
  共有 file-backed page が含まれる。**両者を足したり引いたりしない。**
  **1 秒間隔の sampling であり、これより短い spike は取り落としうる。**
  **swap は baseline 0 kB から peak 53680 kB へ増えた。**上の 2026-08-17 の測定
  （build 起因の増加が観測されなかった）とは異なり、**今回は build 起因の swap 使用がある。**
  ただし zram（圧縮 RAM）であり、disk swap ではない。

Storage delta if measured:
  測定前  used 3028124 kB / available 25916636 kB。~/.cargo 56340 kB
  測定後  used 3265516 kB / available 25679244 kB。~/.cargo 56340 kB、target/ 237344 kB
  差分    used +237392 kB（約 232 MiB）。**ほぼ全量が target/ である**
  registry cache は測定中に増えていない（56340 kB で不変）。
  **1 回目の fetch による +40524 kB（約 39.6 MiB）は測定前に発生している。**
  空き容量は約 24.5 GiB。**問題は無い。**

Generated artifact identity: **この再検証では生成物の hash を採っていない。**
  上の 2026-08-17 の記録と違い、再現性の確認ではなく資源の測定を目的としたためである。
  library crate と test binary であり、実行して同一性を確認した binary は無い。

Log or evidence path: この記録本文

Known differences from documented profile:
  - **Pi 側は git checkout ではない。**開発端末から `git archive` で同 commit の tree を
    転送した。**Pi へ git を導入していない**（上の記録の「git は導入していない」を維持した）。
  - **`libudev` を導入していない。**`serial2` は `unix` feature で libc 経由の
    termios だけを使い、C の `libudev` を要求しない（`serialport` crate を採らなかった
    理由の 1 つである。`crates/deskcat-serial/Cargo.toml` の依存選定を参照）。
    **Pi 上で `pkg-config --modversion libudev` は「見つからない」を返すが、build は通る。**
  - **package を 1 つも新規導入していない。**測定に使った sampler は shell と
    /proc だけで書いた。
  - **release profile を測っていない。**debug のみである。

Conclusion: Partial。**この節の対象は、この 1 台・Raspberry Pi Direct Build profile に限る。**
  その範囲で、**依存を持つこのrepositoryの crate の build、format 検査、lint、test が
  すべて Pi 上で成功した。**

  **[Raspberry Pi Rust Toolchain](../raspberry-pi-rust-toolchain.md) の `確定条件` に
  未達で残っていた 2 項目のうち、埋まったのは 1 つと半分である。**
  - **`project の test command を記録した` は埋めた。**`cargo test --locked -p <crate>` が
    Pi 上で通る。138 tests passed、0 failed。所要は deskcat-serial 190 秒、
    deskcat-protocol 339 秒。**ただし `--workspace` は実行していない。**
  - **`direct build を継続するか cross compilation へ移るか決定した` は、この測定の時点では
    埋まっていない。**移行条件のうち memory は評価した（peak 単一 process RSS 247364 kB＝
    MemTotal の 56.6%、OOM なし）。**したがって「dependency build が memory 不足で安定しない」は
    当たらない。**残る `clean build が許容できない` は、この時点では数値を出しただけである。
    **（2026-08-27 追記: この実測値をもとに direct build の継続を決定し、同項目は埋まった。
    判断と、それを覆す条件は
    [Raspberry Pi Rust Toolchain](../raspberry-pi-rust-toolchain.md#cross-compilationへ移る条件)
    にある。**この記録の測定値は変更していない。**）

  **`Verified` にしないのは、次が未実行のためである。**
  - **`cargo test --workspace` と `cargo clippy --workspace` を実行していない。**
    1 crate ずつに絞った。**workspace 全体を一度に回したときに通るかは、この記録では言えない。**
  - **`serial_link` example を build しただけで、実行していない。**
  - **実 serial port を開いていない。`/dev/ttyUSB*` の device 名も確定していない。**
  - **reboot 後の再実行を今回は行っていない。**
  - **release profile を測っていない。**

Next action:
  - **clean build 1344 秒を「開発サイクルとして許容できるか」の判断は human に残る。**
    移行条件の 1 つ「clean build が許容できない」は、依存 0 件のときの 4〜5 秒台と
    22 分 24 秒とで見え方が変わる。**この記録は数値を出すところまでとし、可否を決めない。**
  - 実 serial port の確認（Issue #11 の後半）は、ESP32 との接続経路が
    段階 C の gate の内側にあるため、この記録の範囲外である。
  - Raspberry Pi Runtime profile（`deskcatd` の実行）の記録は別途必要である。
```

### この再検証が主張しないこと

- **workspace 全体の検証 command が Pi で通ることを主張しない。**`-p` で 1 crate ずつ回した。
- **実 serial port について何も主張しない。**port を開いていない。device 名も確定していない。
- **ESP32 との通信について何も主張しない。**`serial_link` は build しただけである。
- **電源経路について何も主張しない。**測定中に 1 度 Pi の電源が落ちたが、原因を切り分けていない。
- **この測定は cross compilation へ移るか否かを決めていない。**移行条件のうち memory の 1 つが
  「当たらない」と言えるようになっただけである。**判断そのものは 2026-08-27 に別途行った**
  （[Raspberry Pi Rust Toolchain](../raspberry-pi-rust-toolchain.md#cross-compilationへ移る条件)）。
  **この記録は測定の記録であり、判断の記録ではない。**
- **release profile と reboot 後の再実行を主張しない。**どちらも今回は行っていない。


## 補足

### 匿名化について

[Version Record Template](../version-record-template.md) の「記録してはいけないもの」に従い、
次を本記録から除いている。

- 端末名と username。`uname -a` の出力は hostname を含むため、この記録には採らず
  `uname -r`（kernel release）と `uname -m`（machine）に分けて記録した。
- IP address と MAC address。SSH の接続先と接続 command も記録しない。
- 個人の絶対 path。作業 directory は `<work>`、home directory は `$HOME` と表記した。
- Wi-Fi 情報。この作業では Wi-Fi の設定を読んでいない。
- 秘密鍵と password。**SSH 鍵は開発端末の `~/.ssh` 配下だけに置き、repository へ
  入れていない。**Pi 側では既存の `authorized_keys` に公開鍵を 1 行追加しただけであり、
  `sshd_config` は読んでいない（変更もしていない）。

`Board revision` の `9000c1` は Raspberry Pi の revision code であり、機体固有の serial
番号ではない。同一 model・同一 revision の機体で共通の値である。

### SSH を access method として使ったことの位置づけ

**profile 文書と runbook のどちらにも、対象機の shell へ到達する手段の記述が無い。**
[Machine Profiles](../machine-profiles.md) が `実機 Linux に限る` と `人間の監視` を課すのは
`ESP32 Flash / HIL` profile であり、`Raspberry Pi Direct Build` には課していない。
[ADR-0005](../../decisions/0005-standard-development-os.md) の container / VM に関する制限も
「Raspberry Pi 側の 2 profile は対象外」と定めている。
[runbook](../../runbooks/raspberry-pi-development-machine-setup.md) は開始条件に
「安定した電源と network がある」を挙げるが、shell への到達手段には触れていない。

したがって SSH の使用は、禁止規則に触れずに行えた一方で、**文書に根拠を持たない運用でもある。**
本記録では次の 2 点を守ることで、記録の位置づけを保った。

- **測定は Pi の shell 内で実行した。**開発端末側で得た値を Pi の値として扱っていない。
  `Container / VM / native:` は `native（実機）` である。
- **対象機の同一性を install より前に確認した。**`/sys/firmware/devicetree/base/model` が
  `Raspberry Pi Zero W Rev 1.1` を返すことを確認してから toolchain を導入した。
  [AI Agent Policy](../../governance/ai-agent-policy.md) の「正確なハードウェア対象を
  特定できない」場合の停止規則に対応する。

**現物裏面の silkscreen による確認は SSH では代替できない。**この点は human が実施済みで
（[hardware-bom.md](../../hardware/hardware-bom.md) SBC-01 の `Raspberry Pi Zero W V1.1`）、
本作業では devicetree の値と突き合わせた。

runbook 4 節の reboot 後再実行は、reboot によって SSH session が切れる。本作業では
再接続してから同じ binary を実行した。**runbook はこの session 再確立に触れていない。**

access method を文書へ明記するかどうかは human の判断に委ねる。
