//! Protocolの上限値。
//!
//! ここに置く値は、`docs/protocol/esp32-pi-protocol.md`が`Candidate`または`TBD`としている
//! 段階の暫定値である。実測で確定するまで「検証済み」として扱わない。
//!
//! # string上限は行長を保証しない
//!
//! string上限はUTF-8 byte長で測る。一方JSON encodeは`"`を`\"`、`\`を`\\`、制御文字を
//! `\u00XX`（1 byte→6 byte）へ広げる。したがって**全fieldが上限内でも、encode後の行が
//! [`MAX_LINE_BYTES`]を超えることがある。**
//!
//! この場合`encode_line`が[`crate::ErrorCode::LineTooLong`]を返すため、黙って上限を
//! 超えた行がwireへ出ることはない。しかし「field上限を守れば行長も収まる」とは言えない。
//! `tests/limits.rs`が検査しているのは、**escapeが起きない文字での**worst caseだけである。
//! escapeを含むworst caseは同fileが別testで「検出されること」を固定している。
//!
//! escape後のwire sizeまで含めた上限の確定は`PROTO-TBD-002`に含める。
//!
//! # oversize時に保持するprefix長の定数は置かない
//!
//! §8手順6は、overflowした行を破棄する前に上限付きprefixから`(sid, id)`の復元を試みると定め、
//! そのprefix長を`PROTO-TBD-002`としている。**仕様は候補値を一つも示していない。**
//! JSONはkeyの順序を縛らず、未知のkeyが間に入りうるため、「`v`、`type`、`sid`、`id`を
//! 収めるのに必要なbyte数」に計算可能なworst caseは無い。だからTBDなのである。
//!
//! したがってここに`OVERSIZE_PREFIX_BYTES`のような定数を置かない。
//! [`crate::framing::LineFramer`]は、既に保持している行bufferをそのままprefixとして使う。
//! これは新しい数を決めずに済む退化した上限であり、追加のmemoryも要らない。
//! `PROTO-TBD-002`が「行長上限より十分小さいこと」を求めているとおり、後から
//! [`crate::framing::LineFramer::with_prefix_budget`]で縮められる。**縮めることは純粋な制限**
//! であり、いま復元できない行が復元できるようになることはない。

/// 実装が受理するwire protocolのmajor version（`v`）。
///
/// `docs/protocol/esp32-pi-protocol.md` §11。schemaがDraftである間、この値が一致しても
/// wire互換を意味しない。互換の根拠はconformance fixtureの一致である。
pub const PROTOCOL_VERSION: u16 = 1;

/// 改行を含む、encode済み1 lineの最大byte数。
///
/// §2の候補値であり、`PROTO-TBD-002`で確定する。worst-caseのpayloadとmemory testを
/// 経るまで確定値として扱わない。
///
/// **正本は仕様§2のtransport表である。**この定数はその値の写しであり、
/// 両者の一致を検査する自動化は無い。§2を変えるときはここも同時に変える。
pub const MAX_LINE_BYTES: usize = 1024;

/// 終端の改行を除いた、1 lineのbody部の最大byte数。
///
/// [`MAX_LINE_BYTES`]から導出した値であり、新しく決めた数ではない。§2の上限は改行を含むため、
/// 改行1 byteを引いたものがbufferへ蓄えてよいbody長になる。
///
/// **受信bufferの容量にはこちらを使う。**[`MAX_LINE_BYTES`]をそのまま容量にすると、
/// 上限ちょうどの行を1 byte分多く受け入れてしまい、`line_too_long`の判定が
/// [`crate::decode_line`]側と受信側の2箇所に分かれる。
///
/// CRLFで終わる行では、`\r`もbodyとして数える。改行の直前の`\r`は改行を受けた時点で
/// 取り除くが、それまではbufferを占めるためである。したがってCRLFの行は`\n`だけの行より
/// body予算を1 byte多く使う。§2は上限に`\r`を含めるかを書いていないため、
/// この振る舞いは`PROTO-TBD-002`への入力として記録する。
pub const MAX_LINE_BODY_BYTES: usize = MAX_LINE_BYTES - 1;

/// `boot.firmware`と`status.firmware`のbyte上限。
pub const MAX_FIRMWARE_BYTES: usize = 64;

/// `boot.board`のbyte上限。
pub const MAX_BOARD_BYTES: usize = 32;

/// `boot.reset_reason`と`status.reset_reason`のbyte上限。
pub const MAX_RESET_REASON_BYTES: usize = 32;

/// `hello.host`のbyte上限。
pub const MAX_HOST_BYTES: usize = 64;

/// `hello.version`のbyte上限。
pub const MAX_VERSION_BYTES: usize = 64;

/// `ack.detail`のbyte上限。
///
/// §6は「短く上限を設けた`detail`」とだけ定めており、値は`PROTO-TBD-006`で確定する。
pub const MAX_DETAIL_BYTES: usize = 128;

/// `status`が持つstate名とexpression名のbyte上限。
///
/// 列挙値そのものは`PROTO-TBD-006`（最終status field）と`PROTO-TBD-008`（motion名）で
/// 確定するため、ここでは長さだけを縛る。
pub const MAX_STATE_NAME_BYTES: usize = 32;
