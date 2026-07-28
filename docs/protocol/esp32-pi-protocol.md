# ESP32–Raspberry Pi Protocol

> 状態: Draft v1 — transport制限と実装fixtureは引き続き検証が必要
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
{"v":1,"id":1234,"ts_ms":456789,"type":"head_touched","payload":{}}
```

| Field | Type | 必須 | 意味 |
|---|---|---:|---|
| `v` | unsigned integer | Yes | Protocol major version |
| `id` | unsigned integer | Yes | 送信側内で単調増加するmessage ID |
| `ts_ms` | unsigned integer | Yes | 送信側のuptime（milliseconds） |
| `type` | string | Yes | snake_caseのmessage type |
| `payload` | object | Yes | Type固有のfield |

規則:

- Top-level valueのtypeが異なる場合は不正とする。
- 空のpayloadは`null`ではなく`{}`とする。
- `ts_ms`はwall-clock timeではなく、device間で直接比較できない。
- 送信側が再起動したとき、ID追跡はresetされる。
- `boot` messageによって、新しいESP32 sessionを開始する。
- Typeに別の規定がない限り、同じmajor version内では未知の追加payload fieldを無視してよい。
- 未知のmessage typeは計数し、通信方向に応じて無視するかerrorを返す。

正確なinteger widthとwrap動作は、実装を受け入れる前に共有test fixtureで確定する。

## 4. ESP32からPiへのmessage

### 4.1 `boot`

Protocol taskの準備完了後に一度送信する。

```json
{"v":1,"id":1,"ts_ms":100,"type":"boot","payload":{"firmware":"0.1.0","board":"esp32","reset_reason":"power_on"}}
```

必須payload:

| Field | Type | 意味 |
|---|---|---|
| `firmware` | string | Firmware version／build identity |
| `board` | string | Firmware board-configuration ID |
| `reset_reason` | string | Machine-readableなreset reason |

将来のoptional fieldにはprotocol capabilityとhardware revisionを含められる。Secretまたは生のenvironment dataを公開しない。

### 4.2 `head_touched`

```json
{"v":1,"id":22,"ts_ms":8400,"type":"head_touched","payload":{"duration_ms":720,"strength":0.68}}
```

正確なtouch hardwareが確定するまで、`strength`の意味と範囲は`TBD`とする。比較可能なstrengthが存在しない場合、値を捏造せずfieldを省略する。

### 4.3 `tapped`

```json
{"v":1,"id":23,"ts_ms":9300,"type":"tapped","payload":{"magnitude_g":1.84}}
```

正確なevent classifierと報告するmagnitudeの意味は、選定したaccelerometerと実験根拠を必要とする。

### 4.4 `lifted`

```json
{"v":1,"id":24,"ts_ms":11000,"type":"lifted","payload":{"duration_ms":1200}}
```

最初の受け入れsliceではoptionalとする。Accelerometer classifierを検証した後にのみ有効化する。

### 4.5 `environment`

```json
{"v":1,"id":25,"ts_ms":15000,"type":"environment","payload":{"temperature_c":27.4,"humidity_pct":55.1,"pressure_hpa":1008.3}}
```

選定したsensorが対応する測定量だけを含める。最初のdraftでcompact eventからquality fieldを省略する場合でも、各実装はfreshnessとdevice errorを追跡しなければならない。

### 4.6 `status`

`get_status`へのresponseとして送信し、必要に応じてrate limit付きの定期health messageとして送信する。

初期payload group:

```json
{
  "v":1,
  "id":26,
  "ts_ms":15100,
  "type":"status",
  "payload":{
    "firmware":"0.1.0",
    "reset_reason":"power_on",
    "display":{"state":"ready","expression":"neutral"},
    "servo":{"state":"disabled"},
    "sensors":{"touch":"unknown","acceleration":"unknown","environment":"unknown"},
    "protocol":{"parse_errors":0,"oversize_lines":0,"unknown_types":0}
  }
}
```

最終schemaは、意味が安定し、encode後のsizeに上限を設けられるfieldだけに絞る。

### 4.7 完了・fault event

提案中のtype:

- `motion_completed`
- `motion_stopped`
- `sensor_fault`
- `protocol_fault`

長時間commandには直ちに`ack`を返し、完了時には元のcommand IDを参照する個別eventを使用する。

## 5. PiからESP32へのcommand

### 5.1 `set_expression`

```json
{"v":1,"id":901,"ts_ms":52000,"type":"set_expression","payload":{"name":"happy","transition_ms":300}}
```

初期のexpression名:

- `neutral`
- `happy`
- `surprised`

Firmwareは未知の名前をrejectする。Transition durationにはdisplay実装で定義する上限を設ける。

### 5.2 `play_motion`

```json
{"v":1,"id":902,"ts_ms":52200,"type":"play_motion","payload":{"name":"nod","speed":0.45,"repeat":1}}
```

Piはraw pulse widthではなく、名前付きの高水準motionを送信する。

Firmwareは次を実行する。

- Motion名を検証する
- 有限の数値入力であることを検証する
- 正規化されたspeedに上限を設ける
- Repeat countに上限を設ける
- Hard angle、velocity、acceleration limitを適用する
- 要求をrejectするか、clampしたことを報告する

初期に受け入れるmotion名は、サーボ機構のcalibrationが完了するまで`TBD`とする。

### 5.3 `show_text`

```json
{"v":1,"id":903,"ts_ms":52300,"type":"show_text","payload":{"text":"なでてくれて、ありがと。","duration_ms":5000}}
```

Firmwareは次に上限を設ける。

- UTF-8 byte length
- Display duration
- Control character
- Line countまたはlayout処理量

Textとface描画の優先順位はUI state machineで定義する。

### 5.4 `show_choices`

```json
{"v":1,"id":904,"ts_ms":53000,"type":"show_choices","payload":{"prompt":"少し休憩する？","choices":[{"id":"yes","label":"する"},{"id":"later","label":"あとで"}],"timeout_ms":15000}}
```

Choice count、ID、label、prompt length、timeoutに上限を設ける。正確なinteraction eventは、touch hardwareとlayoutの選定まで保留する。

### 5.5 `get_status`

```json
{"v":1,"id":906,"ts_ms":53500,"type":"get_status","payload":{}}
```

ESP32はcommandへacknowledgeし、`status` snapshotを送信する。

### 5.6 `ping`

```json
{"v":1,"id":907,"ts_ms":54000,"type":"ping","payload":{}}
```

ESP32はACKを返し、ACK payloadにuptimeを含めてよい。Heartbeatの最終用途とintervalは`TBD`とする。

## 6. Acknowledgement

```json
{"v":1,"id":905,"ts_ms":53100,"type":"ack","payload":{"reply_to":903,"status":"ok"}}
```

必須payload:

| Field | Type | 意味 |
|---|---|---|
| `reply_to` | unsigned integer | Acknowledgeするcommand ID |
| `status` | string | `ok`または`rejected` |

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

Parser counterでは、invalid UTF-8、invalid JSON、invalid envelope、unknown type、oversize lineを区別する。

## 8. 受信動作

Receiverは次の手順で動作する。

1. 固定上限を持つbufferへbyteを蓄積する。
2. 任意の回数のreadに分割された一つのmessageを処理する。
3. 1回のreadに含まれる複数messageを処理する。
4. 改行受信時に、直前にある任意のcarriage returnを除去する。
5. そのlineの不正なUTF-8またはJSONをrejectする。
6. Overflow時は次の改行まで破棄する。
7. 該当counterを増加させる。
8. Resetせず後続lineのparseを続ける。
9. Protocol出力によってsensor、motion safety、watchdogの進行をblockしない。

候補の1024-byte制限には、encode済みobjectと改行を含む。正確なbuffer計算をtestで確認する。

## 9. Retryとduplicate処理

Draft v1の初期policy:

- ACKが必要なcommandはtimeout後に1回retryする。
- 同じcommand IDを再利用する。
- ESP32は直近に処理したcommand IDとresultを保持する。
- Duplicateには保持したresultを返し、非idempotentな動作を再実行しない。

受け入れ前の`TBD`:

- ACK timeout
- 保持件数と期間
- Integer wrapの処理
- Duplicateが保持履歴より古い場合の動作
- ACKを必要とするmessage

状態設定commandはidempotentにする。Relativeまたは名前付きの物理motionにはduplicate suppressionが必要である。

## 10. Reconnect

1. ESP32が起動または再接続し、`boot`を送信する。
2. PiがESP32 sessionのID追跡をresetする。
3. Piが`get_status`を送信する。
4. ESP32がACKと`status`を送信する。
5. Piが実際のdisplay／motion stateとdesired stateを比較する。
6. Piが安全な状態設定commandを送信する。
7. どちらも古いrelative motionを自動再実行しない。

Piだけが再起動した場合は、portを開いた後に`get_status`を送信する。

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
- 同じIDによるretry
- Reconnectとstatus同期

環境が許す範囲で、hostとfirmwareの両実装が同じfixtureを使用しなければならない。

## 13. 未決定事項

| ID | 判断 | 必要な根拠 |
|---|---|---|
| PROTO-TBD-001 | 最終baud | Pi／ESP32のthroughput・安定性test |
| PROTO-TBD-002 | 最終最大line byte数 | Worst-caseの上限付きpayloadとmemory test |
| PROTO-TBD-003 | Integer widthとwrap | Rust modelと長時間動作の検討 |
| PROTO-TBD-004 | ACK timeout | 測定latencyとrecovery test |
| PROTO-TBD-005 | Duplicate保持 | Memory予算とretry window |
| PROTO-TBD-006 | 最終status field | 診断要件とencode size test |
| PROTO-TBD-007 | Textとchoiceの制限 | LCD layoutとmemory測定 |
| PROTO-TBD-008 | Motion名と範囲 | Servo calibrationと動作設計 |
| PROTO-TBD-009 | Touch strengthの意味 | 正確なtouch controllerと実験 |

## Revision履歴

| 日付 | Version | 変更 |
|---|---|---|
| 2026-07-27 | Draft 1 | 引継ぎprotocolを統合し、検証・recovery要件を追加 |
