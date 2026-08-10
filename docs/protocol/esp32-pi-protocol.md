# ESP32–Raspberry Pi Protocol

> 状態: Draft v2 — transport制限、session境界、流量制限の実装fixtureは引き続き検証が必要
> 正本とする情報: USB serial／JSON Linesのwire上の動作

## 1. 適用範囲

このprotocolは次を伝送する。

- ESP32からRaspberry Piへのboot、入力、sensor、fault、status event
- Raspberry PiからESP32へのdisplay／motion command
- Acknowledgement、error、health check、reconnect時の同期

任意のbinary asset、firmware update、audio、camera data、secretは伝送しない。

## 2. Transport

| 特性 | 初期値 | 状態 |
|---|---|---|
| 物理／論理link | USB serial | Project decision |
| Encoding | UTF-8 | Project decision |
| Framing | 1行に一つのJSON object | Project decision |
| 送信時line ending | `\n` | Draft |
| 受信可能なline ending | `\n`。直前の`\r`は除去する | Draft |
| UART framing | 8N1、flow controlなし | Candidate |
| Baud | 115200 bps | Candidate。両端で検証する |
| Encode済み最大line | 改行を含む1024 bytes | Candidate。検証が必要 |

Protocol channelから送信するすべてのbyteは、有効にframe化されたmessageの一部でなければならない。自由形式のfirmware logでJSON lineを分断してはならない。

## 3. Envelope

```json
{"v":1,"sid":41207,"id":1234,"ts_ms":456789,"type":"head_touched","payload":{}}
```

| Field | Type | 必須 | 意味 |
|---|---|---:|---|
| `v` | unsigned integer | Yes | Protocol major version |
| `sid` | unsigned integer | Yes | 送信側のsession ID。起動ごとに新しい値を選ぶ |
| `id` | unsigned integer | Yes | 同一session内で単調増加するmessage ID |
| `ts_ms` | unsigned integer | Yes | 送信側のuptime（milliseconds） |
| `type` | string | Yes | snake_caseのmessage type |
| `payload` | object | Yes | Type固有のfield |

規則:

- Top-level valueのtypeが異なる場合は不正とする。
- 空のpayloadは`null`ではなく`{}`とする。
- `ts_ms`はwall-clock timeではなく、device間で直接比較できない。
- 送信側が再起動したとき、`sid`を新しい値へ変更し、`id`を初期値へ戻す。
- **processが再起動したときは、必ず新しい`sid`を選ぶ。**再起動しても同じ`sid`を名乗ると、受信側は遷移を検知できず、旧sessionのduplicate履歴が新しい`id`空間へそのまま適用される。生成方法と衝突確率の許容値は`PROTO-TBD-011`とする。**衝突を完全には排除できないため、衝突を検知したあとの再生成を§3.1で規定する。**
- **`sid`が変わるのは、processの再起動と、衝突検知による選び直し（§3.1）のときだけである。**選び直しにprocessの再起動は要らない。serial linkの切断と再接続だけでは`sid`を変えない。link断は相手processの終了を意味せず、duplicate履歴を捨てる理由にならない。
- Message同一性は`id`単独ではなく`(sid, id)`の組で判定する。
- `boot` messageによって、新しいESP32 sessionを開始する。
- `hello` messageによって、新しいPi sessionを開始する。
- `boot`／`hello`とも、現在のsessionと同じ`sid`であればsession遷移として扱わない（§5.1）。
- Typeに別の規定がない限り、同じmajor version内では未知の追加payload fieldを無視してよい。
- 未知のmessage typeは`unknown_types`を増加させ、方向ごとに次のとおり扱う。
  - **Pi→ESP32**: `sid`と`id`を復元できる場合は、`status: rejected`、`code: unknown_type`、
    `reply_sid`、`reply_to`を持つ相関ACKを返す。復元できない場合は応答せず計数だけを行う。
    identityを復元できるのに黙って捨てると、Piは実行されたと誤認する。
  - **ESP32→Pi**: 応答せず無視する。Piは新しいevent typeの追加を前方互換に受け入れる。
  - どちらの方向でも、`type`に対応するschemaが解決できない時点で拒否するため、**受理budgetを消費しない**（§8の手順7と同じ扱い）。
  - `unknown_type`の送出は§8.2の送出上限の対象とする。
  - この判定は§8の手順7で行い、**session権限判定（手順8）より先**である。retiredな`sid`や未知の`sid`であっても、typeが未知であれば`stale_session`ではなく`unknown_type`を返す。`unknown_types`だけを増やし、`stale_sessions`は増やさない。§5.1の優先順位は、手順7を通過した既知typeに適用する。

Integer widthは、共有test fixture（§12.1）とあわせて次のとおり確定した。宣言した幅に収まらない値、負数、非整数は、envelopeとして復元できないため`invalid_envelope`で拒否する。

| Field | 型 | 根拠 |
|---|---|---|
| `v` | `u16` | major versionは増加が遅く、幅を広げる意味がない |
| `sid` | `u32` | session ID |
| `id` | `u32` | 同一session内のmessage ID |
| `ts_ms` | `u64` | uptime ms。`u32`は約49.7日でwrapし、長時間動作で`ts_ms`の単調性が崩れる |

**送信側が`id`の上限に達したときの動作は未確定である**（`PROTO-TBD-003`）。受信側の判定ではなく送信側の運用であり、session確立とduplicate履歴の扱いに関わるため、host serial session（#11）とACK／reconnect実装（#12）で決める。

### 3.1 Session IDが必要な理由

`id`は送信側の再起動でresetされる。一方で受信側は、duplicate抑止のために直近のcommand IDと処理結果を保持する（§9）。ESP32は`boot`によってPiへ再起動を通知できるが、**Piの再起動を検知する経路がなければ**、Piが振り直した`id`が、ESP32に残る旧sessionのduplicate履歴と衝突する。

その結果、次が起こりうる。

- 新規commandが旧sessionのduplicateと誤判定され、実行されずに`ok`が返る。
- 実行していない動作に対して、保持していた過去の結果が返る。

いずれも「Piは成功したと認識し、ESP32は何もしていない」状態を作る。物理動作を伴うcommandでこれを許容できない。`duplicate_expired`は履歴から失われた場合のcodeであり、この「履歴は生きているが別session」のcaseを扱わない。

`sid`と`hello`は、この不一致を構造的に防ぐために導入する。受信側は、**現在のsessionと異なる`sid`の`hello`／`boot`を受理し、session遷移を確定したときだけ**、その相手に関する**現行sessionの**ID追跡とduplicate履歴を破棄する。遷移前の現行`sid`はretired集合へ移し、**保持期間内のretired `sid`集合は残す**（§5.1）。retired sessionの個々の`(sid, id)`と結果は保持しない。同一`sid`の再送では破棄せず、保持した結果のreplay経路へ進む（§5.1）。`hello`／`boot`への保持ACKは、§8.2で定める有限の正規retry quota内では予約容量から送出する。

**`sid`の変化を検知しただけでは切り替えない。**`sid`は乱数を含みうるため、未知の値が新しいsessionなのか遅れて届いた古いmessageなのかを区別できない。切り替えの条件は§5.1で規定する。

`sid`の生成方法（乱数、不揮発カウンタ、またはその併用）、再起動直後に同じ値を選ぶ確率の許容値、retired sessionの保持件数と期間、および`hello`の同一identity最大retry回数は`PROTO-TBD-011`とする。

**保持件数の上限によって、保持期間の満了前にretired sessionを追い出してはならない。**
追い出すと、その`sid`は「未知」に戻る。遅れて届いた`hello`／`boot`が遷移候補として
受理され、現在のduplicate履歴を破棄して実行中motionを停止する。
`stale_session`で拒否されるはずの経路が、逆に副作用を起こす経路になる。

したがって保持件数は、**保持期間内に起こりうる最大のsession遷移回数から決める**。
遷移速度の上限も併せて定め、上限を超える遷移は`rate_limited`で拒否する（§5.1）。

所有は分ける。**保持件数と保持期間は`PROTO-TBD-011`**、**遷移速度の上限と
cooldownは`PROTO-TBD-012`**である。`PROTO-TBD-011`は`sid`の生成、衝突回復、
retired sessionの保持、および`hello`送信側の有限なsession回復上限を扱う。
送出budget、table上限、帯域などlink全体の負荷管理parameterは持たない。
session churnのfixtureを§12へ追加する。

保持件数の下限は単位を含めて導出する。`PROTO-TBD-012`の遷移上限を
任意の連続windowあたり`N_transition`回、window長を`T_window`、`PROTO-TBD-011`の保持期間を
同じ時間単位の`T_retention`としたとき、retired session保持件数は
**`N_transition × ceil(T_retention / T_window)`件以上**とする。保持期間がwindowの
端数を含む場合は切り上げる。各TBDの確定記録には数値だけでなく時間単位も残す。

#### `sid`が衝突した場合

新しく選んだ`sid`が、受信側のretired session集合に残っている値と一致することがある。
この`hello`／`boot`は§5.1の優先順位1によって`stale_session`で拒否され、遷移しない。

**同じ`sid`で再送しても永久に拒否される。**`boot`は同じ`(sid, id)`で再送し、
`hello`も同じ`sid`を保つため、どちらの再送経路も自力では抜けられない。

したがって次を規則とする。

- **`hello`／`boot`に対して`stale_session`を受けた送信側は、再送の前に新しい`sid`を選び直す。**
  `id`も初期値へ戻す。これは衝突以外では起こらない（現在のsessionなら遷移不要、
  未知なら遷移候補になる）ため、`stale_session`はretired衝突の識別子として使える。
- **`sid`を選び直すのは、明示的な`stale_session`を受信したときだけである。**
  ACKが無いだけでは選び直さない。送信側は、ACK喪失と`sid`衝突を区別できない。
  最初の`hello`が処理済みでACKだけが失われていた場合、新しい`sid`で送り直すと
  **2回目のsession遷移**が確定し、duplicate履歴の破棄と実行中motionの停止が
  もう一度起きる。link損失のたびに履歴が消える経路を、送信側の推測で作らない。
- `hello`が無応答の場合は、`reason`によらず**現在の`sid`のまま**、同じ`(sid, id)`で
  有限のrecovery budget内で再送する。使い切ったら`protocol_fault`で報告して停止する。
  ESP32側の保留tableへ受け付けたidentityには、保持ACKを§8.2の予約容量から返し、
  `stale_session`を含む拒否ACKにもTTL内で少なくとも1回の送出機会を与える（§5.1）。
  専用budgetが無ければ即時応答を延期する。table上限で
  新規identityを受け付けられない場合は応答せずlocal counterへ記録する。したがって
  無応答は応答経路の障害または有界な過負荷を表し、`sid`を変えても直らない。
- **`reason`が`port_reopen`／`resync`の`hello`では、`sid`を選び直さない。**
  §7はこの2つに現在の`sid`を要求しており、選び直すと`reason`と`sid`の組が矛盾して
  `invalid_payload`で拒否される。link再接続とresyncが、無応答をきっかけに
  永久に成立しなくなる。無応答が続く場合は、**同じ`(sid, id)`で有限のrecovery budget内で
  再送し、使い切ったら`protocol_fault`で報告して停止する。**
  **停止状態から自動で`startup`へ切り替えない。**budget満了を契機に`reason`を
  `startup`へ変えると、processが動き続けたまま新しい`sid`を名乗ることになり、
  link断だけでduplicate履歴の破棄と実行中motionの停止を起こせる。§3が
  `sid`の変更をprocess再起動と衝突回復に限っているのは、この経路を塞ぐためである。
  `startup`で再開してよいのは、**processを再起動したとき**、または
  **運用者が明示的にsession resetを指示したとき**だけとする。後者は
  protocol内の自動遷移ではなく、外部からの操作として扱う。
- 選び直しの回数には上限を設ける。上限に達したら`protocol_fault`で報告し、
  session確立messageの自動送出を停止する。運用者の明示的なsession resetまたはprocess再起動まで
  再開せず、motion commandを送らない。上限後も自動試行を続けると、上限が実質的に無くなる。
- **この規則は`hello`／`boot`自身が拒否された場合にだけ適用する。**通常commandが
  `stale_session`で拒否されたのは、そのsessionがまだ確立されていないという意味であり、
  衝突ではない。この場合は現在の`sid`のまま`hello`から再開する（§10.2）。
  区別せずに選び直すと、正常な`sid`を捨てて確立をやり直し続ける。
- **retired sessionの保持期間は、遅延messageの最大生存時間と再送windowの合計を
  下回ってはならない。**短すぎると、retiredから外れた旧sessionのmessageが
  現在のsessionとして受理される。

ACK timeoutは`PROTO-TBD-004`、`stale_session`を契機とする選び直し回数の上限、無応答時のrecovery budget、同一identityの最大retry回数、およびretired session保持期間の下限は
`PROTO-TBD-011`に含める。`boot`の無応答時はこの`hello`規則ではなく§4.1に従う。

## 4. ESP32からPiへのmessage

### 4.1 `boot`

Protocol taskの準備完了後に一度送信する。

```json
{"v":1,"sid":41207,"id":1,"ts_ms":100,"type":"boot","payload":{"firmware":"0.1.0","board":"esp32","reset_reason":"power_on"}}
```

必須payload:

