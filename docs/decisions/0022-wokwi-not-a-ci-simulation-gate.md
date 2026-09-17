# ADR-0022: Wokwi を CI の simulation gate として採らない

> 状態: Accepted
> 日付: 2026-09-17
>
> 起案: AIが起案した（2026-09-17）。**[#325](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/325)の候補2として、人間が対象を指定している。**
> **選択肢と決定の文面はAIが起案した。人間の承認はPull Requestのmerge承認として得る**
> （[ADR-0017](0017-what-counts-as-a-primary-source.md)と同じ扱いである）。
>
> **`Accepted`は、根拠が揃ったという意味ではない。**#325の候補2が挙げた6項目のうち、
> **「実際に試して記録する」と「無償ライセンスを公式資料で確認する」の2つは、
> この作業では実行できていない**（判断要因5・6）。**決定はその2つに依存しない根拠
> （判断要因1・2・3）で立てている。**埋め方は「検証」節が持つ。

## 背景

[#325](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/325)の候補2は、Wokwi を**実機前の simulation gate** として使うかを問うている。
出所は Interface 2026年9月号であり、[ADR-0017](0017-what-counts-as-a-primary-source.md)の**丙**である。
**#325 本文が「候補として挙げるだけであり、採否は DeskCat 側の根拠に基づいて決める」と定めている。**
この ADR は誌面を根拠にしない。

現状は次のとおりである。

- firmware の生成条件が `wokwi=false` である。**記録の正本は[ESP32 Rust toolchain](../toolchains/esp32-rust-toolchain.md#生成条件の候補)の
  「生成条件の候補」表**であり、生成 command は[ESP32開発端末セットアップ](../runbooks/esp32-development-machine-setup.md)にある。
  そのため `firmware/esp32` に `wokwi.toml` と `diagram.json` は無い（2026-09-17に確認）
- `simulator/deskcat-sim/` は README だけであり、実装が無い

## 判断要因

### 1. Wokwi CI は API token を要する（公式資料で確認した）

[`wokwi-cli` の README](https://github.com/wokwi/wokwi-cli)は次を求めている（2026-09-17取得）。

> First, ensure that you set the `WOKWI_CLI_TOKEN` environment variable to your Wokwi API token.
> You can get your token from your Wokwi CI Dashboard.

**この link は revision を固定していない。**既定 branch の README であり、**内容は変わりうる。**
固定できない documentation には取得日を併記するという、[ESP32 Rust toolchain](../toolchains/esp32-rust-toolchain.md)の
既存の扱いに揃えている。

**token は account に紐づく credential である。**同 README の `## License` は
`The MIT License (MIT)` と書いているが、**それは CLI 自身の license である。
動かす先は Wokwi の service であり、CLI の license はその利用条件ではない。**

### 2. CI profile は「秘密情報を使わない build」を必須要件に置いている

[Machine Profiles](../toolchains/machine-profiles.md)の Profile一覧は、`CI` の必須要件を
**「固定 runner（`ubuntu-24.04`）、pin 済み action、秘密情報を使わない build」**と定めている。

**CI で Wokwi を回すには token を secret として置く必要があり、この要件と正面から衝突する。**

**この筋には、明示しておくべき前提が1つある。**`machine-profiles.md` は「秘密情報」を定義していない。
**「CI の secret store へ注入した API token は、同要件が言う『秘密情報』に当たる」と読んでいる。**
[AI Agent Policy](../governance/ai-agent-policy.md)が「資格情報または秘密鍵の取り扱い」を
人間の確認が要る操作へ挙げていることと整合する読みだが、**定義そのものは正本に無い。**
**この前提だけを否とするなら、判断要因2は崩れる。**そこだけを切り離して議論できるよう、ここへ書いておく。

**これは Wokwi 固有の欠点ではない。**CI へ credential を持ち込む形すべてに当たる要件である。
要件の側を変えるなら、それは Wokwi の採否とは別の判断になる。

### 3. simulation は実機試験の gate を開けない

`AGENTS.md`「検証」は**「実機試験が必要な変更を、PC テストだけで完了扱いにしない」**と定める。
**#325 本文自身も、採る場合の条件として「flash と実機試験の代替ではないことを検証節へ明記する」を挙げている。**

**この ADR は安全要件5項目の根拠水準に触れない。**要求する水準の正本は
[Hardware Safety Policy](../governance/hardware-safety-policy.md)の「5項目の中での根拠の水準」であり
（[ADR-0016](0016-evidence-bar-inside-the-safety-requirements.md)）、**ここへ再掲しない。**
同 policy は「**この節に「一律で一次資料または実測」とは書かない**」と明記している。

したがって Wokwi を入れても、**通さなければならない試験は1つも減らない。**
増えるのは「実機の前に気付ける可能性」だけである。**その価値を否定はしないが、gate ではない。**

### 4. artifact の形としては成立する見込みがある

[ESP32開発端末セットアップ](../runbooks/esp32-development-machine-setup.md)が記録済み commit として pin している
`esp-idf-template` の `08115a069d167a5ee37363e84f168a565f17bbca` は、`cargo/wokwi.toml` と
`cargo/diagram.json` を持つ。`wokwi.toml` の中身は次である（2026-09-17に同 commit の raw 内容を確認）。

```toml
[wokwi]
version = 1
gdbServerPort = 3333
elf = "target/{{ rust_target }}/debug/{{ project-name }}"
firmware = "target/{{ rust_target }}/debug/{{ project-name }}"
```

**`elf` と `firmware` の両方が、cargo が出力する ELF をそのまま指している。**
Rust＋ESP-IDF の build 生成物を別形式へ変換する段は、少なくともこの template には無い。
**この2 file が入るかどうかは、生成時の選択で決まる。**
同 commit の `cargo/cargo-generate.toml` が次を持つ（2026-09-17に raw 内容を確認）。

```toml
[conditional.'!wokwi']
ignore = [".vscode/launch.json", "diagram.json", "wokwi.toml"]
```

**条件は `wokwi` だけではない。**同 file の `[conditional.'!advanced']` の `ignore` にも
同じ2 file が入っている。**DeskCat は `advanced=true` を選んでいるため**
（[ESP32 Rust toolchain](../toolchains/esp32-rust-toolchain.md#生成条件の候補)）、
**この件では `wokwi` の選択だけが効いている。**

**つまり `wokwi=false` を選んだために除外された、というだけである。**`wokwi=true` を選べば、
上の ignore 配列の3 file（`wokwi.toml`／`diagram.json`／`.vscode/launch.json`）が生成物へ入っていた。

**この repository は既に同 template の Wokwi 定義を1つ読んでいる。**
[ESP32 Rust toolchain](../toolchains/esp32-rust-toolchain.md)は「同 template は MCU `esp32` の
Wokwi board を `board-esp32-devkit-c-v4` と定義しており、対象 board と MCU 選択の対応を裏づける」と
記録している（2026-08-06 取得）。

**確認済みなのは、template がその board 名を書き込むことだけである。**
**Wokwi の service 側に当該 board が実在するかは、この記録からは出ない。**判断要因6のとおり
`wokwi.com`／`docs.wokwi.com` を読めていない以上、**そちらは未確認である。**

**ただし、ここまではすべて file の中身から読んだ事実であって、動かして確かめた結果ではない。**

### 5. 実際に動かすことはできなかった

#325 の候補2は「**Rust＋ESP-IDF の artifact で成立するかを実際に試して記録する**」を求めている。
**この作業では実行できなかった。**理由は次の3つであり、いずれも実測である（2026-09-17）。

| 妨げているもの | 確かめ方と結果 |
|---|---|
| ESP-IDF toolchain が無い | 作業端末に `espup`／`idf.py`／`espflash` のいずれも無い（`which` で確認） |
| toolchain を入れられない | `dl.espressif.com` へ到達できない（`curl: (56) CONNECT tunnel failed, response 403`。作業環境の egress policy が proxy の CONNECT を拒否する） |
| Wokwi を呼べない | `wokwi.com` へ到達できない（同じ 403）。**token も持っていない** |

**「成立しない」ではない。「確かめていない」である。**この区別を崩さない。

### 6. 無償ライセンスの範囲で足りるかは確認できていない

#325 の候補2は「無償ライセンスの範囲で足りるかを公式資料で確認する」も求めている。
**`docs.wokwi.com` と `wokwi.com` がどちらも作業環境の egress policy で遮断されており、公式の利用条件を読めなかった**
（2026-09-17実測。いずれも `curl: (56) CONNECT tunnel failed, response 403`）。`wokwi-cli` の README には CI 利用の条件が書かれていない（MIT と書かれているのは CLI 自身の license である）。

**`AGENTS.md`の推測禁止に従い、「無償で足りる」とも「足りない」とも書かない。**

## 検討した選択肢

### 選択肢A: CI の simulation gate として採用する

push または Pull Request のたびに Wokwi で firmware を起動し、boot と初期化の失敗を実機の前に落とす。

**利点。**flash する前に、boot loop や panic を機械が見つける。実機を触れる人と時間に依存しない。

**コスト。**判断要因 1・2 のとおり、CI へ token を置くことになり、Machine Profiles の CI 要件と衝突する。
さらに判断要因 5・6 が未確認のまま残っており、**採用を決めても、動くかも、使ってよいかも確かめていない状態で入ることになる。**

### 選択肢B: gate にしない。local の任意利用は禁じない

`wokwi=false` を維持し、CI にも検証手順にも入れない。個人が自分の端末で使うことは妨げないが、
**その結果を検証の根拠として記録しない。**

**利点。**CI 要件と衝突しない。未確認のまま何かを採用することにならない。保守 cost が増えない。

**コスト。**flash 前に落とせたはずの失敗を、実機で踏む可能性が残る。
**ただし現時点で、実機試験がボトルネックになっているという観測は無い。**`firmware/esp32/src` はまだ4 file である（`main.rs`／`config.rs`／`health.rs`／`protocol.rs`。2026-09-17に数えた）。

### 選択肢C: 判断を保留する

判断要因 5・6 が埋まるまで決めない。

**利点。**根拠が揃ってから決められる。

**コスト。**#325 の候補2が開いたまま残る。**そして埋める条件は、この作業の側では作れない**
（別 profile の端末、Wokwi の account、egress policy の変更のいずれかが要る）。
**「誰かが条件を揃えるまで待つ」は、待つ主体が居ないと止まったままになる。**

## 決定

**選択肢Bを採る。Wokwi を CI の simulation gate としては採らない。**

- `firmware/esp32` の生成条件 `wokwi=false` を**変更しない**。
  **`wokwi=false` の記録は3箇所にある。**扱いを分ける。
  **(a) 生成条件の正本**（[ESP32 Rust toolchain](../toolchains/esp32-rust-toolchain.md#生成条件の候補)の
  「生成条件の候補」節）へは、**同節の「理由:」側**へこの ADR への link を足す。
  **据え置きが判断の結果であることを読めるようにするためである。**
  **同節の表にある `Wokwi` の行は変えない。**表は生成時に入力した値そのものの記録である。
  **(b) 生成 command の例**（[ESP32開発端末セットアップ](../runbooks/esp32-development-machine-setup.md)）は
  そのままにする。**(a) と同じことを2箇所へ書かない。**
  **(c) [2026-08-06 の version record](../toolchains/version-records/2026-08-06-esp32-build-linux.md)は
  書き換えない。**当時何を選んで何を確認したかの記録であり、**後の判断で過去の記録を塗り替えない**
- CI（`.github/workflows/`）へ Wokwi を追加しない
- `CONTRIBUTING.md`の検証手順へ Wokwi を追加しない
- **local での任意利用は禁じない。ただしその結果を、検証の根拠として正本へ書かない**

**この決定は「Wokwi が Rust＋ESP-IDF で動かない」という判断ではない。**
動くかどうかは判断要因 5 のとおり確かめていない。**決定の根拠は 1・2・3 である。**
判断要因 4 は、むしろ**動く見込みがある**ことを示している。

`simulator/deskcat-sim/` との関係も、この決定の範囲で書いておく。
**両者は代替関係にない。**`deskcat-sim` は host 上で domain 動作と protocol fixture を扱う計画であり
（**まだ README だけで実装は無い**）、Wokwi は（使うなら）firmware を ESP32 の模擬上で起動するものである。

**代替しないことの出典は、両者で別である。**`deskcat-sim` については同 README が
「実LCD、timing、電気、機構の検証を代替しない」と書いている。**Wokwi についての同等の記述は
repository 内に無い。**この ADR の判断要因3が、その判断そのものである。

## 影響

### 利点

- CI が secret を持たない状態を保てる（Machine Profiles の CI 要件）
- 未確認の事項（実動、license）を、採用の後ろに隠さずに残せる
- 保守対象が増えない。`wokwi.toml`／`diagram.json` の維持も、CI の1 jobも増えない

### 欠点

- flash 前に落とせたはずの firmware の失敗を、実機で踏む可能性が残る
- **Wokwi が実際に使えるかどうかは、この ADR の後も分からないままである**

### リスクと対策

| リスク | 対策 |
|---|---|
| 実機試験が増えて開発が詰まる | 見直し条件の1つ目に置く。**観測してから動く。**現時点で詰まっているという記録は無い |
| 「Wokwiは使えない」と誤って引用される | 決定節に**「動かないという判断ではない」**を明記した。判断要因 4 と 5 を分けてある |
| 判断要因 5・6 が埋まらないまま忘れられる | 見直し条件に埋め方を書く。**埋めた人が ADR を更新する** |

## 検証

**この決定は次のいずれかが起きたら見直す。**

1. **実機試験の待ちが、実際にボトルネックとして観測されたとき。**「増えそうだ」ではなく、
   Issue または実験記録に待ちが現れたときである
2. **CI へ credential を置く方針が変わったとき。**[Machine Profiles](../toolchains/machine-profiles.md)の
   CI 必須要件が変われば、判断要因 2 は消える
3. **判断要因 5・6 が埋まったとき。**次の2つを満たした記録が出た場合である
   - ESP32 Build profile の端末で `wokwi=true` 相当の `wokwi.toml`／`diagram.json` を用意し、
     **実際に Wokwi で起動して結果を記録した**（成立しない結論も記録に当たる）
   - **Wokwi の公式の利用条件を読み、CI と個人利用それぞれで無償の範囲に収まるかを記録した**

**3 を満たす作業は、この端末では行えない**（判断要因 5 の表）。**別 profile の端末と、Wokwi の account が要る。**

## 置き換える決定

なし。
