# Version Record: #512 KiCad の回路図の読み込みと ERC（VM）

> 判定: `Partial`。KiCad の GUI では開いていない（`kicad-cli` で読み込み、ERC と netlist の出力をした）。

[Version Record Template](../version-record-template.md)に沿い、この作業の実行環境と範囲を記録する。
Rust・ESP32 の項目は、この作業で使っていないため省く。

```text
Record ID: VR-2026-11-15-KICAD-ERC-VM
Date: 2026-11-15
Machine profile: Docs / Review に KiCad を追加（machine-profiles.md の Profile一覧に KiCad を扱う profile は無い。
  同表の Docs / Review の「不要なもの」は Rust、ESP-IDF、USB ドライバ、実機で、KiCad はそこに無い）
Operator role: AI agent（導入は 2026-11-15 に人間が承認した。VM 側の PM のセッション）
Repository commit: d9fa34d9（develop base）
Working tree clean: no（#512 の回路図の pin 番号の差分を読み込んだ）

OS name / version: Ubuntu 22.04.5 LTS
CPU architecture / userspace: x86_64 / 64 bit
Container / VM / native: VM（systemd-detect-virt: microsoft）

KiCad: 8.0.9（package 8.0.9-0~ubuntu22.04.1）
KiCad source: KiCad project の公式 PPA ppa:kicad/kicad-8.0-releases
  Ubuntu 22.04 の標準 repository の候補は 6.0.2 で、回路図の形式（version 20240620、KiCad 8）を読めないため使わなかった。
  導入: apt-get install --no-install-recommends kicad（symbol・footprint・3D の library package は入れていない）

Commands run / Actual result:
  回路図と project file を作業用の複製へ写し、複製に対して実行した（repository の file を KiCad に書き換えさせない）。
  kicad-cli sch erc --format report --severity-all: 読み込み成功。Errors 0、Warnings 135。
    endpoint_off_grid 111（pin と wire の端が接続の grid 外）、
    lib_symbol_issues 24（symbol の library `DeskCat` が library table に無い。symbol は回路図に埋め込み済み）。
  電源の経路（PROTO-02）と project の library（DeskCat.kicad_sym、DeskCat.pretty、sym-lib-table、fp-lib-table）を足した後:
    Errors 0、Warnings 161（endpoint_off_grid 143、lib_symbol_issues 18。KiCad 標準の Device library が入っていないため）。
  サーボ信号の直列抵抗 R19 を足した後: Errors 0、Warnings 165（endpoint_off_grid 146、lib_symbol_issues 19）。
  kicad-cli sch export netlist --format kicadxml: 成功（hardware/overview/deskcat-netlist.xml の元）。
  回路図を機能ごとの配置に描き直した後（A3）: Errors 0、Warnings 160（endpoint_off_grid 141、lib_symbol_issues 19）。
    描き直しの前後の netlist（kicadxml）で、net の名前と各 net の pin の集合が33本とも一致した。
  ACCEL-01 の symbol に VDD の pin を足し、3V3 へつないだ後（2026-11-16）: Errors 0、Warnings 161（endpoint_off_grid 142、lib_symbol_issues 19）。
  kicad-cli pcb drc --schematic-parity --severity-all（deskcat.kicad_pcb）: 違反 0、未接続 0、回路図との食い違い 0。
  kicad-cli pcb export svg: 成功（hardware/universal-board/plan-front.svg、plan-back.svg）。
  基板ファイルは KiCad 8.0.9 の Python API（pcbnew）で一度だけ組み、以後は KiCad で編集する正本とした。
  kicad-cli sch export netlist --format kicadxml: 成功。
  pin 番号の差分の前（d9fa34d9 の file）と後で、ERC の件数と種類は同じ。

Known differences from documented profile:
  machine-profiles.md に KiCad の扱いが無い。この記録は、この VM でこの版の kicad-cli が
  この回路図を読めたことに限る。GUI での表示、別の版、別の端末は確かめていない。
  回路図の symbol の pin の電気種別はすべて passive であり、ERC の pin の種別の矛盾の検査は効かない。

Conclusion: Partial（kicad-cli での読み込み・ERC・DRC・netlist と SVG の出力は成功。GUI での表示は未実施）
Next action: 回路図を GUI で開いて表示を確かめる場合は、KiCad 8 以上の GUI がある端末で行う。
```