| Field | Type | 意味 |
|---|---|---|
| `firmware` | string | Firmware version／build identity |
| `board` | string | Firmware board-configuration ID |
| `reset_reason` | string | Machine-readableなreset reason |

将来のoptional fieldにはprotocol capabilityとhardware revisionを含められる。Secretまたは生のenvironment dataを公開しない。

Piは`boot`を受信したとき、`sid`が現在のESP32 sessionと**異なる場合にだけ**session遷移として扱い、旧sessionのID追跡を破棄する。

`boot`も再送されうる（ESP32のretry、Pi側の取りこぼし後の再読み出し）。現在のsessionと同じ`sid`の`boot`でID追跡を破棄すると、`hello`と同じ理由で、直後のretryが「未処理」と判定され二重実行を招く。§5.1の`hello`と同じ規則を適用する。

#### `boot`の受理確認と再送

**`boot`は`hello`と同じくACKを必要とする。**Piは、その`boot`の`(sid, id)`を`(reply_sid, reply_to)`へ写したACKを返す。

現在のsessionを前提とする任意のcommandを受理確認にしてはならない。commandはPi側のsessionを識別するだけで、ESP32の新しい`(sid, id)`を参照しない。Piが`boot`を取りこぼしたまま`get_status`を送ることはありうる。それを確認とみなすと、Piが承認していないsessionをESP32が「承認済み」として扱う。

確認を得るまで、ESP32は**同じ`(sid, id)`で**`boot`を再送する。再送間隔と上限回数は`PROTO-TBD-017`とする。ただし`stale_session`を受けた場合は例外で、`sid`がretired sessionと衝突している。同じ`(sid, id)`で再送せず、新しい`sid`を選び直して`id`を初期値へ戻す（§3.1）。

終了条件を次の一つの契約として定める。`PROTO-TBD-017`はこの契約のparameter（初期間隔、backoff係数、通常再送の回数、recovery間隔、recovery budget）だけを決める。

| 事象 | ESP32の動作 |
|---|---|
| ACKが返らない（通常再送の回数以内） | 初期間隔でbackoffしながら再送する |
| 通常再送の回数を超えた | **直ちには止めない。**recovery間隔まで延ばし、有限のrecovery budget内で再送を続ける |
| ACKが無いままrecovery budgetを使い切った | `boot`の送出を止め、サーボ出力を有効にせず`protocol_fault`で報告する。Piから有効な`hello`を受信したら、**同じ`(sid, id)`のまま**新しい有限budgetで`boot`の再送を再開してよい。ただし**再開は1つのPi sessionにつき1回まで**とする（下記）。**`sid`は選び直さない。** |
| `status: ok`のACKを受信 | 再送を終了する |
| `status: rejected`かつ`code`が`stale_session`のACKを受信 | **終端ではない。**`sid`がretired sessionと衝突している。新しい`sid`を選び直し`id`を初期値へ戻して再送する（§3.1） |
| `status: rejected`かつ`code`が`rate_limited`のACKを受信 | **終端ではない。**一時的な流量制限であり、再送で解消しうる。cooldown経過後に**同じ`(sid, id)`で**再送する。`PROTO-TBD-017`の有限budget内に収め、使い切ったら`protocol_fault`で報告して停止する |
| `status: rejected`（`stale_session`／`rate_limited`以外）のACKを受信 | **終端応答として再送を終了する。**`code`を`protocol_fault`で報告し、Piの介入を待つ |
| `sid`／`id`を復元できず、Piが相関ACKを構成できない | 有限budgetまで再送し、使い切ったら送出を止める。サーボ出力を有効にせず`protocol_fault`で報告する（下記） |

通常再送の上限で**直ちに**止めないのは、Piが一時的に`boot`またはACKを取りこぼしただけでsessionを承認する経路が消えるためである。一方、無応答のまま無期限に送出するとlink帯域を占有する。有限のrecovery期間を確保し、満了後は物理出力を無効に保ったまま外部から観測可能な停止状態へ移る。

一方、**Piが明示的に拒否した`boot`を再送し続けてはならない。**不正なenvelope、未対応version、payload不正は再送で解決しない。**`rate_limited`だけは例外**で、一時的な流量制限であり再送で解消しうる（上表）。予約容量が枯渇しうる以上、これを終端にすると有効な`boot`がsession確立前に停止する。Piは`sid`と`id`を復元できる`boot`を拒否するとき、`status: rejected`のACKを下記の専用budgetと**Piが所有するESP32→Pi方向の保留table**による送出対象にする。復元できない場合は相関ACKを構成せず、ESP32が上記の有限budgetで再送を終える。

抑制対象外とするのは、**`boot`の送出と、受理時に返す最初の`status: ok`のACKだけ**である。
duplicateへ返す保持ACKは§8.2の有限な正規retry quotaと予約容量に従う。
recovery間隔まで延びた再送も対象外だが、間隔が延びているため帯域への影響は通常再送より小さい。

**拒否ACK（`status: rejected`）は§8.2の送出上限の対象とする。**不正な`boot`は
schema検証で拒否されて受理budgetを消費しないため、対象外にすると、
異なる`id`の不正な`boot`を送り続けるだけで1入力ごとにACKを強制でき、
Pi→ESP32方向のcommand／session確立messageとESP32側の受信処理を圧迫できる。

ただし、`sid`と`id`を復元でき、Pi側の保留tableへ受け付けた`boot`には、**同一`(sender_role: ESP32, sid, id)`につき少なくとも1回は拒否ACKを返す**。返さないと、
送信側はrecovery budgetが満了するまで終端応答を得られない。

**拒否ACKを集約してはならない。**ACKは`reply_to`を1つしか持たず、
1件のACKで複数の`(sid, id)`を終端できない。集約は「少なくとも1回返す」と両立しない。
異なる`id`の不正な`boot`が連続する場合は、**拒否ACK専用の送出budget**で抑える。
budgetを使い切った受信機会では即時応答せず、受け付け済みentryを送出待ちのまま残し、
Pi側のlocal `suppressed_responses`で計数する。budget回復後は下記の公平性規則で再送する。
この値はESP32が送る`status`には含めない。budget値は`PROTO-TBD-012`で扱う。

**「少なくとも1回返す」義務は無制限ではない。**そのままでは、異なる`id`の不正な`boot`を
送り続けるだけで未応答の`(sid, id)`が際限なく積み上がり、受信側は全件を覚え続け、
送信側は全件を再送し続ける。parserとlinkの容量を消費し、`status`とfault eventを枯渇させうる。

次で有界にする。

- 応答を保留しているidentityは**受信側が所有する上限付きのtableで管理する。**`boot`用はPi、
  `hello`用はESP32が所有し、他方のprocessとmemoryを共有しない。entryのkeyは
  `(sender_role, sid, id)`とし、方向をidentityの一部にする。上限に達したら、
  それ以上の新しいidentityを受け付けず、受信側のlocal `suppressed_responses`で計数する。
- 各entryはACKの残り送出回数を保持する。初回の送出機会に加え、送信側へ許可する
  同一identityの最大retry回数分だけを確保する。`hello`の最大retry回数は
  `PROTO-TBD-011`、`boot`は`PROTO-TBD-017`で定める。ACKをwireへ渡すたびに一つ減らし、
  0になった後の反復には応答せず、受信側のlocal `suppressed_responses`を増やす。
- 同じentryに対して同時に保持する未送出ACKは1件までとする。送出待ちの間に同じidentityを
  再受信してもentryや未送出ACKを追加せず、その受信機会で即時応答しなかったことだけを
  受信側のlocal `suppressed_responses`へ記録する。残quotaはwireへ渡すまで減らさない。
- 保留entryにはTTLを設ける。満了したentryは破棄し、以後その`(sid, id)`への
  応答義務を負わない。
- budget回復時の応答は、古いentryから順に行う。特定の送信元が新しい`id`を
  送り続けても、先に保留された分が飢えないようにする。
- TTLは、table上限まで保留されたときの最悪送出待ち時間を下回ってはならない。
  tableへ受け付けたentryが1回も送出機会を得ないまま期限切れになる設定を禁止する。
- 送信側は、`PROTO-TBD-017`のrecovery budgetを使い切ったら`boot`の再送を止め、
  サーボ出力を有効にせず`protocol_fault`で報告する（§4.1）。

上限件数、TTL、公平性の規則は`PROTO-TBD-012`に含める。`PROTO-TBD-017`の
recovery budgetが表す総待ち時間は、ここで決める最悪送出待ち時間を下回ってはならない。

`hello`と`boot`の拒否ACKは異なる受信側が生成するため、一つの実体を共有してはならない。
ESP32はPi→ESP32方向の`hello` table、PiはESP32→Pi方向の`boot` tableを所有する。
`PROTO-TBD-012`では各方向の件数上限と送出budgetを独立に定め、その**和をlink全体の
静的上限**として記録する。片方向の未使用枠を他方向へ貸し出さない。このため、
`hello`と`boot`を交互に送っても、各方向の上限も合計上限も迂回できない。

`rate_limited`だけは一時的な拒否であり、最終の処理済み結果として扱わない。
同じ`(sid, id)`の再送では、保留した拒否を手順8でreplayせず、手順9の現在のbudgetと
cooldownを再評価する。再評価で受理した場合は、同じentryに残る未送出の拒否ACKを
取り消してから`status: ok`を最終結果として保存し、古い`rate_limited`を後から送らない。
保留tableへ入れる場合のretryable区分は§5.1に従う。

**拒否ACKを返せるのは、`sid`と`id`を復元できた場合だけである。**ACKは
`(reply_sid, reply_to)`で要求を参照するため、`sid`または`id`が欠落しているか、
型が不正でidentityとして復元できないmessageには、相関の取れたACKを構成できない。
`type`が未知であること自体は相関不能の理由ではない。identityを復元できる
Pi→ESP32 messageには`unknown_type`の相関ACKを返す。この場合Piはidentityを
復元できないmessageだけ`parse_errors`へ計上して応答しない。

したがってESP32側にも終端規則が要る。**`boot`のenvelope identityは送信側が
生成するものであり、復元できない`boot`を送るのは送信側の実装障害である。**
ESP32は自分が送出した`boot`のidentityを検証してから送る。

それでも応答が得られない場合、**再送では直らない。**identityが復元できない以上、
Piは何度受けても相関ACKを構成できない。通常再送とrecovery再送に**有限のbudget**を設け、
使い切ったら`boot`の送出を止める。

止めたあとは次のとおりとする。

- **サーボ出力を有効にしない。**sessionが確立していない状態で物理動作を許さない。
- `protocol_fault`を送出し、`status`のcounterで観測できるようにする。
- **motion commandを受理しない。**session未確立のまま届いたcommandは`stale_session`で拒否する。
  停止状態を抜けるには、**新しい`boot`のACKでsessionを確立することが必要**である。
- Piからの有効な`hello`を受信したら、**同じ`(sid, id)`で**`boot`の再送を新しい有限budgetで再開してよい。
  **ただし1つのPi sessionにつき1回だけである。**再開したかどうかをPi sessionごとに記録し、
  同一Pi sessionからの2通目以降の`hello`ではbudgetを再生成しない。
  記録は、`hello`によるPi session遷移が確定したときにだけ解除する。

**この再開で`sid`を選び直してはならない。**`sid`が変わるのはprocessの再起動と衝突検知による
選び直し（§3.1）だけであり（§3）、通常のcommand受信や`reason`が`resync`の`hello`は
どちらにも当たらない。停止状態からの再開を`sid`変更の契機にすると、受信側はduplicate履歴を
破棄する経路を1つ余分に得る。相手は無応答のまま`hello`を送るだけで履歴を捨てさせられ、
非idempotentなcommandの二重実行を防ぐ仕組みが迂回される。
**新しいESP32 sessionが必要な場合は、processを再起動するか§3.1の衝突回復に従う。**
どちらの経路でも、新しい`sid`の`boot`がACKされるまでmotion commandを受理しない。

無限に再送すると、直らない障害のために送出帯域を占有し続ける。
budget値は`PROTO-TBD-017`に含める。

再送に対するPiの動作は、最初の`boot`を処理したかどうかで分かれる。判定基準は同じで、
**受信した`sid`が現在のESP32 sessionと異なるか**である（上記）。

- 最初の`boot`が届いていなかった場合、その`sid`はまだ現在のsessionではない。再送で
  1回だけ遷移を確定し、旧sessionのID追跡を破棄してACKを返す。
- Piが処理済みでACKだけが失われた場合、その`sid`は既に現在のsessionである。遷移は
  起こらず、保持したACKのreplay経路へ進む（§9）。保持ACKは§8.2の正規retry quota内で
  予約容量から送出する。ID追跡は破棄しない。

どちらでも遷移は高々1回であり、再送が二重実行を招かない。

### 4.2 `head_touched`

```json
{"v":1,"sid":41207,"id":22,"ts_ms":8400,"type":"head_touched","payload":{"duration_ms":720,"strength":0.68}}
```

正確なtouch hardwareが確定するまで、`strength`の意味と範囲は`TBD`とする。比較可能なstrengthが存在しない場合、値を捏造せずfieldを省略する。

### 4.3 `tapped`

```json
{"v":1,"sid":41207,"id":23,"ts_ms":9300,"type":"tapped","payload":{"magnitude_g":1.84}}
```

正確なevent classifierと報告するmagnitudeの意味は、選定したaccelerometerと実験根拠を必要とする。

