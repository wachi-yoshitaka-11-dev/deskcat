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
      （Q1 の 180 度回転を読めるよう改め、star point の穴を (7,17) へ変えた。
        star point を (9,17) にした負の例で NG になることを、別に実行して確かめた）
    kicad-cli pcb export svg: 成功（hardware/universal-board/plan-front.svg、plan-back.svg）。
      firefox の headless で描画して読んだ（作業セッションが画像を読んだ。人は見ていない）。

Known differences from documented profile:
  machine-profiles.md に KiCad の扱いが無い。この記録は、この端末でこの版の kicad-cli と pcbnew が
  この基板ファイルを読み・更新し・DRC と書き出しを実行できたことに限る。
  GUI（pcbnew）は起動していない（DISPLAY が無い）。回路図の ERC はこの記録では実行していない。
  別の版、別の端末は確かめていない。

Conclusion: Partial（kicad-cli と pcbnew での更新・DRC・書き出しは成功。GUI での表示は未実施）
Next action: 基板ファイルを GUI で開いて確かめる場合は、KiCad 8 以上の GUI がある端末で行う。
```
