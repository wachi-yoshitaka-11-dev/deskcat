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
> 起票時点（2026-09-17）の作業では実行できていなかった**（判断要因5・6）。**決定はその2つに
> 依存しない根拠（判断要因1・2・3）で立てている。**
> **判断要因6は「足りるか」という問いには依然として答えていない。**2026-09-21に、
> 無償（`Free`／`Community`）plan のCI simulation時間の数値上限（月50分）と、
> Terms of Service本文（`the Service`の無償利用は`personal non-commercial purposes`限定。
> `the Extension`はopen source projectであれば別途無償）を確認した。
> **`Free`＝`Community`の対応付けは読みであり、公式資料に明示の一文は無い。**
> **DeskCatでのCI自動実行がこの条件に当たるかは、Terms本文だけでは判定できず、
> Wokwiへの問い合わせが要る**（判断要因6参照）。
> **判断要因5（実際に動かして成立するかの確認）は、依然として未実行のままである**
> （ESP-IDF toolchainとWokwi accountを要するため）。

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
| ESP-IDF toolchain が無い | 起案した作業端末に `espup`／`idf.py`／`espflash` のいずれも無い（`which` で確認） |
| toolchain を入れられない | `dl.espressif.com` へ到達できない（`curl: (56) CONNECT tunnel failed, response 403`） |
| Wokwi を呼べない | `wokwi.com` へ到達できない（同じ 403）。**token も持っていない** |