### 4.4 `lifted`

```json
{"v":1,"sid":41207,"id":24,"ts_ms":11000,"type":"lifted","payload":{"duration_ms":1200}}
```

最初の受け入れsliceではoptionalとする。Accelerometer classifierを検証した後にのみ有効化する。

### 4.5 `environment`

```json
{"v":1,"sid":41207,"id":25,"ts_ms":15000,"type":"environment","payload":{"temperature_c":27.4,"humidity_pct":55.1,"pressure_hpa":1008.3}}
```

選定したsensorが対応する測定量だけを含める。最初のdraftでcompact eventからquality fieldを省略する場合でも、各実装はfreshnessとdevice errorを追跡しなければならない。

### 4.6 `status`

`get_status`へのresponseとして送信し、必要に応じてrate limit付きの定期health messageとして送信する。

初期payload group:

```json
{
  "v":1,
  "sid":41207,
  "id":26,
  "ts_ms":15100,
  "type":"status",
  "payload":{
    "firmware":"0.1.0",
    "reset_reason":"power_on",
    "display":{"state":"ready","expression":"neutral"},
    "servo":{"state":"disabled"},
    "sensors":{"touch":"unknown","acceleration":"unknown","environment":"unknown"},
    "protocol":{"parse_errors":0,"invalid_payloads":0,"unsupported_versions":0,"oversize_lines":0,"unknown_types":0,"rate_limited":0,"busy":0,"out_of_range":0,"stale_sessions":0,"session_switches":0,"suppressed_responses":0}
  }
}
```

`protocol`のcounterは、拒否と抑制の規則が要求する事象をすべて観測可能にする。

| Counter | 対応する規則 |
|---|---|
| `parse_errors` | §8のUTF-8／JSON／envelope不正。**`sid`／`id`を復元できず相関ACKを構成できなかったmessageもここへ計上する**（§4.1、§5.1） |
| `invalid_payloads` | §8の手順7（type固有schema）または手順9（現在stateとの整合）で`invalid_payload`として拒否した件数。`reason`と`sid`が矛盾する未処理の`hello`や、現在の`sid`で新しい`id`を使う`boot`を含む。envelopeは読めており`parse_errors`とは区別する |
| `unsupported_versions` | §8の手順7で`unsupported_version`として拒否した件数。version不整合を、payload不正や解析失敗と混ぜない |
| `oversize_lines` | §8のoverflow |
| `unknown_types` | §3の未知type。方向を問わず計上する |
| `rate_limited` | §8.1の受理上限超過、§5.1のsession遷移budget／cooldown超過、[Servo Safety Limits](../hardware/servo-safety-limits.md)の秒あたり受理command数超過を**合算**する |
| `busy` | `busy`で拒否した件数。resourceの一時的な占有による拒否 |
| `out_of_range` | `out_of_range`で拒否した件数 |
| `stale_sessions` | §5.1の`stale_session`拒否 |
| `session_switches` | §5.1の実際に発生したsession遷移 |
| `suppressed_responses` | **ESP32が受信機会に即時送出しなかった応答の総数。**§8.2で集約・抑制した拒否応答、正規retry quotaを超えた保持ACK、duplicateへの**非ACKの**保持結果、ESP32所有のPi→ESP32保留tableで送出待ちの間に重ねて受信した同一identity、およびtable上限で受け付けなかった新規identityを含む。保留後に送出できても、以前の受信機会で即時送出しなかった計数は戻さない。Piが抑制または保留した`boot`への応答はPi側の同名local counterへ記録し、このfieldへ合算しない。内訳が必要になった時点でfieldを分割する（`PROTO-TBD-006`） |

`rate_limited`はreceiver層とservo層の拒否を合算する。層ごとの内訳が必要になった時点で
fieldを分割する。合算のままにするか分割するかは`PROTO-TBD-006`（最終status field）で決める。

抑制した応答を計数しなければ、「静かに返していない」状態を外から検出できない。
拘束／過負荷は`rate_limited`ではなくfault eventで報告するため、ここには含まれない。

最終schemaは、意味が安定し、encode後のsizeに上限を設けられるfieldだけに絞る。

### 4.7 完了・fault event

提案中のtype:

- `motion_completed`
- `motion_stopped`
- `sensor_fault`
- `protocol_fault`

長時間commandには直ちに`ack`を返し、完了時には元のcommandの
`(sid, id)`を`reply_sid`と`reply_to`で参照する個別eventを使用する。
`id`だけでは、command送信側の再起動後に別sessionのcommandと区別できない。

## 5. PiからESP32へのcommand

### 5.1 `hello`

Piがprocessを起動した直後、およびserial portを開き直した直後に、他のcommandより先に一度送信する。

```json
{"v":1,"sid":90312,"id":1,"ts_ms":40,"type":"hello","payload":{"host":"deskcatd","version":"0.1.0","reason":"startup"}}
```

必須payload:

| Field | Type | 意味 |
|---|---|---|
| `host` | string | Host process identity |
| `version` | string | Host version／build identity |
| `reason` | string | `startup`、`port_reopen`、`resync`のいずれか |

`reason`が`port_reopen`または`resync`の場合、Piは**現在の`sid`を維持する**。serial linkを開き直しただけではPi processは再起動しておらず、sessionは変わらない。同一`sid`の`hello`はsession遷移として扱われず、duplicate履歴も破棄されない。新しい`sid`を使うのは`startup`のときだけである（§3）。

`resync`はstateの再取得を要求するものであって、sessionをresetしない。
Piがsessionをやり直す手段は、processの再起動と、衝突検知による`sid`の選び直し（§3.1）だけである。`resync`はどちらでもない。
duplicate履歴を任意に破棄できる経路を設けると、二重実行を防ぐ仕組みが迂回できる。

この対応関係は**受信側が検証する**。`reason`の列挙値とtype固有schemaは§8の手順7、
現在のsessionに対する`reason`と`sid`の整合は、duplicate照会後の手順9で検証する。
処理済みの`startup hello`を現在の`sid`で再送した場合は、整合判定より先に手順8で
保持ACKを返す。未処理で`reason`と`sid`が矛盾する`hello`だけを`invalid_payload`で
拒否し、**session stateを変更しない**。受理budget、遷移budget、cooldownも消費しない。

- `startup`なのに`sid`が現在のPi sessionと同じ
- `port_reopen`または`resync`なのに`sid`が現在のPi sessionと異なる

`sid`の不一致だけで遷移を判定すると、schema上は妥当な`port_reopen`が
新しい`sid`を持つだけでduplicate履歴の破棄とmotion停止を起こせる。
逆に`startup`が現在の`sid`を名乗ると、新しいsessionが確立されない。

ESP32は`hello`を受信したとき、`sid`が現在のPi sessionと異なる場合にだけ、次を実行する。

1. 送信元の`sid`を現在のPi sessionとして記録する。
2. 旧sessionのcommand ID追跡とduplicate履歴を、**現行session分として**破棄する。旧`sid`はretired session集合へ移し、保持期間中は`stale_session`判定に使う（§5.1）。
3. 実行中のrelative motionを安全に停止する。
4. ACKを返す。

`hello`はACKを必要とする。ACK後、Piは`get_status`で実stateを取得する。

#### 現在sessionで未処理のsession確立message

手順8で現在の`sid`と一致し、まだ処理済みでない`(sid, id)`を見つけた場合の動作を
明示する。処理済みなら、以下へ進まず保持結果をreplayする。

- `hello`: `reason`が`port_reopen`または`resync`であれば、sessionを維持するための
  control messageとして受理する。通常の受理上限だけを適用し、遷移budgetとcooldownは
  消費しない。`status: ok`のACKを最終結果として保存して返すが、session遷移、
  duplicate履歴の破棄、実行中motionの停止は行わない。`startup`で現在の`sid`を
  指定した`hello`は上記の整合規則どおり`invalid_payload`で拒否する。
- `boot`: 現在の`sid`で正規の再送なら同じ`(sid, id)`であり、手順8のreplayで終わる。
  **現在の`sid`に未処理の新しい`id`を付けた`boot`は`invalid_payload`で拒否し、
  最終の拒否結果を保存する。**ESP32 processの再起動には新しい`sid`が必須であり、
  これを受理すると旧sessionのduplicate履歴を残したまま新しいID空間を承認してしまう。
  拒否時もsession state、duplicate履歴、実行中motionを変更しない。

#### 遷移は完全な検証を通してから確定する

session遷移は、duplicate履歴の破棄と実行中motionの停止という**取り消せない副作用**を伴う。
検証の途中で副作用を起こしてはならない。

この手順が対象とするのは、**§8の手順7と手順8を通過し、現在sessionのduplicateではないと
確認された遷移候補**である。処理済みの`(sid, id)`の再送は手順8で保持結果を返して終了し、
ここへ到達しない。範囲を限定しないと、飽和時やcooldown中に`hello`／`boot`の再送が
保持ACKではなく`rate_limited`で拒否されうる。

次の順で処理する。副作用は最後のstepでだけ発生する。

1. Envelopeを検証する（`v`、`sid`、`id`、`ts_ms`、`type`、`payload`のtypeと必須性）。
2. `v`が対応するmajor versionか確認する。対応外なら`unsupported_version`を返す。
3. **`type`に対応するschemaで**payloadを検証する。`hello`は`host`、`version`、`reason`（§5.1）。`boot`は`firmware`、`board`、`reset_reason`（§4.1）。相手側のschemaで検証してはならない。
4. Session権限とduplicateを照会する。処理済みなら保持結果を返して終了する。
5. 未処理messageの`reason`と`sid`の整合を検証する。不整合なら`invalid_payload`で拒否する。
6. 受理上限と遷移budgetを判定する（§8.1）。
7. ここまで**すべて通った場合にだけ**遷移を確定し、履歴破棄とmotion停止を行う。

不正なenvelope、未対応version、不正payloadの`hello`／`boot`は、
**session stateを一切変更せずに**拒否する。
検証前に遷移を確定すると、壊れた制御messageだけでretryを無効化し、動作中のmotionを止められる。

#### 遷移自体にも上限を設ける

§8.1で`hello`／`boot`に受理budgetを予約するが、予約は「枯渇させない」ための措置であり、
**遷移を無制限に許す意味ではない**。予約枠があるために、有効な`hello`を`sid`を変えながら
連続送信すると、そのたびに履歴破棄とmotion停止を起こせる。

したがって、受理budgetとは別に**遷移そのものの上限**を設ける。

- 単位時間あたりに確定してよいsession遷移の回数に上限を設ける。
- 上限超過分は`rate_limited`で拒否し、session stateを変更しない。
- 直前の遷移から一定時間はcooldownとし、その間の遷移要求を拒否する。
- 拒否した遷移要求も`stale_sessions`ではなく`rate_limited`として計数する。

上限値とcooldownは`PROTO-TBD-012`で扱う。

#### `hello`の再送は履歴を破棄しない

`hello`もACKを必要とするため、ACKが失われるとPiは同じ`(sid, id)`で再送する（§9）。`stale_session`を受けた場合だけは、`sid`を選び直してから再送する（§3.1）。

**現在のsessionと同じ`sid`の`hello`は、session遷移として扱わない。**
`(sid, id)`が既に処理済みであれば、保持したACKのreplay経路へ進むだけとし、
duplicate履歴の破棄とmotion停止を再実行しない。実際のACK送出は§8.2の正規retry quota内で予約容量から行う。

この規定がない場合、次が起こる。

- `hello`のACK消失による再送のたびに、そのsessionのduplicate履歴が消える。
- 履歴を失った直後に、非idempotentなcommandのretryが「未処理」と判定され、二重実行される。

履歴を破棄してよいのは、**実際にsessionが遷移したとき**だけである。

#### `hello`の拒否は終端応答として扱う

`hello`はACKを必要とする。ACK喪失と`stale_session`衝突は上記と§3.1で扱うが、
**それ以外の拒否codeに対する送信側の動作**を定めないと、送信側は終端結果を得られないまま
再送を続けるか、`sid`を選び直し続ける。§4.1が`boot`に対して定めた契約を`hello`にも対にして置く。

**`sid`と`id`を復元でき、ESP32側の保留tableへ受け付けた`hello`には、
`(reply_sid, reply_to)`で要求を参照する`status: rejected`のACKを返す。**
同一`(sender_role: Pi, sid, id)`につき**少なくとも1回**は返す。
返さないと、送信側は§3.1のtimeoutと選び直しの経路しか持たず、拒否理由に到達できない。

**この義務は無制限ではない。**`boot`側とまったく同じ理由で、そのままでは異なる`id`の
不正な`hello`を送り続けるだけで未応答の`(sid, id)`が際限なく積み上がる。
ESP32は、Pi→ESP32方向の`hello`拒否ACK用tableを所有し、entryを
`(sender_role: Pi, sid, id)`で識別する。上限に達したらそれ以上の新しいidentityを
受け付けずESP32側のlocal `suppressed_responses`で計数し、TTLが満了したentryは破棄して以後その
identityへの応答義務を負わない。上限件数・TTL・公平性規則は§4.1のPi側`boot` tableと
同じ性質を持つが、memoryは共有しない。方向別上限とその和であるlink全体の静的上限は
`PROTO-TBD-012`で扱う。

