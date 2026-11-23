# Version Record: #512 PROTO-02 の基板ファイルの更新と DRC（実機 Linux x86_64）

> 判定: `Partial`。`kicad-cli` と Python API（`pcbnew`）で基板ファイルを更新し、DRC・図の書き出しを実行した。GUI は起動していない。

[Version Record Template](../version-record-template.md)に沿い、この作業の実行環境と範囲を記録する。
Rust・ESP32・Raspberry Pi の項目は、この作業で使っていないため省く。

```text
Record ID: VR-2026-11-23-KICAD-PCB-NATIVE-LINUX
Date: 2026-11-23
Machine profile: Docs / Review に KiCad を追加（machine-profiles.md の Profile一覧に KiCad を扱う profile は無い。
  VR-2026-11-15-KICAD-ERC-VM と同じ扱い）
Operator role: AI agent（作業セッション）。KiCad の導入の操作はユーザーが行った
Repository commit: 220b9dd（develop base。作業 worktree の未 commit の差分に対して実行した）
Working tree clean: no（hardware/deskcat.kicad_pcb ほかの更新差分）

OS name / version: Ubuntu 24.04.4 LTS
CPU architecture / userspace: x86_64 / 64 bit
Container / VM / native: native（systemd-detect-virt: none）

KiCad: 8.0.9（package 8.0.9-0~ubuntu24.04.1。kicad-cli version: 8.0.9）
KiCad source: KiCad project の公式 PPA ppa:kicad/kicad-8.0-releases（apt の source が
  ppa.launchpadcontent.net/kicad/kicad-8.0-releases であることを apt-cache policy で確かめた。
  Ubuntu の標準 repository の候補は 7.0.11 で、使っていない）
Python: 3.12.3（system。KiCad の Python API は /usr/lib/python3/dist-packages の pcbnew、バージョン 8.0.9）
導入の承認: 承認があった：承認者 wachi-yoshitaka-11-dev、2026-11-22（UTC）、PM のセッション、対象 #512。
  範囲は AI の問い「PROTO-02 の図を直すため、この端末に KiCad 8 を入れてよいですか？」。
  範囲は、この端末への KiCad 8.0.9 の導入（公式 PPA）。

Commands run / Actual result:
  更新前の hardware/deskcat.kicad_pcb（HEAD の版）:
    kicad-cli pcb drc --schematic-parity --severity-all: 違反 0、未接続 0、回路図との食い違い 0。
  配置の更新:
    Python API（pcbnew）で、J5・F1・Q1（180 度回転）・J6・J7・R18・J8 の位置と、はんだ面（B.Cu）の導体 15 本を置き換え、
    C1 の枠と文字を移し、題の文字と R18・J6 の参照番号の位置を直した。保存は pcbnew の Save。
    手順の script は repository に入れていない（機械で再現する必要が出れば別途判断）。
  更新後:
    kicad-cli pcb drc --schematic-parity --severity-all: 違反 0、未接続 0、回路図との食い違い 0。
    （一度、R18 の参照番号が J8 の silk と重なる警告 1 件が出て、文字の位置を動かして 0 にした。
      同じ file を別の directory で実行すると library table が無いため library の警告が出るので、repository の中で実行した）
    python3 -I hardware/universal-board/check_star_point.py: OK
      （Q1 の 180 度回転を読めるよう改めた。後のレビューの指摘で、基板に実在する穴だけを導体が通ること、
        R18 の足2 が star point に直接つながり Pi・logic の戻りの導体を共有しないことの検査を足し、
        star point の穴を (5,16)（R18 の足2 の穴そのもの）にした。負の例として、(a) 指摘前の版
        （R18 が母線に付き、導体が基板に無い穴 (2,17) を通る）、(b) (2,17) を通る GND の線を足した版、
        (c) star point を (9,17) にした版は、いずれも NG になることを実行して確かめた）
    指摘の後の更新：R18 を (5,14)・(5,16) へ、GND の導体を実在の穴だけで置き直し、C1 の枠を1列左へ移した。
      DRC は違反 0（一度、R18 の外形が F1・J6 の外形と重なる error 2 件が出て、R18 の位置を変えて 0 にした）。
    kicad-cli pcb export svg: 成功（hardware/universal-board/plan-front.svg、plan-back.svg）。
      firefox の headless で描画して読んだ（作業セッションが画像を読んだ。人は見ていない）。

  v5 の更新（shunt R20・C1・C2・出口 W1・J8 を足し、部品を角に寄せた配置。2026-11-23）:
    回路図・基板ファイルを、KiCad 8.0.9 の kicad-cli と Python API（pcbnew）で更新した。手順の script は repository に入れていない。
    kicad-cli pcb drc --schematic-parity --severity-all: 違反 0、未接続 0、回路図との食い違い 0
      （部品の外形 courtyard の重なりを含む。途中、J8 と J5・F1、J7 と J5 の courtyard の重なり error 4 件が出て、配置を直して 0 にした）。
    kicad-cli sch erc --severity-all: Errors 0、Warnings 168（endpoint_off_grid 148、lib_symbol_issues 20）。
    python3 -I hardware/universal-board/check_star_point.py: OK（90 度の倍数の回転を読めるよう改め、star point を
      「B から J5− への経路上で、経路の外へ枝が出る最初の点（B 自身でもよい）」とした。負の例として、
      J7− を J8− へ直結した版（迂回）と、J7− を servo の node A へつないだ版は、NG になることを実行して確かめた）。
    その後の更新（レビューの指摘で GND を「J5−・shunt の B・J7−・J8−・R18 が同じ node」にした配置 v6。2026-11-23）:
      kicad-cli pcb drc --schematic-parity --severity-all: 違反 0、未接続 0、回路図との食い違い 0。
      kicad-cli sch erc --severity-all: Errors 0、Warnings 168（回路図は変えていない）。
      python3 -I hardware/universal-board/check_star_point.py: OK。規則を「B と J5− の間の導体が 2 穴以内、J7−・J8−・R18 の
        導体が別々に node へつき、穴を共有せず、node を迂回しない」へ強めた。負の例として、直前の版（v5。B と J5− の間に
        45.7 mm の導体があり、Pi の GND がその途中につく）と、J8− と J7− の導体を直結した版は、NG になることを実行して確かめた。
    さらにその後の更新（レビューの指摘で C1・C2 の−を A の node から GND の star point の側へ移した配置。2026-11-23）:
      kicad-cli pcb drc --schematic-parity --severity-all: 違反 0、未接続 0、回路図との食い違い 0。
      kicad-cli sch erc --severity-all: Errors 0、Warnings 168（回路図の C1・C2 の − の label を ADC-SHUNT から GND へ変えた）。
      python3 -I hardware/universal-board/check_star_point.py: OK。規則に「C1・C2 の−が A の node に無く、Pi・ESP32・R18 と導体を
        共有しない別の導体で star point の node へつく」を足した。負の例として、C の−が A の node にある直前の版は NG になることを
        実行して確かめた。W1 の footprint から外形（courtyard）を外した（本体が無い穴のため）。
    巡9 の指摘後の更新（R18 の足2 の導体を B の穴へ直接つけた。2026-11-23）:
      DRC は違反 0、未接続 0、食い違い 0。ERC は Errors 0、Warnings 168。
      check_star_point.py に「R18 の足2 の導体が B の穴に直接つく」を足した。負の例として、R18 が B と J5− の間の (3,12) につく
      直前の版は NG になることを実行して確かめた。
    PTC を枝ごとに分けた更新（F1：logic の枝、F2：servo の枝、ジャンパー W2。2026-11-23）:
      回路図に F2・ジャンパー W2（新しい symbol `WIRE_JUMPER`）を足し、5V の節点を 5V-NODE・5V-LOGIC・5V-SERVO に分けた。
      kicad-cli sch erc --severity-all: Errors 0、Warnings 167。
      kicad-cli pcb drc --schematic-parity --severity-all: 違反 0、未接続 0、回路図との食い違い 0
        （途中、ジャンパーの部品面の線を 1 本の直線にして Q1 の足と短絡した error、部品の silk と参照番号の重なりの warning が出て、
         ジャンパーの通り道と参照番号の位置を直して 0 にした）。
      python3 -I hardware/universal-board/check_star_point.py: OK。「PTC は枝ごとに1個で共有しない」「ジャンパーは2本まで。部品面の線も導体として
        数え、net ごとの足がつながる」を足した。負の例として、F2 を外した版、F2 の b を logic の net にした版、J6＋を logic の net にした版、
        ジャンパーの線を外した版は、いずれも NG になることを実行して確かめた。
    kicad-cli sch export netlist（kicadxml）で overview の netlist を書き出し直した。
    kicad-cli pcb export svg: 成功。firefox の headless で描画して読んだ（作業セッションが画像を読んだ。人は見ていない）。

  巡10の指摘の後の更新（W2 の端を (9,5) から (11,5) へ移し、J8 の口の正面から外した。2026-11-23）:
    pcbnew で W2 の足2、F.Cu の線、B.Cu の 5V-IN の導体（(7,5)→(11,5)）を直し、footprint を WireJumper_dx9_dy-4 に替えた。
    kicad-cli pcb drc --schematic-parity: 違反 0、未接続 0、footprint の誤り 0。
    kicad-cli sch erc: Errors 0（Warnings 167）。
    check_star_point.py・render_view.py --check・overview と breadboard-rebuild の generate.py --check・
    validate_doc_links.py・validate_table_columns.py: OK。
    plan.md の部品表の穴を、基板ファイルの足の位置と script で突き合わせ、全部品で一致を確かめた。
    kicad-cli pcb export svg: plan-front.svg・plan-back.svg を書き出し直した。

Known differences from documented profile:
  machine-profiles.md に KiCad の扱いが無い。この記録は、この端末でこの版の kicad-cli と pcbnew が
  この基板ファイルを読み・更新し・DRC と書き出しを実行できたことに限る。
  GUI（pcbnew）は起動していない（DISPLAY が無い）。回路図の ERC は、上の v5 の更新で実行した（記録の前半の更新では実行していない）。
  別の版、別の端末は確かめていない。

Conclusion: Partial（kicad-cli と pcbnew での更新・DRC・書き出しは成功。GUI での表示は未実施）
Next action: 基板ファイルを GUI で開いて確かめる場合は、KiCad 8 以上の GUI がある端末で行う。
```