**この 403 を「端末の性質」や「profile の性質」として読まない。**
**同じ project directory を見る別 session が、3 host とも `200` で到達したと報告している**
（2026-09-17。[PR #414](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/414) の review）。
**起案 session からは再測しても 403 のままである。**したがって遮断しているのは
**session ごとの egress 経路**であり、端末でも profile でもない。

**「成立しない」ではない。「確かめていない」である。**この区別を崩さない。

### 6. 無償ライセンスの範囲は、CI simulation時間とTerms本文について確認できた（2026-09-21追記）

#325 の候補2は「無償ライセンスの範囲で足りるかを公式資料で確認する」も求めている。
**起案 session（2026-09-17）からは `docs.wokwi.com` と `wokwi.com` のどちらも読めなかった**
（`curl: (56) CONNECT tunnel failed, response 403`）。当時は「別 session が到達して引用した
内容を、確定として写さない」という立場を採り、この項目を開いたままにしていた。

**2026-09-21に、この後始末を行うsession自身が`docs.wokwi.com/wokwi-ci/getting-started`
と`wokwi.com/pricing`へ実際に到達し、読んだ。**「別sessionが到達して引用した内容を
確定として写さない」という制約（`AGENTS.md`「セッションの役割」）は、この節が対象と
していない。**同制約が対象とするのは、他sessionからの伝聞をそのまま断定として書くことである。
ここでは、この節を書いているsession自身がfetchして原文を得ている。**原文（英語）は
次のとおりである。

> Each user has a limit of simulation time per month, according to their Wokwi plan:
>
> - Free users: 50 minutes
> - Hobby and Hobby+ users: 200 minutes
> - Pro users: 2000 minutes

（`docs.wokwi.com/wokwi-ci/getting-started`、2026-09-21取得。**この link は revision を
固定していない。**既定 branch の doc であり、内容は変わりうる）

`wokwi.com/pricing`（同日取得）は表形式で、**行を一行ずつに整理した（原文の逐語引用では
ない）**。plan名と月額は次のとおりである。

- Community: €0 /mo
- Hobby: €5.6 /mo
- Hobby+: €8.1 /mo
- Wokwi Pro: €20 /seat/mo

同ページには、上位planへの案内として「contact us to discuss options for a custom plan」
という文もある。**この一文がsimulation時間の超過時を指すのか、より一般的な上位plan案内かは、
表からは判別できなかった。**

**プラン名の対応付けは推測である。**`wokwi-ci/getting-started`側の`Free`／`Hobby`／
`Hobby+`／`Pro`という表記と、`pricing`側の`Community`／`Hobby`／`Hobby+`／`Wokwi Pro`
という表記は、料金体系（`Hobby`と`Hobby+`が両ページに共通、無料側だけ名称が異なる）から
**同一のplan区分を指すと読める。**ただし両ページに「`Free`＝`Community`」と明示する一文は
無い。**したがって、Wokwi CI は無料（`Free`／`Community`）plan でも使用でき、
月50分のsimulation時間が上限である、という結論は、この対応付けの読みに依存する。**

**`AGENTS.md`の推測禁止に従い、この50分が DeskCat の実際の CI 利用に「足りる」とも
「足りない」とも書かない。**理由は、実際の1回あたりのsimulation所要時間もCI起動頻度も
測っていないためである（この repository は `wokwi=false` を維持しており、`wokwi.toml`を
使ったCI実行を1度も行っていない）。**判明したのは、無償枠に具体的な数値上限（50分/月）が
存在し、ゼロではないという事実だけである。**

**個人利用・商用利用の条件も、同日に追加で確認できた。**`wokwi.com/legal/terms`
（2026-09-21取得。この link も revision を固定していない）の`REGISTRATION, USER ACCOUNT,
EXTENSION`節が持つ利用許諾の原文は次のとおりである（`...`は取得した引用の途中省略を示し、
何が省かれたかは判別できていない。取得手段の制約による）。

> ...we hereby grant you a worldwide, limited, revocable, non-exclusive, non-sub-licensable,
> non-transferable and non-assignable right and license... to use the Service in
> accordance with these Terms, for your personal and non-commercial purposes only,
> and to use the Extension for open source projects, following the open source
> licenses terms, without charge.

**この一文は`the Service`と`the Extension`を分けて許諾している。**「personal and
non-commercial purposes only」が掛かるのは`the Service`であり、「open source projects
であれば無償」という条件が明記されているのは`the Extension`（VS Code拡張）側だけである。
`the Service`は同 Terms 内で「a web simulator for embedded & IoT Systems...which is
also available as an extension」と定義されており、**Wokwi CI（Web simulatorをCIから
呼ぶ機能）は`the Extension`ではなく`the Service`に当たると読める。**この読みが正しければ、
「open source projectだから無償」という理屈はWokwi CIには及ばず、掛かるのは
「personal and non-commercial purposes only」の方である。

`ACCEPTABLE USE`節は、禁止行為の1つとして次を挙げている。

> Using the Service at no cost for commercial purposes without receiving our explicit
> consent and agreement

同Terms内の別の条項は、商用利用の相談先として次のように案内している。

> For more details on the available options for commercial use to the Extension and
> Service, please contact us at: [連絡先。fetch結果ではmailアドレスが難読化され、
> 正確な文字列は確認できなかった]

**Wokwi CI固有の利用（GitHub Actions等での自動テスト）を名指しする記述は、この Terms
内に見当たらなかった。**Wokwi自身のdocs（`docs.wokwi.com/wokwi-ci/getting-started`）は
「robust simulation solution for automated testing...on CI systems like GitHub Actions,
GitLab CI」とCI利用を促す記述を持つ一方、Terms側の無償利用条件は`the Service`について
「personal and non-commercial purposes only」に限定されている。**DeskCatは公開
repositoryで非商用のhobby projectだが、「personal」（個人）と言えるかは別の軸である。**
CIでの自動実行が「personal」な利用に当たるかどうかは、この Terms の文面だけからは
判定できない。**`AGENTS.md`の推測禁止に従い、DeskCatでのCI利用が無償条件に
当たるとも当たらないとも書かない。**この判定には、Wokwiへの問い合わせ（Terms内で
商用利用の相談先として案内されている連絡先）が要る。

**この決定（選択肢B、Wokwi をgateにしない）は判断要因1・2・3に依っており、
判断要因6でCI simulation時間の数値上限・Terms本文が判明したことによっては変わらない**
（下の「検証」節が既にそう定めている）。

## 検討した選択肢

### 選択肢A: CI の simulation gate として採用する

push または Pull Request のたびに Wokwi で firmware を起動し、boot と初期化の失敗を実機の前に落とす。

**利点。**flash する前に、boot loop や panic を機械が見つける。実機を触れる人と時間に依存しない。

**コスト。**判断要因 1・2 のとおり、CI へ token を置くことになり、Machine Profiles の CI 要件と衝突する。
さらに判断要因 5 が未確認のまま残っており、**採用を決めても、動くかを確かめていない状態で入ることになる。**
判断要因 6 は CI simulation 時間の上限（無償`Free`／`Community` plan で月50分。plan名の
対応付けは読み）と、Terms of Service本文（`the Service`の無償利用は`personal
non-commercial purposes`限定）が判明しているが、**DeskCatでのCI自動実行がその条件に
当たるかは、Wokwiへの問い合わせを経ないと確定しない**（判断要因6を参照）。

### 選択肢B: gate にしない。local の任意利用は禁じない

`wokwi=false` を維持し、CI にも検証手順にも入れない。個人が自分の端末で使うことは妨げないが、
**その結果を検証の根拠として記録しない。**

**利点。**CI 要件と衝突しない。未確認のまま何かを採用することにならない。保守 cost が増えない。

**コスト。**flash 前に落とせたはずの失敗を、実機で踏む可能性が残る。
**ただし現時点で、実機試験がボトルネックになっているという観測は無い。**`firmware/esp32/src` はまだ4 file である（`main.rs`／`config.rs`／`health.rs`／`protocol.rs`。2026-09-17に数えた）。**[#446](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/446)により、この観測（2026-09-17時点の`firmware/esp32/src`は4 file）は現在の状態と一致しない。**この観測は2026-09-17時点の記録として保持し、書き換えない。「実機試験がボトルネックになっているという観測は無い」という結論自体の再評価は、このADRの対象外として扱う。

### 選択肢C: 判断を保留する

判断要因 5・6 が埋まるまで決めない。

**利点。**根拠が揃ってから決められる。

**コスト。**#325 の候補2が開いたまま残る。**そして判断要因5を埋める条件は、
起案 session の側では作れない**（別 profile の端末と Wokwi の account が要る）。
**「誰かが条件を揃えるまで待つ」は、待つ主体が居ないと止まったままになる。**
**判断要因6の方は別である。**到達できる session なら読めるため、保留の理由にはならない。

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
| 判断要因 5 が埋まらないまま忘れられる（6は2026-09-21にCI simulation時間の数値上限とTerms本文が判明した。CI自動実行が無償条件に当たるかは未送信のまま。Wokwiへの問い合わせ自体がまだ行われていない） | 見直し条件に埋め方を書く。**埋めた人が ADR を更新する** |

## 検証

**この決定は次のいずれかが起きたら見直す。**

1. **実機試験の待ちが、実際にボトルネックとして観測されたとき。**「増えそうだ」ではなく、
   Issue または実験記録に待ちが現れたときである
2. **CI へ credential を置く方針が変わったとき。**[Machine Profiles](../toolchains/machine-profiles.md)の
   CI 必須要件が変われば、判断要因 2 は消える
3. **判断要因 5 が埋まったとき。**ESP32 Build profile の端末で `wokwi=true` 相当の
   `wokwi.toml`／`diagram.json` を用意し、**実際に Wokwi で起動して結果を記録した場合である**
   （成立しない結論も記録に当たる）。**これには別 profile の端末と Wokwi の account が要る。**
4. **Wokwiへ問い合わせ、DeskCatでのCI自動実行が`personal non-commercial purposes`に
   当たるかの回答を得て記録したとき。**2026-09-21に、CI simulation時間の数値上限
   （無償`Free`／`Community` planで月50分。plan名の対応付けは読み）と、`wokwi.com/legal/terms`
   の原文（`personal non-commercial purposes only`／商用利用には`explicit consent`が要る）
   の両方を確認した。**ただしTerms本文には、CIでの自動実行を名指しする記述が無く、
   それが`personal non-commercial`に当たるかは文面だけでは判定できない。**この判定を
   得るには、`wokwi.com/legal/terms`が案内する連絡先へ問い合わせる必要がある。
   **この節を書いたsessionは、fetch結果でmailアドレスが難読化され、連絡先の文字列を
   正確に取得できなかった。**次に埋める session は、`wokwi.com/legal/terms`を
   ブラウザ等で直接開いて連絡先を確認するところから始める（accountもESP-IDF環境も
   不要）。**3と4を同じ条件にしない。**
   3は実際にWokwiを起動する検証で別profileの端末とWokwi accountが要るが、4は
   問い合わせと文書化だけであり、そのどちらも要らない。

**判断要因6でCI simulation時間の数値上限とTerms本文が判明しても、それだけで決定が
変わるわけではない。**決定は判断要因 1・2・3 に依っており、**5・6 はその根拠に
含まれていない。**判断要因5・6は、選択肢Aのコスト欄が挙げる**追加のコスト**
（判断要因1・2による確定的な衝突とは別の、「動くか」「使ってよいか」を確かめていない
という不確実性）の裏付けである。**選択肢Aのコスト欄は2026-09-21にあわせて更新した。**
判明した内容（無償`Free`／`Community` planは月50分、DeskCatでのCI自動実行が無償利用
条件に当たるかは未確定）は、この不確実性コストを部分的に減らした（simulation時間の
上限という数値は得た）が、**完全には解消していない**（無償条件に当たるかがまだ
確定しない）。**主たる決定根拠（判断要因1・2・3、CIへのcredential持ち込みと
Machine Profiles要件の衝突）には、この判明内容は触れていない。**
**判断要因6は数値上限とTerms本文まで判明したが、この決定を見直す事由には当たらなかった。**

## 置き換える決定

なし。