ただし、**`rate_limited`は最終の処理済み結果としてduplicate履歴へ保存しない。**
送出待ちのため保留tableへ一時登録する実装では、entryをretryableとして区別し、
手順8の最終結果replayには使わない。同じ`(sid, id)`の再送は手順9で現在のbudgetと
cooldownを再評価する。cooldown後に受理できた場合は、`status: ok`の最終結果で置き換える。
同じentryに未送出の`rate_limited` ACKがあれば受理確定前に取り消し、`status: ok`の後から
古い拒否ACKを送らない。
`invalid_payload`、`unsupported_version`など再評価しても変わらない拒否は、従来どおり
最終の拒否結果として保持してよい。

| 拒否code | 送信側の動作 |
|---|---|
| `invalid_payload` | **終端。**再送で直らない。`hello`の送出を止め、`protocol_fault`で報告する。同じ`sid`のままcommandを送らない |
| `unsupported_version` | **終端。**同上。version交渉なしに再送しても結果は変わらない |
| `rate_limited` | **終端ではない。**cooldown経過後に**同じ`(sid, id)`で**再送する。`sid`を選び直さない。ただし**有限のretry budget**を設け、使い切ったら終端として`protocol_fault`で報告する。budgetとcooldownは`PROTO-TBD-012`で扱う |
| `stale_session` | **終端ではない。**retired sessionとの衝突であり、`sid`を選び直して`id`を初期値へ戻す（§3.1） |

`rate_limited`にbudgetを設けるのは、cooldown経過後の再送だけでは終端状態が
定義されないためである。受信側が飽和し続ける限り、送信側は永久に再送しうる。

終端と判定した送信側は、**motion commandを送らない。**sessionが確立していない状態で
物理動作を要求しない。`get_status`のような読み取りも、sessionが未確立なら`stale_session`で拒否される。

**拒否ACKが抑制または喪失した場合**も、送信側は`sid`を選び直さない（§3.1）。
現在の`sid`のまま同じ`(sid, id)`で有限のrecovery budget内で再送する。受信側は
保留tableへ受け付けたidentityにはTTL内で少なくとも1回の送出機会を保証する。
専用budgetが無ければ即時応答を延期し、その受信機会をESP32側のlocal counterへ記録する。
table上限で受け付けなかったidentityには保証せず、同じcounterへ記録する。
どちらの場合もbudgetを使い切ったら`protocol_fault`で報告して停止する。黙って停止しない。ACK喪失を契機に`sid`を
変えると、処理済み`hello`のACKだけが失われていた場合に2回目のsession遷移を作る。

`hello`の拒否ACKも§8.2の対象だが、**保留tableへ受け付けたidentityには、初回送出機会と
`PROTO-TBD-011`の最大retry回数を合わせた有限quotaを確保する。**専用budgetは`boot`側と
同じ考え方で`PROTO-TBD-012`に含める。quota超過分と、table上限で受け付けなかった
新規identityは抑制し、その分をESP32側のlocal `suppressed_responses`で計数する。

**`sid`または`id`を復元できない`hello`**には相関ACKを構成できない。受信側は
`parse_errors`を増やして応答しない。送信側は自分が送出した`hello`のidentityを
送出前に検証する。それでも応答が無い場合、§3.1の有限budgetで送出を止め、
`protocol_fault`で報告して**sessionを確立しないままmotion commandを送らない**。

#### Session切り替えは`hello`／`boot`だけが起こす

`sid`は乱数を含みうるため、受信側は未知の`sid`を見ただけでは、それが新しいsessionなのか、切り替え前のsessionから遅れて届いたmessageなのかを値だけでは判別できない。

したがって次を守る。

この優先順位は、§8の**手順7（envelopeと`type`に対応するschemaの検証）を通過したmessageに適用する。**未知typeは手順7で`unknown_type`として拒否され、ここへ到達しない（§3）。

**判定の優先順位**（§8の手順8の内側で、duplicate照会より前に適用する）:

1. 送信元`sid`がretired sessionに含まれる場合、`type`にかかわらず`stale_session`で拒否する（手順7を通過した既知typeが対象）。**duplicate照会より先に判定し、保持した結果を返さない。**
2. 送信元`sid`が現在のsessionの場合、duplicate照会の対象とする。処理済みなら保持した結果のreplay経路へ進む。保持ACKは§8.2の正規retry quota内で予約容量から送出し、非ACKの保持結果には§8.2の送出上限を適用する。
3. 送信元`sid`が未知で`hello`／`boot`の場合、遷移の候補とする。
4. 送信元`sid`が未知でそれ以外の場合、`stale_session`で拒否する。

**duplicate照会が対象とするのは現在のsessionだけである。**retired sessionの
`(sid, id)`が履歴に残っていても、それを再生してはならない。遷移を確定できるのは、
retiredでない未知の`sid`による`hello`／`boot`だけである。

- **session切り替えを起こすのは`hello`（Pi側）と`boot`（ESP32側）だけとする。**
- `hello`／`boot`以外のmessageで未知の`sid`を受信した場合、session切り替えを行わず、`stale_session`を返して計数する。
- 直前まで有効だった`sid`を、上限付きの「retired session」集合として保持する。retired sessionからのmessageは`stale_session`で拒否し、duplicate履歴を破棄しない。
- 流量制限のbudgetはsessionではなくlinkに対して持つ。`sid`を変えても受理上限をresetできない。

この規則がない場合、`A → B`へ切り替えた後にsession Aから遅れて届いたmessageが「新しいsession」と誤認され、Bのduplicate履歴を破棄したうえで古い物理commandを再実行しうる。物理動作を伴うprotocolでこれを許容できない。

`hello`が失われた場合、Piのcommandは`stale_session`で拒否される。Piは`stale_session`を受けたら`hello`から再開する。**正しさを`hello`の到達に依存させてよい。到達しなければ、危険側ではなく拒否側へ倒れるためである。**

retired sessionの保持件数と保持期間は`PROTO-TBD-011`とする。保持期間の下限（遅延messageの最大生存時間＋再送window）と、衝突時の`sid`選び直しについては§3.1を参照する。

### 5.2 `set_expression`

```json
{"v":1,"sid":90312,"id":901,"ts_ms":52000,"type":"set_expression","payload":{"name":"happy","transition_ms":300}}
```

初期のexpression名:

- `neutral`
- `happy`
- `surprised`

Firmwareは未知の名前をrejectする。Transition durationにはdisplay実装で定義する上限を設ける。

### 5.3 `play_motion`

```json
{"v":1,"sid":90312,"id":902,"ts_ms":52200,"type":"play_motion","payload":{"name":"nod","speed":0.45,"repeat":1}}
```

Piはraw pulse widthではなく、名前付きの高水準motionを送信する。

Firmwareは次を実行する。

- Motion名を検証する
- 有限の数値入力であることを検証する
- 正規化されたspeedに上限を設ける
- Repeat countに上限を設ける
- Hard angle、velocity、acceleration limitを適用する
- 単位時間あたりの受理数と連続動作時間の上限を適用する
- 要求をrejectするか、clampしたことを報告する

初期に受け入れるmotion名は、サーボ機構のcalibrationが完了するまで`TBD`とする。

上限を超えた場合は、制限の種類に応じたcodeを返し、新しいtrajectoryを開始しない。

**(a) commandを受理しない場合** — 実行中の動作は継続する。

| 超過した制限 | 返すcode |
|---|---|
| 単位時間あたりの受理数 | `rate_limited` |
| 実行中trajectoryによるresourceの一時的な占有 | `busy` |
| 値そのものが許容範囲外 | `out_of_range` |

**(b) 実行時の安全制限を超過した場合** — 実行中のtrajectoryを中止する。

連続動作時間とduty cycleの超過、および拘束・過負荷の検知は、
`busy`で新規commandを断る事象ではない。**実行中の動作そのものが危険源**であり、
[Servo Safety Limits](../hardware/servo-safety-limits.md#拘束stallと過負荷)は
trajectory中止またはPWM disableを要求する。

これらは専用のfault eventで報告する。event名とpayload schemaは`PROTO-TBD-014`で確定する。
schemaは、拘束／過負荷、最大連続動作時間の超過、duty cycle上限の超過を**区別できなければならない**。
原因が判別できないと、Piは再試行してよいのか冷却まで待つのかを決められない。
`busy`へ対応づけてはならない。`busy`は「資源が塞がっているので今は受けられない」であり、
「今動いていること自体が危険」を表さない。

安全要件（何を満たすべきか、検知したら何をするか）は[Servo Safety Limits](../hardware/servo-safety-limits.md)、実測値は[TBD台帳](../hardware/tbd-register.md)を正本とする。Protocolはこの制限が存在することだけを規定し、値は保持しない。

### 5.4 `show_text`

```json
{"v":1,"sid":90312,"id":903,"ts_ms":52300,"type":"show_text","payload":{"text":"なでてくれて、ありがと。","duration_ms":5000}}
```

Firmwareは次に上限を設ける。

- UTF-8 byte length
- Display duration
- Control character
- Line countまたはlayout処理量

Textとface描画の優先順位はUI state machineで定義する。

### 5.5 `show_choices`

```json
{"v":1,"sid":90312,"id":904,"ts_ms":53000,"type":"show_choices","payload":{"prompt":"少し休憩する？","choices":[{"id":"yes","label":"する"},{"id":"later","label":"あとで"}],"timeout_ms":15000}}
```

Choice count、ID、label、prompt length、timeoutに上限を設ける。正確なinteraction eventは、touch hardwareとlayoutの選定まで保留する。

### 5.6 `get_status`

```json
{"v":1,"sid":90312,"id":906,"ts_ms":53500,"type":"get_status","payload":{}}
```

ESP32はcommandへacknowledgeし、`status` snapshotを送信する。

### 5.7 `ping`

```json
{"v":1,"sid":90312,"id":907,"ts_ms":54000,"type":"ping","payload":{}}
```

ESP32はACKを返し、ACK payloadにuptimeを含めてよい。Heartbeatの最終用途とintervalは`TBD`とする。

## 6. Acknowledgement

```json
{"v":1,"sid":41207,"id":905,"ts_ms":53100,"type":"ack","payload":{"reply_sid":90312,"reply_to":903,"status":"ok"}}
```

必須payload:

| Field | Type | 意味 |
|---|---|---|
| `reply_sid` | unsigned integer | Acknowledgeする要求messageの送信session ID |
| `reply_to` | unsigned integer | Acknowledgeする要求message ID |
| `status` | string | `ok`または`rejected` |

ACKは、要求messageの`(sid, id)`を`(reply_sid, reply_to)`へそのまま写す。
ACKを受信した側は、次をすべて確認する。

- Envelopeの`sid`が、現在承認している応答送信側のsessionである。
- `reply_sid`が、自分の現在の送信sessionである。
- `reply_to`が、そのsessionで未完了の要求message IDである。

いずれかが一致しないACKを、現在の要求message（command、`hello`、`boot`）の結果として受理してはならない。
session切り替え後は旧sessionの未ACK commandをtimeout扱いにする。

`reply_sid`がない場合、Pi再起動後に`id`が再利用されると、同じESP32 sessionから
遅れて届いた旧commandのACKを、新しいPi sessionのcommandに対応するACKと誤認できる。
Envelopeの`sid`はACK送信側のsessionであり、要求送信側のsessionを識別しないため、
`reply_to`だけではこのcaseを防げない。

`rejected`の場合はmachine-readableな`code`を含める。診断用に短く上限を設けた`detail`を含めてよい。

ACKは、commandが検証され、処理対象として受け入れられたことを意味する。長時間の物理動作が完了したことは意味しない。

## 7. Error code

初期code:

| Code | 意味 |
|---|---|
| `unsupported_version` | Protocol major versionに未対応 |
| `unknown_type` | Message typeが未知 |
| `invalid_envelope` | 必須fieldまたはtop-level typeが不正 |
| `invalid_payload` | Type固有payloadが不正 |
| `out_of_range` | 上限のある値が許容範囲外 |
| `line_too_long` | 受信lineが設定した最大長を超過 |
| `busy` | 上限のあるresourceがcommandを受け入れられない |
| `hardware_unavailable` | 必要なhardwareの準備が未完了 |
| `duplicate_expired` | 保持履歴から失われたduplicateを安全に再実行できない |
| `rate_limited` | 単位時間あたりの受理上限、session遷移budget／cooldown、または[Servo Safety Limits](../hardware/servo-safety-limits.md)の秒あたり受理motion command数を超過した |
| `stale_session` | 現在のsessionとして承認されていない`sid`のmessageを受信した（retired sessionおよび`hello`／`boot`を経ていない未知の`sid`を含む） |

次の2 codeは、返す条件を個別に定める。

- `line_too_long`は、overflowを検知した行について**`(sid, id)`を復元できた場合にだけ**返す。
  復元は、手順6で保持した上限付きprefixに対して行う。envelope fieldがprefixへ収まらなければ
  復元できない。prefix長は`PROTO-TBD-002`で決め、少なくとも`v`、`type`、`sid`、`id`を
  含みうる大きさとする。
  復元できない場合は`oversize_lines`の計数だけを行い、応答しない。相関できない拒否応答は
  送信側が対応付けられず、§8.2の送出帯域だけを消費する。
  その行が`boot`である場合は、通常のerrorとしてではなく
  **`status: rejected`のACKの`code`として**`line_too_long`を返す。§4.1はACKの
  `status: rejected`だけを終端条件としているため、通常のerrorで返すとESP32は
  終端を判定できずrecovery再送を続ける。復元できない場合は相関ACKを構成できないため、
  §4.1の「復元できない`boot`」の規則に従う。
- `hardware_unavailable`は、対象hardwareの初期化が完了していない状態で受けた
  display／motion commandに返す。`busy`と混同しない。`busy`は初期化済みのresourceが
  一時的に塞がっている状態であり、待てば受け付けられる。`hardware_unavailable`は
  待っても受け付けられない。

Parser counterでは、invalid UTF-8、invalid JSON、invalid envelope、unknown type、oversize line、rate limit超過、session不一致を区別する。

### 単一lineの検証で決まるcodeの対応付け

同じ「不正」でも、どの段階で落ちたかによってcodeが変わる。§12.1のconformance fixtureが次の対応を固定している。

**この表が定めるのは「どのcodeに分類するか」だけである。****そのcodeを実際に相手へ返すかどうかは、この表では決まらない。**送出の可否は方向（§8の拒否時の動作）と、下記`line_too_long`の個別条件、および§8.2の送出上限に従う。とくに次の2つは、分類できても応答しない場合がある。

- `line_too_long`は、下記のとおり`(sid, id)`を復元できた場合にだけ返す。復元できなければ`oversize_lines`の計数だけを行う。
- `unknown_type`は、§3のとおりPi→ESP32でidentityを復元できる場合に相関ACKを返し、ESP32→Piでは応答せず計数だけを行う。

分類と送出を混同すると、応答してはならない場合に拒否応答を送り、§8.2の送出帯域を消費する。

| 事象 | Code |
|---|---|
| top-level typeが不正、envelope必須fieldの欠落、envelope数値が宣言幅・符号・整数性を満たさない、`payload`がobjectでない | `invalid_envelope` |
| `v`が未対応 | `unsupported_version` |
| `type`が未知 | `unknown_type` |
| type固有payloadの必須field欠落・型違い・列挙外の値（例: `reason`が`startup`／`port_reopen`／`resync`以外） | `invalid_payload` |
| 上限のある値が範囲外（string byte長の超過など） | `out_of_range` |
| 改行を含むlineが最大長を超過 | `line_too_long`（送出条件は上記の個別規定に従う） |

**string byte長の超過を`out_of_range`とするのは、この節で新たに決めた分類である。**§5.3は「値そのものが許容範囲外」を`out_of_range`としているが、§5.4のtext byte長のように、上限の存在だけを定めて超過時のcodeを書いていない箇所がある。`invalid_payload`（型と必須fieldの問題）と`out_of_range`（型は正しいが値が上限を超える）を分けることで、送信側は「payloadを直す」のか「値を縮める」のかを区別できる。分けない選択もありえたため、変更する場合は§12.1のfixtureも同時に変える。

判定は§8の手順どおりこの表の上から順に行う。**先に落ちたものが返るcodeを決める。**たとえば未対応`v`と未知`type`を同時に持つlineは`unsupported_version`であり、`unknown_type`ではない（§5.1の手順2が手順3より先である）。

`sid`は乱数を含みうるため、受信側は未知の`sid`が「古い」のか「新しい」のかを値の大小で判定できない。したがって`stale_session`は時系列ではなく、**現在のsessionとして承認されていない**ことを意味する。

## 8. 受信動作

この節と、そこから参照する[§5.1のsession判定規則](#session切り替えはhellobootだけが起こす)は、**受信側がどちらであっても適用する**。§5.1が「PiからESP32へのcommand」の下にあるのは`hello`の説明と一体で書いたためであり、ESP32だけの規則ではない。Piも、ESP32から受け取ったmessageに同じ判定を行う。

拒否したときの**応答方法だけが方向で異なる**。

| 方向 | 拒否時の動作 |
|---|---|
| Pi→ESP32 | `sid`と`id`を復元できるACK-required request（`hello`、command、未知typeを含む）は、`status: rejected`、対応する`code`、`reply_sid`、`reply_to`を持つ相関ACKを返し、`status`のcounterへ計上する。identityを復元できないenvelopeはcounterだけを増やして応答しない。明示的に一方向と定義されたeventも応答しない |
| ESP32→Pi（event） | 応答を返さず、Pi自身のlogへ記録する。eventは要求ではないため、返す先の要求が存在しない |
| ESP32→Pi（`boot`） | **`sid`と`id`を復元できる場合は、拒否するときも`status: rejected`、対応する`code`、`reply_sid`、`reply_to`を持つ相関ACKを返す。**`boot`はACKを要する要求であり、応答しないとESP32が終端を判定できず再送を続ける。**復元できない場合は相関ACKを構成できないため、Pi側の`parse_errors`へ計上して応答しない。**送信側は§4.1のbounded recoveryに従う |
| 方向が逆のsession確立message | ESP32が受けた`boot`、Piが受けた`hello`は`unknown_type`で拒否する。identityを復元できるPi→ESP32の`boot`には上のPi→ESP32規則で相関ACKを返し、ESP32→Piの`hello`には応答しない。**どちらもduplicate照会とsession遷移を行わない**（§8の手順8） |

`status`の`protocol` counterはESP32の観測値である。Pi側の拒否件数はそこに現れない。
Pi側で計数しないと、ESP32の再起動が検知されないまま拒否が続く状態を観測できない。
Piも`parse_errors`、`unknown_types`、`suppressed_responses`をlocal metricとlogへ保持する。
特に、Pi所有のESP32→Pi `boot`保留tableで抑制した応答はPi側の
`suppressed_responses`だけを増やし、ESP32の`status`へ合算しない。

Receiverは次の手順で動作する。

1. 固定上限を持つbufferへbyteを蓄積する。
2. 任意の回数のreadに分割された一つのmessageを処理する。
3. 1回のreadに含まれる複数messageを処理する。
4. 改行受信時に、直前にある任意のcarriage returnを除去する。
5. そのlineの不正なUTF-8またはJSONをrejectする。
6. Overflow時は次の改行まで破棄する。**ただし破棄の前に、上限付きのprefixから`sid`と`id`の復元を試みる。**
   行全体を無条件に捨てると、`line_too_long`を`(sid, id)`付きで返す規則（下記）が成立せず、`boot`が終端応答を得られない。
   prefixの長さは有界とし、`PROTO-TBD-002`に含める。復元できたらその`(sid, id)`で応答し、
   できなければ`oversize_lines`の計数だけを行う。**prefixを保持しても、行の残りは破棄する。**
7. Envelopeと、`type`に対応するpayload schemaを検証する。不正なら**受理budgetを消費せず**に拒否する。応答するかどうかは方向による。Pi→ESP32のACK-required requestと未知typeは、identityを復元できる場合に`(reply_sid, reply_to)`付きの拒否ACKを返し、復元できない場合は計数だけを行う。ESP32→Piでは、未知typeとeventは応答せず計数だけを行い、identityを復元できる`boot`のschema不正には相関ACKを返す（§3、§4.1）。
8. **session権限を判定し、そのうえでduplicate履歴を非破壊で照会する。**未知typeは手順7で`unknown_type`として拒否済みであり、この手順へ到達しない（§3）。
   - 送信元`sid`がretired sessionなら`stale_session`で拒否する。**保持した結果を返さない。**
   - 送信元`sid`が現在のsessionなら、`(sid, id)`が処理済みかを照会する。処理済みなら保持した結果のreplay経路へ進んで終了する。受理budgetもcooldownも消費しない。ただし**非ACKの保持結果を送出すること自体は§8.2の送出上限の対象**とする。保持ACKは§8.2の正規retry quota内で予約容量から送出し、quota超過分は抑制する。
   - 送信元`sid`が未知の場合、`hello`／`boot`は手順9へ進める。それ以外は`stale_session`で拒否する。
   - ただし**方向が逆のsession確立messageは、この判定より前に拒否する。**`hello`はPi→ESP32、`boot`はESP32→Piに限る。ESP32が受けた`boot`、Piが受けた`hello`は、typeとしては既知でもその方向では定義されていない。`unknown_type`で拒否し`unknown_types`を増加させる。**duplicate照会もsession遷移も行わない。**`stale_session`で扱うと、遷移候補として手順9以降へ進み、duplicate履歴の破棄とmotion停止を起こしうる。

   優先順位の詳細は§5.1に記す。**duplicate照会は現在のsessionだけを対象とする。**
9. **未処理のsession確立messageの状態依存条件を検証してから、受理上限を適用する。遷移候補には遷移上限も適用する。**
   - `hello`の`reason`と`sid`の整合を検証する。現在の`sid`なら`port_reopen`／`resync`、異なる未知の`sid`なら`startup`だけを許す。不整合は`invalid_payload`で拒否して最終の拒否結果を保存し、どのbudgetも消費しない。処理済みの`startup hello`再送は手順8で終了しているため、この判定へ到達しない。
   - 現在の`sid`で未処理の新しい`id`を持つ`boot`は`invalid_payload`で拒否し、最終の拒否結果を保存する。正規の`boot`再送は手順8で終了している。
   - 単位時間あたりの受理上限（§8.1）は、上記の検証を通った未処理messageすべてへ適用する。`hello`／`boot`はこの判定に予約枠を使う。
   - **session遷移の上限とcooldown（§5.1）**は、現在のsessionと異なる`sid`の`hello`／`boot`、すなわち遷移候補だけに適用する。受理上限とは別のbudgetであり、予約枠では免除されない。現在の`sid`を維持する`port_reopen`／`resync`は遷移ではないため、このbudgetを消費しない。
   - いずれかの上限超過は`rate_limited`で拒否し、**session state、duplicate履歴、実行中motionのいずれも変更しない。`hello`／`boot`への`rate_limited`は最終結果として保存せず、同じ`(sid, id)`の再送でこの手順を再評価する。**通常commandへの`rate_limited`はそのrequestの最終拒否結果として保存し、再要求する場合はcooldown後に新しい`id`を使う。
10. 上限内であれば、`sid`と`type`に応じて処理する。現在の`sid`で未処理の`port_reopen`／`resync`の`hello`は、sessionを変更せず受理してACKを最終結果として保存する。`hello`／`boot`で`sid`が現在のsessionと異なる場合だけ遷移を確定する。それ以外の未知・retiredな`sid`は`stale_session`で拒否する（§5.1）。
11. 該当counterを増加させる。
12. Resetせず後続lineのparseを続ける。
13. Protocol出力によってsensor、motion safety、watchdogの進行をblockしない。

手順7から10の順序を入れ替えてはならない。それぞれ別の失敗を防いでいる。

| 順序 | 入れ替えると起きること |
|---|---|
| 検証（7）を後回しにする | 不正なmessageが受理budgetを消費する |
| duplicate照会（8）を上限判定より後にする | **飽和時やcooldown中に、処理済みretryが`rate_limited`で拒否される。**処理済み結果のreplay経路へ到達できず、送信側は新規拒否とduplicateを区別できない |
| `reason`と`sid`の状態依存整合（9）をduplicate照会（8）より先にする | 受理済み`startup hello`のACKが失われた再送を「現在sidのstartup」として`invalid_payload`で拒否し、保持ACKをreplayできない |
| session判定（10）を上限判定（9）より先にする | 上限超過で拒否されるはずの`hello`が、拒否前にduplicate履歴の破棄とmotion停止という副作用を残す |
| 遷移budgetを手順9で見ず、手順10で確定してから見る | **予約枠を通った`hello`／`boot`が、遷移上限とcooldownを迂回して遷移を確定できる。**確定後に拒否しても、duplicate履歴の破棄とmotion停止は済んでいる |

手順8の照会は**非破壊**である。履歴を読むだけで、更新も破棄もしない。
同一`(sid, id)`の`hello`／`boot`再送もここで保持したACKのreplay経路へ進むため、
受理budget、遷移budget、cooldownの影響を受けない。実際の送出は§8.2の予約容量から行う。

**受理側を素通りすることと、送出側も無制限であることは別である。**
この経路は受理budgetを消費しないため、処理済みの`(sid, id)`を送り続けられると
非ACKの保持結果が上限なく送出される。送出側の上限は§8.2で規定する。
保持ACKは正規retry quotaと有界な予約容量の両方で制限する（§8.2）。

候補の1024-byte制限には、encode済みobjectと改行を含む。正確なbuffer計算をtestで確認する。

### 8.1 流量制限

Protocolはline長とparse errorに上限を設けているが、それだけではmessage**数**を制限できない。誤動作したhost、あるいはUSB portへ物理accessした第三者が、有効なcommandを高頻度で送り続ける状況を想定する。

Receiverは次を満たす。

- 受理するmessage数に、単位時間あたりの上限を設ける。
- 上限超過時にbufferを無制限に伸ばさず、拒否して計数する。
- 物理動作を起こすcommandには、§5.3の連続動作制限を併用する。
- 上限超過が継続する場合も、parse stateをresetせず、`status`への応答能力を維持する。

#### 適用順序

**受理上限は、session stateを変更する前に適用する。**

`hello`／`boot`はsession遷移を起こし、duplicate履歴の破棄と実行中motionの停止を伴う。上限判定より先に遷移を確定させると、`sid`を変えた`hello`を高頻度で送るだけで、履歴破棄とmotion停止を繰り返し起こせる。上限超過で拒否されるはずのmessageが、拒否される前に副作用を残す。

したがって遷移は次の順で行う。

1. envelopeとtype固有payloadを検証する。不正なら対応するcodeを返し、**session stateを変更しない**。
2. Session権限とduplicateを照会し、処理済みなら保持結果を返して終了する。
3. 未処理messageの`reason`と`sid`など、現在stateに依存する整合を検証する。
4. 受理上限を判定し、遷移候補には遷移budgetとcooldownも判定する。超過していれば
   `rate_limited`を返し、**session stateを変更しない。`hello`／`boot`では拒否を
   最終結果として保存せず、通常commandでは最終拒否結果として保存する**。
   現在の`sid`を維持する`port_reopen`／`resync`は遷移候補ではない。
5. 上記をすべて通った場合にだけ遷移を確定し、履歴破棄とmotion停止を行う。
6. counterを更新する。

Schema検証（1）、duplicate照会（2）、state依存の整合検証（3）、上限判定（4）は
いずれも遷移確定（5）より前に行う。順序を入れ替えると、保持結果を返すべきretryを
拒否するか、拒否されるはずのmessageが拒否前に副作用を残す。

#### 制御messageの帯域確保

link単位のbudgetをすべて通常commandが消費すると、再接続に必要な`hello`／`boot`自身がrate limitされる。その状態では後続commandが`stale_session`で拒否され続け、**session確立と拒否が互いを維持する膠着**に陥る。

`hello`と`boot`のために、受理budgetの一部を予約するか、優先度を与える。予約分は通常commandが消費できない。予約割合は`PROTO-TBD-012`で扱う。

予約割合と受理上限はlinkの負荷管理parameterであり、protocolの負荷試験で決める。温度／電流試験では決まらない。`PROTO-TBD-012`で追跡する。

servoの秒あたり受理motion command数の値は[TBD台帳](../hardware/tbd-register.md)を正本とする（要件は[Servo Safety Limits](../hardware/servo-safety-limits.md)）。link全体のbudgetは、その値を下回ってはならない。

### 8.2 送出応答の流量制限

受信側の上限だけでは不十分である。すべての超過messageへ`busy`／`rate_limited`／`stale_session`を返すと、**1行の入力が1行の応答を強制する**。有効な行を高頻度で送り続けられた場合、拒否応答自体がlinkを占有し、`status`やeventの送出を枯渇させる。

同じことがduplicateへの応答でも起きる。手順8の照会は受理budgetもcooldownも消費しないため、処理済みの`(sid, id)`を送り続けられれば、保持結果が上限なく送出される。**拒否応答だけを絞っても、この経路が残れば枯渇は防げない。**

送信側にも次の上限を設ける。

- 単位時間あたりに送出する応答の総数に上限を設ける。初回応答と正規retryへの応答を
  優先するが、同じidentityへの応答を無制限に上限外へ置かない。
- **受理済みcommandへの最初のACK、完了event、fault event、およびsession確立message
  （`hello`／`boot`）の送出と、その最初の`status: ok`のACKは抑制しない。**送出順序で
  これらを最優先とし、必要な容量を予約する。物理動作を実行したあとに最初のACKを
  抑制すると、送信側はtimeoutして同じcommandをretryする。
- 保持ACKにはidentityごとの**正規retry quota**を設ける。通常commandは§9で許可する
  1回、`hello`は`PROTO-TBD-011`で定める同一identityの最大retry回数、`boot`は
  `PROTO-TBD-017`の同一identity最大retry回数をquotaとする。
- quotaが残るduplicateには、送出総数上限と通常のper-identity上限に優先する予約容量から
  保持ACKを返す。送達確認は無いため、wireへ渡した時点でquotaを一つ減らす。
  予約容量不足などでwireへ渡せなかった試行では減らさない。quotaを使い切った後の反復は抑制し、受信側のlocal
  `suppressed_responses`を増やす。
- `hello`／`boot`への`status: rejected`は、方向別保留tableのentryへ、初回送出機会と
  上記session messageの正規retry回数を合わせた有限の送出quotaを持たせる。tableへ
  受け付けたentryはquota内で専用budgetから個別に返す。table上限で受け付けなかった
  identityとquota超過分には応答しない。
- 抑制または喪失した場合、送信側は現在の`sid`のまま有限budget内で再送し、満了で
  `protocol_fault`として停止する（§3.1）。`sid`を選び直すのは明示的な
  `stale_session`を受けたときだけであり、ACKの不在は契機にならない。
- 要求identityを参照しない同一codeの連続した**非ACK拒否**は、受信側とcodeが同じ場合に限り、個別に返さず1件へ集約してよい。
- **ACK-required requestへの`status: rejected`は、`hello`／command／未知type／`boot`のいずれも集約しない。**ACKは`reply_to`を1つしか持たず、1件で複数の`(sid, id)`を終端できない。session確立messageは方向別保留tableと専用budget、その他は一般の送出上限で抑え、送出しなかった受信機会は受信側のlocal `suppressed_responses`で計数する。
- 同一`(sid, id)`へ**非ACKの**保持結果をreplayする回数には、**時間窓ごとの上限**を設ける。per-identity上限は§9の正規retry 1回を下回らない。応答の送出総数上限を優先し、いずれかの上限超過分は抑制して`suppressed_responses`を増やす。
- 予約容量は、保持中の各identityに残る正規retry quotaと、各方向で同じ時間窓に到着しうる
  正規retry数から算出する。保持件数、quota、時間窓をすべて有限にし、予約容量だけを
  無制限にしない。`boot`分は`PROTO-TBD-017`、`hello`分は`PROTO-TBD-011`の最大retry回数を
  使い、通常commandの1回分と混同しない。
- quota内のACKを優先する理由は、抑制がそのままduplicateを生むためである。一方、quotaを
  超えた反復は正規senderの契約外であり、応答し続けると一つのidentityで帯域を枯渇できる。
  正規retryにもACKが無ければ、通常commandは結果を**終端的に不明**として扱い、session
  messageは各有限recovery budgetの終了規則に従う。
- **非ACKの保持結果（完了eventのreplayなど）と拒否応答には、従来どおり上限を適用する。**超過分は抑制して`suppressed_responses`を増やす。
- 上限を超えたduplicateは応答を抑制するだけとし、**受理budget、遷移budget、cooldownのいずれも消費しない**。手順8の非破壊性、session stateを変えない性質、非idempotentな動作を再実行しない性質を変えない。
- ESP32が抑制した件数は`status`のcounter、Piが抑制した件数はPi側のlocal metricとlogで報告し、黙って捨てたことが分からない状態にしない。
- `status`と安全に関わるeventのために、送出帯域の一部を確保する。
- 応答の抑制によって、sensor処理、motion safety、watchdogの進行をblockしない。

上限値、集約window、duplicateへ非ACKの保持結果をreplayする時間窓と回数の上限、
**正規retry quota内の保持ACK replay予約容量**、ACK・完了event・fault event・`status`へ
確保する帯域の割合は`PROTO-TBD-012`で扱う。

なおUSB serialには認証がない。物理accessを得た相手はPiと同等のcommandを送れる。**安全境界はprotocolではなくESP32側のhard limitで担保する**という前提を、実装で崩さない。

## 9. Retryとduplicate処理

Draft 2のpolicy:

- ACKが必要なcommandはtimeout後に1回retryする。
- retry後もACKが無ければ実行結果を**不明**として扱う。同じ非idempotent commandを新しい`id`で再実行せず、linkを安全側へ倒して診断を報告する。sessionが有効なら`get_status`で実stateを再取得してよい。
- `boot`はこの1回制限の対象外とする。session確立の唯一の経路であり、諦めると復旧できない。再送方針は§4.1に従う。
- `hello`／`boot`が明示的な`rate_limited`を受けた場合は、ACK timeoutではなく各節の
  retryable rejection規則に従い、cooldown後に同じ`(sid, id)`で再送する。
- 通常commandが明示的な`rate_limited`を受けた場合、そのrequestは最終的に拒否された
  ものとして扱う。同じ`(sid, id)`をretryせず、cooldown後に改めて要求する場合は
  新しい`id`を割り当てる。ACKが無い場合の1回retryとは区別する。
- 同じ`(sid, id)`を再利用する。
- ESP32は直近に処理した`(sid, id)`とresultを保持する。
- Duplicateには、要求の`(sid, id)`を`(reply_sid, reply_to)`に保持したresultのreplay経路を使い、
  非idempotentな動作を再実行しない。実際の送出には§8.2の上限を適用する。
  **保持ACKは§8.2の正規retry quotaが残る間、予約容量から送出する。**通常commandの
  quotaは1回であり、そのretryにも無応答なら上の規則どおり不明として停止する。
  quotaを超える反復には応答せず、動作も再実行しない。
- **Duplicate判定は必ず`(sid, id)`の組で行う。`id`だけで判定してはならない。**
- duplicate履歴を破棄するのは、**異なる`sid`の`hello`／`boot`によるsession遷移を確定したときだけ**とする（§5.1）。破棄の対象は**現行session分だけ**であり、保持期間内のretired session分は残す。同一`sid`の`hello`／`boot`再送では破棄せず、保持したACKのreplay経路へ進む（§8.2の正規retry quota内で予約容量から送出する）。`sid`の変化を見ただけでも破棄しない。
- retired sessionからのmessageは`stale_session`で拒否し、履歴を破棄しない。

受け入れ前の`TBD`:

- ACK timeout
- 保持件数と期間
- Integer wrapの処理
- Duplicateが保持履歴より古い場合の動作
- ACKを必要とするmessage
- `sid`の生成方法と衝突許容確率

状態設定commandはidempotentにする。Relativeまたは名前付きの物理motionにはduplicate suppressionが必要である。

`id`だけでduplicateを判定する実装は、送信側の再起動後に**新しいcommandを実行せず`ok`を返す**。この失敗はACKが正常に返るため、host側からは成功と区別できない。conformance fixtureで必ず検出する（§12）。

## 10. Reconnect

### 10.1 ESP32が再起動した場合

1. ESP32が**再起動**し、新しい`sid`で`boot`を送信する。
2. Piが`boot`の`sid`を新しいESP32 sessionとして記録し、旧sessionのID追跡を破棄する。
3. Piが`(reply_sid, reply_to)`にその`boot`の`(sid, id)`を写したACKを返す（§4.1）。
4. Piが`get_status`を送信する。
5. ESP32がACKと`status`を送信する。
6. Piが実際のdisplay／motion stateとdesired stateを比較する。
7. Piが安全な状態設定commandを送信する。
8. どちらも古いrelative motionを自動再実行しない。

serial linkが切れて繋がり直しただけで、ESP32 processが再起動していない場合はこの手順に入らない。`sid`を変えず、`boot`も送らない（§3）。Piは現在のESP32 sessionとduplicate履歴をそのまま保持する。link断を再起動と誤って扱うと、動作中のmotionを不要に停止し、retryを二重実行に変える。

### 10.2 Piが再起動した場合

1. Piがserial portを開き、新しい`sid`で`hello`を送信する（processの再起動なので`sid`を変える。link再接続だけの場合は`sid`を維持する。§3）。
2. ESP32が旧Pi sessionのcommand ID追跡とduplicate履歴を破棄し、旧`sid`をretired session集合へ移す。retired分は保持期間中は捨てない（§5.1）。
3. ESP32が実行中のrelative motionを安全に停止する。
4. ESP32がACKを返す。
5. Piが`get_status`を送信し、実stateを取得する。
6. Piが安全な状態設定commandで desired state を再構成する。

`hello`が失われた場合、ESP32は後続commandを`stale_session`で拒否する。Piは`stale_session`を受けたら`hello`から再開する。未知の`sid`を見ただけで切り替えてはならない（§5.1）。

### 10.3 通信断

物理linkの断とPiの停止を、ESP32はprotocol上区別できない。いずれの場合も、承認済みのサーボfail-safe動作へ移行する。検知方式（heartbeat sourceとloss timeout）は`PROTO-TBD-010`および[HW-TBD-017](../hardware/tbd-register.md)で確定する。断を検知した後に取る動作（fail-safe sequence）は[HW-TBD-018](../hardware/tbd-register.md)で確定する。**検知だけでは足りない。**検知できても取るべき動作が未定なら、断のあとサーボは制御されない状態で残る。

サーボ出力の有効化条件の正本は[Servo Safety Limits](../hardware/servo-safety-limits.md#サーボ出力を有効化してよい条件)であり、ここに挙げた条件はその一部である。Protocolは条件の全体を再掲しない。

## 11. Versioning

- `v`はwire protocolのmajor versionである。
- Backward-compatibleなoptional fieldの追加では`v`を変更しない。
- 必須fieldの削除、改名、意味またはtypeの変更には新しいmajor versionが必要である。
- 両側は未対応のmajor versionを明示的にrejectする。
- Firmwareとhostのrelease noteに対応protocol versionを記載する。

最初の実装は`v: 1`を使用するが、fixtureと両側のparserがconformance testに合格するまでschemaは`Draft`とする。

## 12. 必須fixtureとtest

次の共有fixture fileを作成する。

- 受け入れるすべてのmessage type
- 最小・最大の境界値
- すべてのbyte境界で分割したmessage
- 1回のreadに含まれる複数message
- CRLF入力
- Invalid UTF-8
- Invalid JSON
- Field不足
- 不正なfield type
- 未知のversion
- 未知のtype
- 最大値と同じ長さのline
- 最大値を1 byte超えるline
- Duplicate command
- 同じ`(sid, id)`によるretry

Session境界のfixtureは、遷移の有無で期待結果が逆になる。setupと期待値を分けて記述する。

| Fixture setup | 期待する結果 |
|---|---|
| 有効な`hello`で遷移した後、新`sid`で`id`が旧sessionと重複するcommand | 新規commandとして実行する |
| `hello`／`boot`を経ずに`sid`だけが変化したcommand | `stale_session`で拒否し、履歴を破棄しない |
| retired sessionの`sid`からのmessage | `stale_session`で拒否する |
| retired sessionで**以前処理済みだった**`(sid, id)` | retired `sid`との一致だけで`stale_session`として拒否する。以前の個別結果を保持・返却しない |
| 同一sessionでの`hello`再送 | 履歴を破棄せず、保持したACKのreplay経路へ進む。保持ACKは`PROTO-TBD-011`の正規retry quota内で§8.2の予約容量から送出する |
| 同一sessionでの`boot`再送 | Pi側のID追跡を破棄せず、保持したACKのreplay経路へ進む。保持ACKは`PROTO-TBD-017`の正規retry quota内で§8.2の予約容量から送出する |
| `hello`再送直後の非idempotent commandのretry | 二重実行しない |
| ACK必須の非idempotent commandがtimeoutした（`boot`以外） | 同一payload・同一`(sid, id)`で**1回だけ**再送する。2回目の再送を行わない（§9） |
| その1回の再送にもACKが無い | 結果を**終端的に不明**として扱い、そこで停止する。安全側へ倒して診断を報告する |
| 不明と判定した後に送信側が`get_status`を送る | 実stateの照会は行えるが、**その応答を根拠に自動再送しない**。新しい`(sid, id)`での自動再実行を発生させない |
| 不明と判定した非idempotent commandを、別の明示的な要求として再発行 | 新しい`(sid, id)`の新規commandとして扱う。retryではないため1回制限を引き継がない |
| 通常枠と予約枠を**どちらも**使い切った状態での`hello` | `rate_limited`で拒否し、session stateを変更しない |
| 未対応`v`の`hello` | `unsupported_version`で拒否し、履歴を破棄しない |
| payloadが不正な`hello`（`reason`が列挙外） | `invalid_payload`で拒否し、履歴を破棄しない。保留tableに空きがあれば受け付け、`(reply_sid, reply_to)`付きの`status: rejected`を少なくとも1回返す |
| `invalid_payload`／`unsupported_version`で拒否された`hello` | 送信側は**終端**として`hello`の送出を止め、`protocol_fault`で報告する。`sid`を選び直さず、motion commandも送らない（§5.1） |
| `rate_limited`で拒否された`hello` | 終端ではない。cooldown経過後に**同じ`(sid, id)`で**再送する。`sid`を選び直さない |
| `hello`／`boot`が飽和中に`rate_limited`で拒否され、同じ`(sid, id)`でcooldown後に再送 | 最初の拒否を最終結果としてreplayせず、手順9でbudgetを再評価する。上限内なら同じentryの未送出拒否ACKを取り消して受理し、`status: ok`を最終結果として保存する。古い`rate_limited`を後から送らない |
| 通常commandが`rate_limited`で拒否された後に同じ`(sid, id)`を再送 | 最終の拒否結果をreplayし、commandを実行しない。cooldown後に再要求する場合は新しい`id`を割り当てる |
| 保留済み`hello`の拒否ACKが抑制または喪失した | 送信側は`sid`を選び直さず、同じ`(sid, id)`で有限budget内に再送する。受信側は保留tableから拒否理由を返す。budget満了で`protocol_fault`として停止する |
| ESP32側の保留tableが満杯のときに新しいidentityの不正`hello`を受信 | 新規entryを受け付けず応答しない。`suppressed_responses`を増やす。送信側は有限budget満了で停止し、`sid`を選び直さない |
| 保留tableへ受け付けた同一`(sid, id)`の不正な`hello`を反復 | 初回送出機会と`PROTO-TBD-011`の最大retry回数を合わせたquota内は拒否ACKを返す。quota超過分は抑制して`suppressed_responses`を増やす |
| 異なる`id`の不正な`hello`を送り続ける | ESP32が所有するPi→ESP32方向の保留table上限で新規identityの受け付けを止め、`suppressed_responses`を増やす。Pi側の`boot` tableへ枠を借りない |
| 不正な`hello`と`boot`を両方向で交互に送り続ける | 各受信側が方向別の件数上限と送出budgetを独立に強制し、保持entryの合計が`PROTO-TBD-012`で定める二つの上限の和を超えない。どちらかの未使用枠で他方向の超過を受理しない |
| `rate_limited`で拒否された`hello`をretry budgetまで再送 | budget満了後は終端として`protocol_fault`で報告する。cooldown経過を理由に無期限へ再送しない |
| `sid`または`id`を復元できない`hello` | 相関ACKを構成せず`parse_errors`を増やす。送信側は§3.1の有限budgetで送出を止め、`protocol_fault`で報告し、sessionを確立しないままmotion commandを送らない |
| `sid`を変えた`hello`の連続送信 | 遷移上限とcooldownにより`rate_limited`で拒否される |
| 通常枠だけを使い切り、予約枠が残っている状態での`hello` | 予約分により受理される |
| 受理上限に達した状態での処理済みcommandの正規retry | `rate_limited`にせず保持結果のreplay経路へ進む。保持ACKは残る正規retry quota内で§8.2の予約容量から送出し、非ACKの保持結果は§8.2の上限に従う |
| 遷移cooldown中の同一`(sid, id)`の`hello`正規retry | cooldownを消費せず保持ACKのreplay経路へ進む。保持ACKは残る`hello` retry quota内で§8.2の予約容量から送出する |
| 現在のcommandと`reply_to`は同じだが、`reply_sid`が旧sessionのACK | 現在のcommandのACKとして受理せず、現在のcommandは正しいACKを待つかtimeoutする |
| `reply_sid`と`reply_to`が現在の未完了commandに一致し、envelopeの`sid`も現在の相手sessionであるACK | 対応するcommandのACKとして受理する |
| retiredな`sid`かつ未知typeのmessage | `unknown_type`で拒否する（手順7が先）。`stale_sessions`は増やさず`unknown_types`を増やす |
| 新しい`sid`がretired sessionと衝突した`hello`／`boot` | `stale_session`で拒否する。送信側は新しい`sid`を選び直し、`id`を初期値へ戻して再送する。同じ`sid`で再送しない |
| `hello`への`stale_session`が送出上限で抑制されるか喪失する | 送信側は同じ`(sid, id)`で再送し、受信側は保存された`stale_session`を保留tableから返す。**明示的に`stale_session`を受信してから**`sid`を選び直す。無応答のままなら選び直さず、budget満了で停止する |
| 処理済みの`reason: startup`の`hello`のACKだけが失われ、送信側が現在の`sid`と同一`(sid, id)`で正規retry | `reason`／`sid`の不整合判定より先にduplicateを照会し、残るquota内で保持ACKを§8.2の予約容量から返す。2回目の遷移、duplicate履歴の再破棄、motion停止は起きない |
| `sid`選び直しが上限に達した状態 | `protocol_fault`で報告し、session確立messageの自動送出を停止する。運用者の明示的なsession resetまたはprocess再起動まで再開せず、motion commandを送らない |
| 保持件数の上限に達するまでsession遷移を繰り返した状態での、保持期間内の旧`sid`からの遅延message | `stale_session`で拒否する。保持期間の満了前に追い出さない |
| 保持期間内に上限を超える速度でsession遷移を要求 | `rate_limited`で拒否する。retired sessionを追い出して枠を空けない |
| retired保持期間**内**に届いた旧sessionからの遅延message | `stale_session`で拒否する。保持期間は遅延messageの最大生存時間＋再送windowを下回らないため、この期間を過ぎた遅延messageは前提上存在しない |
| Piが受信した`boot`／ESP32が受信した`hello`（方向が逆） | `unknown_type`で拒否する。duplicate照会もsession遷移も行わず、duplicate履歴を破棄しない |
| `reason`が`port_reopen`／`resync`で`sid`が現在と同じ、未処理の`hello` | 通常の受理上限内なら受理し、`status: ok`のACKを最終結果として保存する。session遷移budgetとcooldownを消費せず、duplicate履歴の破棄とmotion停止を行わない |
| 現在の`sid`で未処理の新しい`id`を持つ`boot` | `invalid_payload`のACKで拒否し、最終の拒否結果を保存する。session state、duplicate履歴、motionを変更しない。正規の`boot`再送は同じ`(sid, id)`なのでこのcaseには入らない |
| 最初の`boot`がPiへ届かず、ESP32が同一`(sid, id)`で再送 | Piは未処理のため、**この再送で1回だけsession遷移を確定**し、旧ESP32 sessionのID追跡を破棄してACKを返す |
| Piが`boot`を処理した後にACKが失われ、ESP32が同一`(sid, id)`で正規retry | **2回目の遷移を行わず**、残る`boot` retry quota内で§8.2の予約容量から保持したACKを返す |
| `boot`再送が通常回数に達した状態 | 直ちには止めず、recovery間隔まで延ばして有限のrecovery budget内で継続する |
| `boot`が無応答のままrecovery budgetに達した状態 | 送出を止め、サーボ出力を有効にせず`protocol_fault`を報告し、motion commandを`stale_session`で拒否する。有効な`hello`を受信したら**同じ`(sid, id)`のまま**有限budgetを再生成する。**`sid`は選び直さない**（§3、§4.1） |
| 停止後に同一Pi sessionから`hello`を反復 | budgetの再生成は**1つのPi sessionにつき1回**まで。2通目以降では再生成せず、`boot`の送出も再開しない |
| `rate_limited`で拒否された`boot` | 終端ではない。cooldown経過後に同じ`(sid, id)`で再送し、`PROTO-TBD-017`の有限budget内に収める。使い切ったら`protocol_fault`で報告して停止する |
| 同一`(sid, id)`の`boot`が最大再送回数まで再送され、そのつど保持ACKを要求する | `PROTO-TBD-017`のquota内は予約容量から返す。quotaを超える反復は抑制し、Pi側のlocal `suppressed_responses`を増やす |
| `reason`が`port_reopen`／`resync`の`hello`が無応答 | `sid`を選び直さず、同じ`(sid, id)`で有限budget内に再送する。使い切ったら`protocol_fault`で報告する。`reason`と`sid`が矛盾する`hello`を作らない |
| `port_reopen`／`resync`のbudgetが満了した後 | `reason`を`startup`へ自動で切り替えない。`sid`を保ったまま停止状態に留まる。`startup`での再開はprocess再起動または運用者の明示的なsession reset指示に限る（§3.1） |
| 停止状態でPiが`resync`の`hello`または通常のcommandを送る | `sid`を変えず、duplicate履歴も破棄しない。`resync`はstate再取得の要求であってsession resetではない（§7）。新しいESP32 sessionが要るならprocess再起動か§3.1の衝突回復による |
| 不正payloadの`boot` | Piは`status: rejected`のACKを返す。ESP32は再送を終了し`protocol_fault`で報告する |
| `sid`または`id`を復元できない`boot` | Piは`parse_errors`を増やして応答しない。ESP32は有限のrecovery budgetで再送を止め、サーボ出力を有効にせず`protocol_fault`で報告する |
| 異なる`id`の不正な`boot`が連続 | 拒否ACKは集約せず、Pi側の専用budgetの範囲で個別に返す。budget超過分は応答せずPi側のlocal `suppressed_responses`を増やす。Pi→ESP32方向のcommand／session確立messageとESP32側の受信処理を保護する |
| budgetを使い切った状態での同一`(sid, id)`の不正`boot`再送 | **保留entryがTTL内である限り**、budget回復後に少なくとも1回は拒否ACKを返す。TTL満了後は下の規則どおり応答義務を終える |
| 拒否budgetを使い切った状態で、異なる`id`の不正な`boot`を送り続ける | Piが所有するESP32→Pi方向の保留table上限で新規identityの受け付けを止め、`suppressed_responses`を増やす。保留件数が無制限に増えない |
| 保留entryのTTL満了 | 最悪送出待ち時間の範囲で少なくとも1回の送出機会を与えた後にentryを破棄し、その`(sid, id)`への応答義務を終える。送信側は`PROTO-TBD-017`のbudget満了で再送を止める |
| 未対応`v`の`boot` | 同上（`unsupported_version`）。無応答で放置しない |
| 処理済みの同一`(sid, id)`へ**非ACK**の保持結果を上限より多く反復 | §9の正規retry分は返し、同じ時間窓の超過分は抑制して`suppressed_responses`を増やす |
| 処理済みの同一`(sid, id)`へ正規retry quotaを超えて保持ACKを要求 | quota超過分を抑制して受信側のlocal `suppressed_responses`を増やす。動作とsession遷移は再実行しない |
| `hello`／`boot`の保持ACK replayが通常の時間窓上限に達した状態で、quota内の正規retry | 予約容量から保持ACKを返す。通常上限の超過だけを理由にquota内応答を抑制しない |
| 応答の送出総数上限に達した状態での処理済みcommandの正規retry | 動作を再実行せず、残るquota内で**保持ACKを予約容量から返す**。quota超過分と非ACKの保持結果は抑制し、その分を`suppressed_responses`へ計上する |
| 保持ACKを返せないまま送信側が1回のretryを終えた場合 | 送信側は結果を終端的に不明として扱い、安全側へ倒して診断を報告する。**この状態を上限の運用で作らない**ことが上の予約容量の目的である |

`sid`は乱数を含みうるため、fixtureでも「古い`sid`」ではなく「retiredまたは未承認の`sid`」として記述する。値の大小に新旧の意味はない。

- 拒否応答が連続する状況での`status`送出
- 受理上限を超える連続command
- Reconnectとstatus同期

環境が許す範囲で、hostとfirmwareの両実装が同じfixtureを使用しなければならない。

### 12.1 fixtureの所在

fixture fileは次にある。

```text
crates/deskcat-protocol/tests/fixtures/
```

形式と追加規則はそのdirectoryのREADMEを参照する。**期待値をRust側のtest codeへ埋め込まず、
言語に依存しないJSON fileとして持つ。**[ADR-0001](../decisions/0001-monorepo-layout.md)が
「コードを共有するかどうかにかかわらず、両側が共通JSON fixtureとprotocol conformance testに
合格しなければならない」と定めているためである。同じdirectoryをfirmware側からpath参照するか
複製するかは、同ADRのcrate共有方針の決定に従う。

fixtureは最低限、次の5群をすべて含む。上の一覧と表がその内訳である。**現時点で揃っているのはSchema群と、Framing／parse群のうちline長境界・CRLF・invalid JSONだけである。**

| 群 | 対象 | 状態 |
|---|---|---|
| Schema | envelope、type固有payload、上限の境界、未知version／type | **作成済み**（#9） |
| Framing／parse | 分割受信、CRLF、invalid UTF-8／JSON、line長境界 | line長境界・CRLF・invalid JSONは**作成済み**（#9）。byte単位の分割受信とinvalid UTF-8は#10 |
| Session判定 | 遷移の成否、`stale_session`、retired session、`hello`／`boot`再送、`sid`衝突時の選び直し、retired保持期間、方向が逆のsession確立messageの拒否 | 未作成（#12） |
| Duplicate replay | 同一`(sid, id)`のretry、保持結果の返却、非idempotent動作の二重実行防止 | 未作成（#12） |
| Budgetと応答 | 受理上限、遷移budget／cooldown、予約枠、送出上限、ACKの優先 | 未作成（#12）。値が`PROTO-TBD-011`／`012`／`017`で未確定 |

**未作成の3群は、単一lineのschema検証ではなく受信側のstateに依存する。**#9が提供するのは
stateを持たない単一lineのdecodeであり、これらのcaseを置いても実行できない。budget群は
parameterの数値自体が未確定である。

したがって**この節はまだ「両側が検証済み」の根拠にならない。**現時点で確定したのは、
schema層についてhost実装がfixtureに合格することだけである。firmware側の合格は#10、
残る3群は#12を待つ。`v: 1`が確定版を意味するのは、4群すべてが揃い両側のparserが
合格した時点である（[§11](#11-versioning)）。

作成作業は[初期Issue](../backlog/initial-issues.md)の#9が着手した。
`PROTO-TBD-016`はprotocol側の未決事項としての追跡であり、
実施Issueを別に立てるものではない。

## 13. 未決定事項

| ID | 判断 | 必要な根拠 |
|---|---|---|
| PROTO-TBD-001 | 最終baud | Pi／ESP32のthroughput・安定性test。[HW-TBD-014](../hardware/tbd-register.md)と対で確定する。値はProtocol側、実機transport testの実施責任はhardware台帳 |
| PROTO-TBD-002 | 最終最大line byte数、および**overflow時にidentity復元のため保持するprefixのbyte数**（`v`、`type`、`sid`、`id`を含みうる大きさ。行長上限より十分小さいこと） | Worst-caseの上限付きpayloadとmemory test。[HW-TBD-014](../hardware/tbd-register.md)と対で確定する。値はProtocol側、実機transport testの実施責任はhardware台帳 |
| PROTO-TBD-003 | ~~Integer width~~（§3で確定。`v`=`u16`、`sid`／`id`=`u32`、`ts_ms`=`u64`）と、**送信側が`id`の上限に達したときの動作** | 残るのはwrap時の運用だけである。session確立とduplicate履歴の扱いに関わるため、host serial session（[#11](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/11)）とACK／reconnect実装（[#12](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/12)）で決める |
| PROTO-TBD-004 | ACK timeout | 測定latencyとrecovery test |
| PROTO-TBD-005 | **現在のsession**のduplicate履歴の保持期間とretry window、および**保持件数の上限と超過時の動作**。期間の下限は遅延messageの最大生存時間＋再送windowを下回らない。件数上限は受理budget（§8.2）と保持する結果の最大sizeから導出する。上限超過時は最も古いentryをevictしてよいが、evictしたentryへの再送は新規commandとして実行しない（`duplicate_expired`で拒否する。§9参照）。`PROTO-TBD-011`のretired session保持期間から導出しない（目的の異なる別モデル。§5.1） | Memory予算とretry window。[HW-TBD-020](../hardware/tbd-register.md)と対で確定する。サーボ出力の有効化条件に含まれる |
| PROTO-TBD-006 | 最終status field | 診断要件とencode size test |
| PROTO-TBD-007 | Textとchoiceの制限 | LCD layoutとmemory測定 |
| PROTO-TBD-008 | Motion名と範囲 | Servo calibrationと動作設計 |
| PROTO-TBD-009 | Touch strengthの意味 | 正確なtouch controllerと実験 |
| PROTO-TBD-010 | Heartbeat方式とlink-loss判定 | 測定latencyとfail-safe試験。[HW-TBD-017](../hardware/tbd-register.md)と対で確定する。**サーボ出力の有効化条件に含まれる**（[servo-safety-limits](../hardware/servo-safety-limits.md#サーボ出力を有効化してよい条件)） |
| PROTO-TBD-011 | `sid`の生成方法、衝突許容確率、**retired session**の保持件数と期間（下限は遅延messageの最大生存時間＋再送window。期間は`T_retention`と時間単位を一組で記録する。保持件数は`PROTO-TBD-012`の`N_transition`回／`T_window`から`N_transition × ceil(T_retention / T_window)`件以上とし、端数windowを切り上げる。retired `sid`を`stale_session`で遮蔽するためのものであり、`PROTO-TBD-005`とは目的が異なる）、`stale_session`受信による`sid`選び直し回数の上限、`hello`無応答時のrecovery budgetと同一identityの最大retry回数（`boot`のrecovery再開は`sid`を選び直さない。§4.1） | 再起動試験とRust実装の検討。[HW-TBD-020](../hardware/tbd-register.md)と対で確定する。**サーボ出力の有効化条件に含まれる** |
| PROTO-TBD-012 | 単位時間あたりの受理上限、応答の送出上限と集約window、**`boot`への拒否ACK専用budget**、**`hello`への拒否ACK専用budget**、**受信側が所有する方向別応答保留table**（ESP32所有のPi→ESP32 `hello` tableと、Pi所有のESP32→Pi `boot` table）の件数上限・TTL・公平性規則、二つの件数上限の和として定義するlink全体の静的上限（entry keyは`(sender_role, sid, id)`。方向間で未使用枠を貸し出さない。TTLは最悪送出待ち時間以上。`rate_limited`は最終結果としてreplayしない）、各保留entryの初回送出機会とsession messageの最大retry回数を合わせた有限送出quota、**`rate_limited`で拒否された`hello`のretry budget**、duplicateへ非ACKの保持結果をreplayする時間窓と回数の上限、**正規retry quota内の保持ACK replay予約容量**（送出総数上限と通常のper-identity上限に優先。通常commandは1回、`hello`は`PROTO-TBD-011`、`boot`は`PROTO-TBD-017`の最大retry回数を使い、各identityの残quotaと同一windowの最大retry到着数から有限容量を算出する）、`hello`／`boot`の受理予約枠割合、session遷移の上限（任意の連続`T_window`あたり`N_transition`回。数値と時間単位を一組で記録し、固定window境界で上限を迂回できない方式にしてPROTO-TBD-011の保持件数式へ渡す）、cooldown、ACK・完了event・fault event・`status`用に確保する帯域 | protocolの負荷試験（throughput、応答遅延、buffer占有、枯渇の有無）。これらはlinkの負荷管理parameterであり、温度／電流試験では決まらない。[HW-TBD-020](../hardware/tbd-register.md)のservoの秒あたり受理command数と対で確定する。その値はhardware台帳が正本であり、ここではlink全体のbudgetへ組み込む条件だけを扱う。**サーボ出力の有効化条件に含まれる**（[servo-safety-limits](../hardware/servo-safety-limits.md#サーボ出力を有効化してよい条件)） |
| PROTO-TBD-013 | Stale commandの拒否条件（command age、session遷移後の未ACK commandの扱い） | Reconnect試験とfail-safe試験。[HW-TBD-018](../hardware/tbd-register.md)の通信断時fail-safe／reconnect条件、および[HW-TBD-020](../hardware/tbd-register.md)のCommand timeout fieldと対で確定する。Command timeoutの実測値はhardware側、stale commandの拒否条件はProtocol側を正とする。**サーボ出力の有効化条件に含まれる**（[servo-safety-limits](../hardware/servo-safety-limits.md#サーボ出力を有効化してよい条件)） |
| PROTO-TBD-014 | 実行時安全制限の超過を報告するfault eventの名前とpayload schema。拘束／過負荷、最大連続動作時間の超過、duty cycle上限の超過を、payload fieldまたはcodeで**区別できる**こと | [Servo Safety Limits](../hardware/servo-safety-limits.md#拘束stallと過負荷)の検知手段確定後。[HW-TBD-020](../hardware/tbd-register.md)と対で確定する。**サーボ出力の有効化条件に含まれる**（[servo-safety-limits](../hardware/servo-safety-limits.md#サーボ出力を有効化してよい条件)） |
| PROTO-TBD-015 | Draft schema revisionの表明方法（envelope fieldかout-of-band照合か） | Draft間の相互接続が必要になった時点。fixture一致で足りるなら追加しない |
| PROTO-TBD-016 | §12のconformance fixtureの実体作成と配置。**schema群と一部のframing群は`crates/deskcat-protocol/tests/fixtures/`へ配置済み**（[#9](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/9)）。残るのはsession判定、duplicate replay、budgetと応答の3群と、firmware側の合格確認 | 残る3群は受信側のstateを必要とするため[#12](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/12)、firmware側の合格は[#10](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/10)で確認する。budget群は`PROTO-TBD-011`／`012`／`017`の確定を待つ |
| PROTO-TBD-017 | `boot`再送契約のparameter（初期間隔、backoff係数、通常再送の回数、recovery間隔、**無応答時に送出を止めるまでの有限recovery budget**、**同一`(sid, id)`の最大再送回数**、**`rate_limited`で拒否されたときのcooldown後再送を含む上限**）。recovery budgetの総待ち時間は`PROTO-TBD-012`の拒否ACK最悪送出待ち時間以上とする。終了条件そのものは§4.1で確定済み | 起動時のlink確立latency測定とreconnect試験 |

## Revision履歴

| 日付 | Version | 変更 |
|---|---|---|
| 2026-07-27 | Draft 1 | 引継ぎprotocolを統合し、検証・recovery要件を追加 |
| 2026-07-28 | Draft 2 | Envelopeへ`sid`を追加。`hello`、`rate_limited`、`stale_session`を追加。Piの再起動でduplicate判定が誤るcaseを修正し、流量制限とlink-loss検知の未決事項を登録 |
| 2026-07-31 | Draft 2 review | ACKと完了eventへ`reply_sid`を追加し、要求送信側の再起動後に旧sessionの応答を誤認するcaseを修正 |
| 2026-08-10 | Draft 2 fixture | §3のinteger widthを確定し、§7へ単一lineの検証で決まるcodeの対応付けを追加（分類と送出の判断を分ける）。§12.1をschema群のfixture作成済みの状態へ更新。wire formatは変更していない |

### Draft schemaの互換性

Draft 1とDraft 2はどちらも`v: 1`を名乗るが、`sid`の必須化により互換性がない。
`v`だけを見るpeerは、互換性がないschemaを「対応するmajor version」と誤認し、
`unsupported_version`ではなく`invalid_envelope`として遅れて失敗する。

このため次を明記する。

- **Draft schemaは互換性の対象ではない。**Draft同士、およびDraftと将来のv1確定版の間で、wire互換を仮定してはならない。
- `v: 1`は、conformance fixtureが確定し両側のparserが合格した時点で初めて「確定したv1」を意味する。
- 確定前にDraft schemaで相互接続する場合、両側が同じDraft revisionのfixtureでbuildされていることを、**conformance fixtureの一致で**確認する。`boot.payload.version`はfirmware版、`hello.payload.version`はhost版であり、いずれもwire schemaの一致を証明しない。build識別子をschema互換の根拠に転用しない。
- Draft間の相互接続を実際に行う必要が生じた時点で、schema revisionを表すenvelope fieldを追加するか、out-of-bandの照合手順を定める。どちらにするかは`PROTO-TBD-015`とする。
- 確定後にwire formatを変更する場合は`v`を上げる。

実装着手前の変更であり、影響を受ける実装は存在しない。
