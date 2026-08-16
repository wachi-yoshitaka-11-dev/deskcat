# microSD（SD-01）health check記録

> 状態: 実施済み。**`HW-TBD-015`は2026-08-15にcloseした**
> 判定: `Partial`
> 実施日: 2026-08-12（`f3`による検査と識別情報の採取）／2026-08-13（registerの読み方の一次資料照合、raw device試験）
> 対象: [hardware-bom.md](hardware-bom.md) の `SD-01`（Samsung EVO Plus microSDHC 32GB、`U1`表示）
> 追跡: [tbd-register.md](tbd-register.md) の `HW-TBD-015`、[#114](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/114)

`SD-01`はRaspberry Piのbootとstorageに使う。Pi へ書き込む前に健全性を確かめ、
後で不具合が出たときにcardを疑わずに済むようにする。

様式は[Version Record Template](../toolchains/version-record-template.md)に倣う。
**同 templateの`記録してはいけないもの`はこの文書にも適用する。**
ただし置き場所は`docs/toolchains/version-records/`ではない。同 directoryの
[README](../toolchains/version-records/README.md)は記録を`Profile`列（toolchainの検証profile）に
対応させる規則であり、部品の健全性checkに対応するprofileが無いためである。

## 判定基準

**この節は実測より前に確定させた。**実測値を見てから基準を後付けしない
（[AGENTS.md](../../AGENTS.md) 推測禁止）。

| # | 観点 | 判定基準 | 取得方法 |
|---|---|---|---|
| ① | 容量詐称（偽造品の検出） | `f3read`が報告する`Data LOST`が**0 sector**である（`Overwritten`／`Corrupted`／`Slightly changed`の全区分で0） | `f3write`→`f3read` |
| ② | 読み書きの通し確認 | `f3write`が全fileを書き切り、`f3read`の`Data OK`が書込み総量と一致する | 同上 |
| ③ | 速度がClass表示と矛盾しないか | `f3write`の平均書込み速度が**10 MB/s以上**である（`U1`＝UHS Speed Class 1の定義値） | `f3write`出力 |
| ④ | SMART相当の情報 | 取得手段の有無を確認して記録する | 後述 |
| ⑤ | 識別情報の裏取り | sysfsのCID／CSD／SCRが現物print（Samsung、32GB、microSDHC）と矛盾しない | `/sys/block/mmcblk0/device/` |

### 基準に付ける条件

**①の検出原理。**偽造microSDは公称容量を名乗って実容量がそれより小さく、実容量を超えて
書くと先頭へ巻き戻って以前のdataを上書きする。検出するには**位置ごとに異なるdata**を
全域へ書いて読み戻す必要がある。`f3write`は位置に依存する擬似乱数を書くためこれを満たす。
**同一patternを繰り返し書く検査（`badblocks -w`等）は、巻き戻っても照合が通るため
①を検出できない。**方法の選定理由はここにある。

**③を満たさない場合の扱い。**`U1`の10 MB/sはUHS-I bus mode下での保証値である。
host側がUHSをnegotiateしていなければ、cardの性能とは別の理由で下回りうる。
**下回った場合は、hostのbus modeとclockを記録したうえで
「card起因と切り分けられない」と書く。**「たぶん正常」で通さない。

> **判定の閾値（10 MB/s）はRevision 0から変えていない。**この段落は当初
> 「High Speed（25 MHz）で動作していれば」と具体的なclock値を書いていたが、
> **その値を一次資料で確かめていなかったため削除した**（Revision 2）。
> 実測したhostのclockは33 MHzであり、当初書いた値とも一致しない。

**カバー範囲。**`f3write`はmount済みfile systemの**空き領域**を埋める。したがって
file systemのmetadata領域と予約領域は触らない。**literalな全セクタを見ていない。**
literalな全セクタ検査には`badblocks -w`（root必須・完全破壊）が要るが、
**今回は実施しない**（下の「実施しなかった項目」に記載する）。

**④の性質。**SMARTはATA／NVMeの機能である。**この個体には取得手段が無い。**
`scr`が示すPhysical Layer Version 3.0Xの世代であり、hostも`sd high-speed`で
接続している。**この経路にSMART相当のcommandは無い。**
したがって④は**「実施しなかった」のではなく「取得手段が存在しない」**である。
この区別を結果欄に書く。

> **「SD card全般にSMARTが無い」とは書かない。**この記録は当初そう書いていたが、
> **誤りである**（Revision 5で訂正）。Physical Layer Simplified Specification Ver 9.10の
> **Section 8.4.7（SD Express CardのPower and Thermal Management）**は、hostが
> `SMART / Health Information Log`から得たcomposite temperatureを使ってよいと記している。
> **SD ExpressはNVMe interfaceを持つため、SMARTが利用できる。**
> **この個体はSD Expressではない**ので結論は変わらないが、主張の範囲を個体に限る。

## 実施環境

```text
Record ID: 2026-08-12-sd01-health-check
Date: 2026-08-12
Machine profile: 該当なし（Machine Profilesに部品health check用のprofileが無い）
  適用される制約は[ADR-0005](../decisions/0005-standard-development-os.md)の
  「実機Linux」であり、これは満たしている。
Operator role: 開発者（human）の監督下でのAI agent作業。card挿入、
  root権限を要するcommand、tool導入と撤去はhumanが実行した
Repository commit: 6f387b6bbb2a8a3e080f27c7d4870245b19881b4
  **これは測定を行った時点のtreeである。**本記録を載せるbranchはその後
  cbd6fa7（#113のmerge後のdevelop）へrebaseしたが、測定をやり直してはいない。
  測定はrepositoryの内容に依存しないため、rebaseは結果に影響しない。
Working tree clean: no（本作業の追加分を含む）

OS name: Ubuntu
OS version: 24.04.4 LTS
Kernel: 7.0.0-28-generic
CPU architecture: x86_64
Userspace bitness: 64-bit
Container / VM / native: native（実機）。systemd-detect-virt: none。
  containerでもVMでもない

Card reader: 内蔵SDHCI（PCI）。driver sdhci-pci、PCI ID 1180:E823（Ricoh）、
  host controller mmc0。USB接続のcard readerではない

Tools:
  f3: 8.0（Ubuntu package f3 8.0-2build2）。作業用に一時導入し、作業後に撤去した
  util-linux (lsblk, wipefs): 2.39.3
  dosfstools (mkfs.vfat): 4.2-1.1build1
  parted: 3.6 (GNU parted)
  udisks2 (udisksctl): 2.10.1-6ubuntu1.3
  coreutils (dd): 9.4
  検査用file system: FAT32（mkfs.vfat -F 32）。単一partitionへ再formatした
```

**`mkfs.vfat`は`--version`を受け付けない。**上の値はpackage版
（`dpkg -l dosfstools`）である。`udisksctl`も同様にpackage版を記載した。

**`f3`の一時導入について。**[AGENTS.md](../../AGENTS.md)は「ツール導入は、対象Issue、端末profile、
人間の確認が揃った開発端末だけで行う」と定める。対象Issueは
[#114](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/114)、導入と撤去はhumanが実行した。
撤去の確認は下の「実測結果」に記載する。

## 実測結果

### 実行したcommand

**別のoperatorが観測値を再現できるよう、取得に使ったcommandを記録する。**
root権限を要するものはhumanが実行した。

```bash
# card識別情報（CID由来field、CSD、SCR、SSR）。root不要
for f in type manfid oemid name hwrev fwrev date scr ssr ocr cid csd preferred_erase_size; do
  printf "%-22s " "$f"; cat "/sys/block/mmcblk0/device/$f"
done
```

```bash
# host側のbus modeとclock、およびcapability register。root必須
sudo sh -c 'for f in /sys/kernel/debug/mmc0/*; do [ -f "$f" ] && echo "--- $f" && cat "$f"; done; echo "=== dmesg ==="; dmesg | grep -i mmc0'
```

```bash
# 検査対象を単一partitionにする（既存内容を破壊する）。root必須
sudo wipefs -a /dev/mmcblk0 && sudo parted -s /dev/mmcblk0 mklabel msdos mkpart primary fat32 1MiB 100% && sudo mkfs.vfat -F 32 -n F3TEST /dev/mmcblk0p1
```

```bash
# mount（polkit経由、root不要）→ 検査 → 後片付け
udisksctl mount -b /dev/mmcblk0p1
f3write /media/<user>/F3TEST/
f3read  /media/<user>/F3TEST/
rm -f /media/<user>/F3TEST/*.h2w && sync
udisksctl unmount -b /dev/mmcblk0p1
```

raw device試験のcommandは後述の該当節に置く。

**`caps`と`caps2`はroot権限でも`EPERM`を返し取得できなかった。**
そのため本記録にhost controllerのcapability registerの値は無い。

### 識別情報（⑤）

**事実と解釈を分けて書く。**下の「読み取った値」は再現できる観測である。
「解釈」はregisterのbit定義に基づく読み方であり、**一次資料と照合した**（`manfid`の1件を除く。後述）。

#### 読み取った値（観測）

`/sys/block/mmcblk0/device/`から読んだ。

| 項目 | sysfs | 値 |
|---|---|---|
| Card type | `type` | `SD` |
| Manufacturer ID | `manfid` | `0x00001b` |
| OEM/Application ID | `oemid` | `0x534d` |
| Product name | `name` | `EB1QT` |
| Hardware revision | `hwrev` | `0x3` |
| Firmware revision | `fwrev` | `0x0` |
| Manufacturing date | `date` | `07/2018` |
| CSD | `csd` | `400e00325b590000ee7f7f800a404000` |
| SCR | `scr` | `02b5800200000000` |
| SSR | `ssr` | `0000000005000000040090000f051c00`（以降すべて0） |
| Erase単位 | `preferred_erase_size` | `4194304`（4 MiB） |
| 容量 | `lsblk` | 29.80 GiB（`/dev/mmcblk0`） |

**`serial`は読んだが記録しない。**[Version Record Template](../toolchains/version-record-template.md)が
個体を識別する値を禁じている。`cid`は`serial`を含むため、そのままは載せず、
上表のとおり分解済みfieldだけを記載する。

#### 解釈（**一次資料と照合済み。1件を除く**）

照合先は[SD Association Simplified Specifications](https://www.sdcard.org/downloads/pls/)が公開する
**Part 1 Physical Layer Simplified Specification Version 9.10（2023-12-01、409 page）**である。
**humanが利用規約に同意してPDFを取得し、2026-08-13に該当節を直接読んだ。**

| register | bit位置 | 値 | 導かれる内容 | 照合先 |
|---|---|---|---|---|
| `csd`先頭byte＝`0x40` | `CSD_STRUCTURE`＝`[127:126]`＝`01`b | 1 | CSD Version 2.0（High Capacity and Extended Capacity） | Table 5-3 |
| `scr` | `SD_SPEC`＝`[59:56]`＝2、`SD_SPEC3`＝`[47]`＝1、`SD_SPEC4`＝`[42]`＝0、`SD_SPECX`＝`[41:38]`＝0 | — | **Physical Layer Specification Version 3.0X** | Table 5-19 |
| `ssr` byte 8 | `SPEED_CLASS`＝`[447:440]` | `04h` | **Class 10** | Table 4-45 |
| `ssr` byte 14上位nibble | `UHS_SPEED_GRADE`＝`[399:396]` | `1h` | **10MB/sec and above（＝`U1`）** | Table 4-52 |
| `ssr` byte 10上位nibble | `AU_SIZE`＝`[431:428]` | `9h` | 4 MB | Table 4-47 |
| `oemid`＝`0x534d` | `OID`＝`[119:104]` | — | OIDは**2文字のASCII文字列**と規定される。`0x53`＝`S`、`0x4d`＝`M` → `"SM"` | Section 5.1 |
| `name`＝`EB1QT` | `PNM` | — | PNMは**5文字のASCII文字列**と規定される。5文字であり整合する | Section 5.1 |

**byte位置の対応が正しいことの裏付け。**上表は「sysfsが`ssr`をMSB firstで出力する」ことを
前提にbyte位置を割り出している。この前提は`AU_SIZE`で独立に検証できた。
byte 10上位nibbleの`9h`＝4 MBは、**kernelが別経路で報告する
`preferred_erase_size`＝4194304（4 MiB）と一致する。**
同じ前提で読んだ`SPEED_CLASS`（byte 8）と`UHS_SPEED_GRADE`（byte 14）も、
したがって位置の取り違えではない。

##### 照合できなかった1件

| 項目 | 状態 |
|---|---|
| `manfid`＝`0x1b`＝Samsung | **この経路では解ける見込みが無い。**仕様書はMIDを「SD-3C, LLCが管理・定義・割り当てる」8 bitの番号と規定するのみで、**どの番号がどのメーカーかという登録簿を収録していない**（Section 5.1）。**2026-08-15に確認したところ、SD-3Cも登録簿を公開していない**（[Licensees](https://www.sd-3c.com/Licensees.aspx)はライセンシー名の一覧であり、MIDとの対応表を持たない）。**規格を定める側とMIDを割り当てる側の両方が公開していないため、「別の一次資料を当たる」経路に見込みが無い。**詳細は下記 |

**確認した2つの資料はいずれもMIDの対応表を持たない**（2026-08-15確認）。

- **Physical Layer Simplified Specification Ver 9.10**: MIDを「SD-3C, LLCが管理・定義・
  割り当てる」8 bitの番号と規定する（Section 5.1）。**対応表は収録していない。**
- **[SD-3C Licensees](https://www.sd-3c.com/Licensees.aspx)**: ライセンシーの**社名一覧**であり、
  **MIDとの対応表を持たない。**

**確認した2つは、SD規格を定める側とMIDを割り当てる側そのものである。**その両方が対応表を
公開していない以上、**この経路で解ける見込みは無い。**したがって追跡を打ち切る。

**「一次資料がこの世に存在しない」とまでは書かない。**確認したのは上の2つであり、
**網羅的に探したわけではない。**打ち切るのは「見込みが無い」からであって、
「存在しないことを証明した」からではない。

**非公式の集計は存在するが根拠に採らない。**個人が実測から編んだ一覧には
`0x1b`をSamsung、OEM IDを`534d`（`SM`）とするものがあり、本個体の値と一致する。
**しかし著者自身が非公式と明記しており、`AGENTS.md`の推測禁止の下で根拠に採れない。**
一致する事実だけを記し、判定には使わない。

#### 現物printとの照合結果

**`32`／`microSDHC`／`U1`の各表示は、card自身のregisterと一致する。**

- `U1`: `UHS_SPEED_GRADE`＝`1h`。**card自身が`U1`相当を申告している**
- `microSDHC`: `CSD_STRUCTURE`＝1はHigh CapacityとExtended Capacityの両方を含むため、
  これだけではSDHCとSDXCを分けられない。**容量29.80 GiB（32 GB未満）と併せてSDHCとなる**
- `32`: 29.80 GiB ＝ 10進で約32 GBであり整合する

**`Samsung`の表示だけはregisterで裏付けられていない。**`oemid`が`"SM"`であることは確認したが、
`"SM"`がSamsungを指すことも`manfid`＝`0x1b`がSamsungであることも、この仕様書では検証できない。

**したがって`SD-01`の識別の正は、引き続き現物printである。**registerが加えたのは、
**メーカー名を除く各属性（容量区分、speed class、UHS speed grade、spec version）について、
printの読み取り誤りを排除できた**ことである。

**なお、判定基準①②④はこの解釈に依存しない。**①②は`f3`の出力から直接判定でき、
④はこの個体の接続経路にSMART相当のcommandが無いことから判定している。

### Bus modeとclock（③の判定に要る）

`/sys/kernel/debug/mmc0/ios`（root権限で取得）。

```text
clock:          33000000 Hz
actual clock:   33000000 Hz
vdd:            21 (3.3 ~ 3.4 V)
bus mode:       2 (push-pull)
chip select:    0 (don't care)
power mode:     2 (on)
bus width:      2 (4 bits)
timing spec:    2 (sd high-speed)
signal voltage: 0 (3.30 V)
driver type:    0 (driver type B)
```

**cardはUHS-I modeで動作していない。**`timing spec`は`sd high-speed`、
`signal voltage`は3.30 Vである。`dmesg`も`mmc0: new high speed SDHC card at address 0001`と
記録している（UHS modeで初期化されていれば`ultra high speed`と表示される）。
**`U1`の規定が適用される条件を満たしていない。**
これは判定基準③の但し書きが想定した条件そのものである。

**ただし「hostがUHS-Iに対応していない」とは書かない。**host controllerのcapability register
（`/sys/kernel/debug/mmc0/caps`／`caps2`）は**root権限でも`EPERM`を返して読めなかった。**
非対応なのかnegotiationが成立しなかっただけなのかを区別していない。

**`clock`値からbus帯域の上限を導かない。**この記録の作成時に一度
「33 MHz × 4 bit ＝ 約16.5 MB/s が上限」と書いたが、**後述の読み出し実測19.15 MB/sが
この値を超えており、導出が誤っていた。**`ios`が示す`clock`は取得時点の値であって、
転送中の実効clockと同一である保証がない。**したがってこの記録では、
bus帯域の上限を計算で示さず、実測値だけを根拠に用いる。**

### f3の出力

`docs/`配下にMarkdown以外の証拠fileを置かない規則のため、**両commandの出力全文を未加工で埋め込む。**
**省略はしていない。**

`f3write /media/<user>/F3TEST/`:

```text
F3 write 8.0
Copyright (C) 2010 Digirati Internet LTDA.
This is free software; see the source for copying conditions.

Free space: 29.80 GB
Creating file 1.h2w ... OK!
Creating file 2.h2w ... OK!
Creating file 3.h2w ... OK!
Creating file 4.h2w ... OK!
Creating file 5.h2w ... OK!
Creating file 6.h2w ... OK!
Creating file 7.h2w ... OK!
Creating file 8.h2w ... OK!
Creating file 9.h2w ... OK!
Creating file 10.h2w ... OK!
Creating file 11.h2w ... OK!
Creating file 12.h2w ... OK!
Creating file 13.h2w ... OK!
Creating file 14.h2w ... OK!
Creating file 15.h2w ... OK!
Creating file 16.h2w ... OK!
Creating file 17.h2w ... OK!
Creating file 18.h2w ... OK!
Creating file 19.h2w ... OK!
Creating file 20.h2w ... OK!
Creating file 21.h2w ... OK!
Creating file 22.h2w ... OK!
Creating file 23.h2w ... OK!
Creating file 24.h2w ... OK!
Creating file 25.h2w ... OK!
Creating file 26.h2w ... OK!
Creating file 27.h2w ... OK!
Creating file 28.h2w ... OK!
Creating file 29.h2w ... OK!
Creating file 30.h2w ... OK!
Free space: 0.00 Byte
Average writing speed: 6.11 MB/s
```

`f3read /media/<user>/F3TEST/`:

```text
F3 read 8.0
Copyright (C) 2010 Digirati Internet LTDA.
This is free software; see the source for copying conditions.

                  SECTORS      ok/corrupted/changed/overwritten
Validating file 1.h2w ... 2097152/        0/      0/      0
Validating file 2.h2w ... 2097152/        0/      0/      0
Validating file 3.h2w ... 2097152/        0/      0/      0
Validating file 4.h2w ... 2097152/        0/      0/      0
Validating file 5.h2w ... 2097152/        0/      0/      0
Validating file 6.h2w ... 2097152/        0/      0/      0
Validating file 7.h2w ... 2097152/        0/      0/      0
Validating file 8.h2w ... 2097152/        0/      0/      0
Validating file 9.h2w ... 2097152/        0/      0/      0
Validating file 10.h2w ... 2097152/        0/      0/      0
Validating file 11.h2w ... 2097152/        0/      0/      0
Validating file 12.h2w ... 2097152/        0/      0/      0
Validating file 13.h2w ... 2097152/        0/      0/      0
Validating file 14.h2w ... 2097152/        0/      0/      0
Validating file 15.h2w ... 2097152/        0/      0/      0
Validating file 16.h2w ... 2097152/        0/      0/      0
Validating file 17.h2w ... 2097152/        0/      0/      0
Validating file 18.h2w ... 2097152/        0/      0/      0
Validating file 19.h2w ... 2097152/        0/      0/      0
Validating file 20.h2w ... 2097152/        0/      0/      0
Validating file 21.h2w ... 2097152/        0/      0/      0
Validating file 22.h2w ... 2097152/        0/      0/      0
Validating file 23.h2w ... 2097152/        0/      0/      0
Validating file 24.h2w ... 2097152/        0/      0/      0
Validating file 25.h2w ... 2097152/        0/      0/      0
Validating file 26.h2w ... 2097152/        0/      0/      0
Validating file 27.h2w ... 2097152/        0/      0/      0
Validating file 28.h2w ... 2097152/        0/      0/      0
Validating file 29.h2w ... 2097152/        0/      0/      0
Validating file 30.h2w ... 1671296/        0/      0/      0

  Data OK: 29.80 GB (62488704 sectors)
Data LOST: 0.00 Byte (0 sectors)
	       Corrupted: 0.00 Byte (0 sectors)
	Slightly changed: 0.00 Byte (0 sectors)
	     Overwritten: 0.00 Byte (0 sectors)
Average reading speed: 19.15 MB/s
```

**sector数の整合。**29 file × 2,097,152 ＋ 1,671,296 ＝ 62,488,704 sectorであり、
`Data OK`の値と一致する。512 B/sectorとして31,994,216,448 B ＝ 29.80 GiBである。
**`f3`が`GB`と表示する値は実際にはGiBである**（29.80 × 2^30 が上の byte 数に一致する）。
公称32 GB（10進、＝29.80 GiB）と整合する。

### raw deviceへの直接書込み（③の原因切り分け）

**実施日は2026-08-13である**（`f3`による検査は2026-08-12）。
**目的は、6.11 MB/sのうちfile system（FAT32）が占める分を分離することである。**
`O_DIRECT`でpage cacheを迂回し、file systemを介さずに`/dev/mmcblk0`へ直接読み書きした。
先頭2 GiBが対象である。root権限を要するためhumanが実行した。

**実施順序を先に明示する。**この節は文書上`f3`の出力の後に置いているが、
**実際にはraw device試験の前に次を済ませてある。**

1. `f3`が書いた`*.h2w` 30 fileを削除し、`sync`した
2. **`udisksctl unmount -b /dev/mmcblk0p1`でpartitionをunmountした**
3. その後にraw deviceへの`dd`を実行した

**mount中のpartitionへraw書込みを行ってはいない。**mountしたままだと、
raw書込みの後にfile systemのmetadataがcacheから書き戻され、
試験範囲を上書きしうるためである。下の「事後処理」はこの1〜2を再掲している。

実行したcommandは次のとおりである。`dd`は`dd (coreutils) 9.4`である。
**`/dev/mmcblk0`はこの時点で単一FAT32 partition（unmount済み）であり、この試験がそれを破壊した。**

```bash
sudo sh -c 'echo "=== raw write ==="; dd if=/dev/urandom of=/dev/mmcblk0 bs=1M count=2048 oflag=direct 2>&1 | tail -1; sync; echo 3 > /proc/sys/vm/drop_caches; echo "=== raw read ==="; dd if=/dev/mmcblk0 of=/dev/null bs=1M count=2048 iflag=direct 2>&1 | tail -1'
```

**このcommandの弱点を明記する。**`dd ... 2>&1 | tail -1`はpipelineであり、
**終了statusは`tail`のものになる。**`sudo sh -c`に`set -o errexit`も`pipefail`も付けていないため、
**`dd`が失敗しても後続の`sync`・`drop_caches`・読出しへ進みうる。**
失敗した試験の値を速度として記録してしまう構成である。**次に同じ試験を行うときは
`set -o pipefail`を付けるか、`dd`の終了statusを直接確認する。これを満たさない測定は、
失敗を検出できないため速度の根拠に採らない。**上の実行記録は実際に流したcommandであり、
**事後に書き換えない。**

**今回の結果が失敗でないことは、出力自体から確認できる。**`dd`は完走時に
転送byte数と所要時間を要約行へ出す。記録した2行はいずれも
`2147483648 bytes (2.1 GB, 2.0 GiB) copied`であり、**指定した`bs=1M count=2048`＝2 GiBと一致する。**
途中で失敗していれば転送量がこれを下回る。**ただしこれは事後の確認であって、
commandが失敗を検出する仕組みを持っていたわけではない。**

**各optionの意図。**`oflag=direct`／`iflag=direct`が`O_DIRECT`を指定してpage cacheを迂回する。
`bs=1M count=2048`で2 GiBを対象とする。書込み後に`sync`し、`drop_caches`へ`3`を書いて
page cacheとdentry／inode cacheを落としてから読み出す。**速度は`dd`自身の最終行の報告を採る**
（`tail -1`）。読出しは`of=/dev/null`であり、**書いた内容との照合は行っていない。**
照合は`f3read`が29.80 GiBに対して実施済みであり、この試験の目的は速度の分離である。

**`/dev/urandom`が律速していないことを事前に確認した。**

```text
536870912 bytes (537 MB, 512 MiB) copied, 2.37549 s, 226 MB/s
```

```text
=== raw write ===
2147483648 bytes (2.1 GB, 2.0 GiB) copied, 288.841 s, 7.4 MB/s
--- read ---
2147483648 bytes (2.1 GB, 2.0 GiB) copied, 105.668 s, 20.3 MB/s
```

`/dev/urandom`は事前に226 MB/sを確認しており、律速していない。

| 経路 | 書込み | 読出し |
|---|---|---|
| FAT32経由（`f3write`／`f3read`、`f3`が書いた空き領域29.80 GiB） | 6.11 MB/s | 19.15 MB/s |
| raw device直接（`dd oflag=direct`、先頭2 GiB） | **7.4 MB/s** | 20.3 MB/s |

**結論: FAT32だけでは10 MB/s未達を説明できない。**file systemを完全に外しても7.4 MB/sであり、
**10 MB/sに届かない。**

**「FAT32が主因ではない」とまでは書かない。**また`7.4 ÷ 6.11 ＝ 1.21`は
**raw経路が21%速いことを示すだけであって、FAT32の寄与率ではない。**下記のとおり
2つの測定は複数の条件が同時に異なるため、寄与率を算出できる比較になっていない。

**この比較の限界を明記する。**2つの測定はfile systemの有無だけでなく、
**I/O方式（`O_DIRECT`の同期書込み対 page cache経由のwriteback）と対象範囲
（先頭2 GiB対 全域）も異なる。**したがってFAT32の寄与を厳密に切り出したのではなく、
**上界を与えたにとどまる。**それでも「file systemを外しても10 MB/sに届かない」という
結論は、この差異に影響されない。

### 対照測定（2026-08-15）

**別cardと block size を変えた測定。**③の原因を切り分けるために実施した。
いずれもfile system経由（`conv=fsync`）であり、書込み後に一時fileを削除している。

対照cardの識別情報（`/sys/block/mmcblk0/device/`）。

```text
type                   SD
manfid                 0x000003
oemid                  0x5344
name                   SU16G
hwrev                  0x8
fwrev                  0x0
date                   08/2010
scr                    0235800000000000
ssr                    0000000004000000010190000b050000（以降すべて0）
csd                    400e00325b59000076b27f800a404000
preferred_erase_size   4194304
```

`ssr` byte 8＝`0x01`（`SPEED_CLASS`＝Class 2）、byte 14＝`0x00`（`UHS_SPEED_GRADE`＝0）。
`manfid`＝`0x03`、`oemid`＝`0x5344`（ASCII `"SD"`）。**`manfid`の社名対応は前述のとおり
一次資料が無いため、メーカー名は判定に使わない。**

Samsung（`SD01` label の空FAT32）へのblock size別書込み。

```text
4K    268435456 bytes (268 MB, 256 MiB) copied, 44.982 s, 6.0 MB/s
64K   268435456 bytes (268 MB, 256 MiB) copied, 48.3193 s, 5.6 MB/s
1M    268435456 bytes (268 MB, 256 MiB) copied, 49.145 s, 5.5 MB/s
8M    268435456 bytes (268 MB, 256 MiB) copied, 48.4426 s, 5.5 MB/s
```

SanDisk `SU16G`（既存のext4 partition上の`/tmp`へ書き、測定後に削除）。

```text
1M   134217728 bytes (134 MB, 128 MiB) copied, 24.8249 s, 5.4 MB/s
8M   134217728 bytes (134 MB, 128 MiB) copied, 25.9639 s, 5.2 MB/s
```

SanDiskの読み出し（既存fileを`cat`で通読。**一部fileはpermissionで読めず、
読めた257,498,296 bytesで算出**）。

```text
読み出し: 257498296 bytes / 29.54 s = 8.7 MB/s
```

**対照cardは検査していない。**このcardにはRaspberry Pi向けUbuntu imageが入っており、
**破壊的な検査を行っていない。**書込みは空き領域への一時fileに限り、測定後に削除した。

### 事後処理

- `f3`の書いた`*.h2w` 30 fileを削除し、partitionをunmountした
- **その後のraw device試験により、cardには現在partition tableもfile systemも無い。**
  先頭2 GiBが乱数で上書きされ、`lsblk`は`mmcblk0 29.8G`のみを表示する（partitionなし）。
  **Piで使うにはimageの書き込みかformatが要る。本記録の範囲外である**
- 検査前に入っていたRockchip向けUbuntu 18.04 imageは、cardの全容量を検査対象にするため消去した
  （消去前に内容を提示し、humanが「消えてもよい」と判断した）
- **一時導入した`f3`をhumanが撤去した**（`apt remove --purge f3` ＋ `apt autoremove`）。
  撤去後に`command -v f3write`が空を返し、`dpkg -l f3`にpackage登録が無いことを確認した
  （[AGENTS.md](../../AGENTS.md)「出所不明のskill、plugin、rule、スクリプトを自動導入しない」
  および作業指示「一時導入したtoolは作業後に撤去する」）

## 判定

**`Partial`。**①②⑤は基準を満たしたが、③は判定不能で、④は取得手段が無い。

**③の「判定不能」と「未達」を分けて書く。**測定値6.11 MB/sは基準値10 MB/sに達していない。
これは事実である。**しかしこの測定はcardが申告する2つの規定のどちらの条件も満たしていない**
（`Class 10`はHigh Speed Modeで40 MHz、`U1`はUHS bus mode。hostは`sd high-speed`の33 MHz）。
**したがって「cardが要件を満たさない」という判定は下せない。**`未達`という語は有効な条件での
失敗を指すため、この表では使わない。**有効な条件で測定した場合にのみ`未達`と判定する。**

| # | 観点 | 基準 | 実測 | 判定 |
|---|---|---|---|---|
| ① | 容量詐称 | `Data LOST`＝0 sector | 0 sector（3区分とも0） | **合格** |
| ② | 読み書きの通し | 書込み総量と`Data OK`が一致 | 30 file・29.80 GiB・62,488,704 sectorが一致 | **合格** |
| ③ | 速度 ≥ 10 MB/s | `f3write`平均書込み速度 | **6.11 MB/s**（raw device直接でも**7.4 MB/s**） | **判定不能（測定条件外）。**hostのclockが33 MHzで、`Class 10`の測定条件40 MHzにも`U1`の条件（UHS mode）にも達しない。**この端末ではcardの速度を判定できない** |
| ④ | SMART相当 | 取得手段の有無を確認 | **この個体には手段が存在しない**（SD Expressではないため） | 確認済み |
| ⑤ | 識別情報 | 現物printと矛盾しない | `32`／`microSDHC`／`U1`はregisterと一致（**読み方を仕様書Ver 9.10と照合済み**）。**`Samsung`だけは未裏付け**（MIDの登録簿が仕様書に無い） | **合格（1項目のみ未裏付け）** |

**確認できたのは次の2点である。**`f3`が書き込んだ全空き領域（29.80 GiB）へ位置依存dataを
書いて読み戻し、`Corrupted`／`Slightly changed`／`Overwritten`のいずれも0 sectorであった。

- **容量詐称は確認されなかった。**公称容量に見合う領域へ位置依存dataを書き、巻き戻りなく読み戻せた
- **検査範囲に読み書き不良は確認されなかった**

**「偽造品ではない」とは書かない。**この検査が見たのはfile systemの空き領域だけであり、
**metadata領域と予約領域は検査していない。**また`Samsung`というメーカー表示は
registerで裏付けられていない（`manfid`の登録簿が仕様書に無い）。
**「容量詐称が確認されなかった」ことと「偽造品でない」ことは同じではない。**

### ③を判定不能とする理由

**「たぶん正常」で通さない**（[AGENTS.md](../../AGENTS.md) 推測禁止）。

**結論を先に書く。この測定はcardの速度を判定できる条件を満たしていない。**
hostのSD clockが33 MHzであり、**仕様書が`Class 10`の測定条件として定める40 MHzを下回る**ためである。

#### `Class 10`の測定条件を満たしていない（決定的な理由）

このcardは`SPEED_CLASS`＝`04h`（**Class 10**）と`UHS_SPEED_GRADE`＝`1h`（**`U1`**）の
2つを申告している。**両者は別の規定であり、測定条件も別である。**

| 申告 | 要求性能 | 測定条件 | 出典 |
|---|---|---|---|
| `SPEED_CLASS`＝Class 10 | `Pw min.` **10 MB/sec** | **High Speed Modeで40 MHz** | Table 4-61／4-62 |
| `UHS_SPEED_GRADE`＝1 | 10MB/sec and above | **UHS-I／UHS-II bus mode** | Section 4.13.3、Table 4-52 |

**本測定のhostは`sd high-speed`・33 MHzである。**したがって、

- **`U1`の条件（UHS mode）を満たさない。**`ios`の`timing spec`は`sd high-speed`、
  `signal voltage`は3.30 Vであり、`dmesg`も`mmc0: new high speed SDHC card`と記録している
  （UHS modeで初期化されていれば`ultra high speed`と表示される）
- **`Class 10`の条件（High Speed Modeで40 MHz）も満たさない。**mode は合っているが
  **clockが33 MHzで40 MHzに届かない**

仕様書は測定条件について次のとおり明記している（Section 4.13.1.8.1 Application Note）。

> Host needs to use higher frequency clock than that of measurement condition.

**hostは測定条件より高いclockを使う必要がある。**33 MHzは40 MHzを下回るため、
**この端末ではそもそも`Class 10`の性能を測れない。**

**したがって6.11 MB/sという値は、cardが規定を満たさないことを示さない。**

#### 別cardでの対照測定

**2026-08-15に別のcardを同じreaderで測った。**目的はhost controllerがどこまで出せるかの確認である。

| | Samsung EVO Plus 32GB | SanDisk `SU16G` |
|---|---|---|
| `SPEED_CLASS` | `04h`（**Class 10**） | `01h`（**Class 2**） |
| `UHS_SPEED_GRADE` | `1h`（`U1`） | `0h`（10MB/sec未満） |
| 製造 | 07/2018 | 08/2010 |
| **読み出し** | **19.15〜20.3 MB/s** | **8.7 MB/s** |
| **書込み** | **5.5〜6.1 MB/s** | **5.2〜5.4 MB/s** |

**読み出しはcardによって2倍以上違う。**host controllerが固定の上限を課しておらず、
cardの能力に追随していることを示す。

**書込みは2枚とも5〜6 MB/sで並ぶ。**規格上2階級違うcardが同じ値になるのは、
**hostのclock不足が両者に共通して効いている**とみると整合する。
なお**SanDiskは`Class 2`の要件（20 MHz Default Speedで`Pw min.` 2 MB/sec）を満たしている。**
Samsungだけが自分のclassの条件下で測られていない。

#### block size依存性

file system経由で書込みblock sizeを変えても速度が変わらない。

| block size | 4K | 64K | 1M | 8M |
|---|---|---|---|---|
| 書込み | 6.0 MB/s | 5.6 MB/s | 5.5 MB/s | 5.5 MB/s |

**block sizeを2000倍変えても差が出ない。**host controllerのper-command overheadが
律速なら4Kは8Mより大幅に遅くなるはずであり、そうならない。
**律速はcommand発行の回数ではなく持続throughput側にある。**

#### file systemの寄与

raw deviceへ直接書いても7.4 MB/sで10 MB/sに届かないため、
**FAT32だけでは未達を説明できない**（上の「raw deviceへの直接書込み」）。
**ただしこれは「FAT32が主因ではない」ことを示すものではない。**
2つの測定は条件が複数異なり、寄与率を出せる比較になっていない。

#### 結論

**「cardの劣化・不良」とは結論しない。**測定条件が規定を満たしていないためである。

**「cardは仕様どおり」とも結論しない。**規定の条件下で測っていない以上、
満たすことも確認できていない。

**この端末では③を判定できない。**判定するには40 MHzを超えるHigh Speed Mode
（`Class 10`の確認）か、UHS-I bus mode（`U1`の確認）で駆動できるhostが要る。
別のcardを用意しても解決しない。

#### ③の追跡を打ち切る

**別のhostを用意して測り直すことはしない。**理由は次の2点である。

**1. ③は何もblockしていない。**`HW-TBD-015`の`妨げる対象`は**耐久性**であり、
速度は含まれない。`SD-01`の用途はRaspberry Piのbootとstorageであり、
そこで効くのは①（容量詐称なし）と②（検査範囲に読み書き不良なし）である。
**両方とも合格している。**書込み速度が影響するのはimage書き込みに要する時間だけである。

**2. 作業指示は③に「検討」を求めていた。**元の指示は
「最低限、次の観点を**検討する**」であり、確定を求めていない。
**検討した結果が「この端末の測定条件では判定できない」であって、それが答えである。**

**したがって③は`判定不能`のまま確定とし、残作業として持ち越さない。**
将来この端末以外でcardを使う機会があれば、そのとき自然に分かる。

**raw device試験で分かったのは「file systemを外しても届かない」という一点である。**
card対hostの分離は、別のreaderを用意しない限りこの端末では進められない。

### この判定が及ぶ範囲

**この記録が示すのはhost側の結果だけである。**測定はx86_64のUbuntu host上で、
Ricoh `1180:E823` SDHCI controllerを介して行った。**Piでの動作は確認していない。**

**確認できたこと。**①②により、`f3`が書き込んだ全空き領域で容量詐称が確認されず、
読み書き不良も確認されなかった。**これはcardをPiのbootとstorageに使う前提として
必要な条件であり、この試験環境ではそれを満たしている。**

**確認していないこと。**次はいずれもこの記録の範囲外である。

- **PiへのOS imageの書き込み**
- **Piでのbootと動作**
- **PiのSD host controllerとの組合せでの互換性・速度。**本記録のbus mode
  （`sd high-speed`・3.30 V signaling）はこのhostのものであり、Piでの動作modeとは別である
- **③の速度。**`U1`が規定するUHS mode条件外の測定であり判定不能である
  （書込み速度はimage書き込みに要する時間に影響するが、この記録では評価していない）

**「Piのbootに使ってよい」という判断は、この記録だけでは下せない。**
下すにはPiでの書き込みとbootの確認が要る。

### 実施しなかった項目

**skipを成功として記録しない。**

| 項目 | 状態 | 理由 |
|---|---|---|
| literalな全セクタの読み書き検査 | **実施しなかった** | `badblocks -w`はroot必須かつ完全破壊であり、①（容量詐称）を構造的に検出できない。①を優先して`f3`を選んだ。全セクタのliteralなカバーが要る場合は別途実施する |
| SMART相当の情報取得 | **手段が存在しない** | この個体の接続経路にSMART相当のcommandが無い（上記④）。実施可能だが行わなかった、ではない。**なおSD Express（NVMe interface）にはSMARTがあるが、この個体はSD Expressではない** |
| 耐久性（寿命）の評価 | **この記録の対象外。求められてもいない** | 1回のhealth checkで書き込み寿命は判定できない。**なお`HW-TBD-015`の`妨げる対象`にある「耐久性」は、同行が未解決だと妨げられる対象であって測定項目ではない。**同行の`必要な根拠`は`health checkの実施`だけであり、耐久性の測定は求められていない（[PR #128](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/128)で誤読を訂正した） |
| **規定の条件下での速度測定** | **実施しない（追跡を打ち切った）** | ③を判定できない直接の原因は**hostのSD clockが33 MHzで、`Class 10`の測定条件40 MHz（High Speed Mode）にも`U1`の条件（UHS bus mode）にも達しない**ことである（Table 4-61、Section 4.13.3）。**ただし③は何もblockしていない**（`妨げる対象`は耐久性であり速度を含まない）。**別hostを用意して測り直すことはしない** |
| 書込み速度の各要因の寄与率 | **実施しなかった** | raw device試験で**FAT32だけでは未達を説明できない**ことは示したが、card・host controller・file systemそれぞれの寄与率は出していない（2つの測定は条件が複数異なる） |
| `manfid`＝`0x1b`がSamsungであることの確認 | **この経路では解ける見込みが無い** | 仕様書もSD-3Cも**MIDと社名の対応表を公開していない**（2026-08-15確認）。**規格を定める側と割り当てる側の両方が公開していないため、追跡を打ち切った。網羅的に探したわけではない。****`SD-01`のメーカー表示の正は引き続き現物printである** |
| host controllerのUHS-I対応可否の確定 | **実施しなかった** | `/sys/kernel/debug/mmc0/caps`／`caps2`がroot権限でも`EPERM`を返すため、capability registerを読めなかった。**測定がUHS modeでないことは`ios`と`dmesg`で確定しているが、hostが非対応なのかnegotiationが成立しなかっただけなのかは区別していない** |

## repository検証

[Version Record Template](../toolchains/version-record-template.md)の`Commands run`／`Expected result`／
`Actual result`に相当する記録である。**この記録を載せるcommitに対して実行した。**
本作業の変更はMarkdown 4 fileのみで、Rust codeに触れていない。
**したがって下記の多くは「変更していないことの回帰確認」である。**

| 確認 | command | 結果 |
|---|---|---|
| 空白・改行の異常 | `git diff --check origin/develop..HEAD` | **問題なし**（出力なし） |
| 文書link検査 | `python3 scripts/validate_doc_links.py` | **成功。**`MARKDOWN=77 LINKS=623 BROKEN=0`（**この節を追加した後の値である。**節内のlinkも計上されるため、追加前は618であった） |
| 公開guard test | `python3 scripts/test_pages_guards.py` | **成功。**26件 |
| link validator test | `python3 scripts/test_link_validators.py` | **成功。**69件 |
| Format | `cargo fmt --all -- --check` | **成功。**差分なし |
| Lint | `cargo clippy --workspace --all-targets --locked` | **成功。**warning 0件 |
| Unit test | `cargo test --workspace --locked` | **成功。**unittests 0件、doc-test 2件 |
| Host integration test | 同上 | **成功。**`tests/conformance.rs` 6件、`tests/limits.rs` 5件。**合計13件合格・0件失敗** |
| Markdown lint | `markdownlint` | **未実行。**この端末に未導入である。**repositoryもCIもこのtoolを使っていない** |
| ESP32 build | — | **N/A。**`firmware/`に触れていない。ESP32 toolchainもこの端末に無い |
| 実機試験 | — | **本記録そのものが実機試験の記録である**（`f3`とraw device試験） |
| 統合・回帰試験 | — | **N/A。**hardware記録の追加であり、実行対象のsoftware動作を変えていない |

**host workspaceの3 commandを実行するにあたり、この端末へRustを導入した。**
実行時点で`cargo`が無く、humanの確認を得たうえで`rustup`経由のstableと
`build-essential`を導入した（[AGENTS.md](../../AGENTS.md)「ツール導入は、対象Issue、
端末profile、人間の確認が揃った開発端末だけで行う」）。版は
[2026-08-10-host-rust-linux.md](../toolchains/version-records/2026-08-10-host-rust-linux.md)と
一致した（rustup 1.29.0、rustc／cargo 1.97.1）。
**ただし同recordはVM上の記録であり、本端末は実機である。**
`Container / VM / native:`が異なるため、**本端末のHost Rust Development profileの
version recordは別途要る。本記録の範囲外であり作成していない。**

## Revision履歴

| 日付 | Revision | 変更 | 根拠 |
|---|---|---|---|
| 2026-08-12 | 0 | 文書を新設し、**実測より前に**判定基準と実施環境を確定した | [tbd-register.md](tbd-register.md) `HW-TBD-015`、[hardware-bom.md](hardware-bom.md) `SD-01` |
| 2026-08-12 | 1 | 実測を行い結果と判定（`Partial`）を記入した。**あわせて、Revision 0の作業中に一度書いた「bus帯域の上限は33 MHz × 4 bitで約16.5 MB/s」という導出を撤回した。**読み出し実測19.15 MB/sがこの値を超えており、導出が誤っていた。`ios`の`clock`は取得時点の値であって転送中の実効clockと同一である保証がない。**この記録はbus帯域の上限を計算で示さず、実測値だけを根拠に用いる** | 本記録の実測結果 |
| 2026-08-12 | 2 | **自己レビューで検出。Revision 1は、registerのbit解釈を一次資料と照合せずに確定形で書いていた**（`SPEED_CLASS`＝Class 10、`UHS_SPEED_GRADE`＝`U1`、CSD Version 2.0 など）。**これは[AGENTS.md](../../AGENTS.md)の推測禁止に反し、この repository が繰り返し是正してきた誤りと同じ型である。**識別情報の節を「読み取った値（観測）」と「解釈（一次資料と未照合）」へ分割し、判定基準⑤を`合格`から`保留`へ改めた。**あわせて、⑤に依存しないこと（①②③④はregisterの解釈を使わずに判定できる）を明記した。**照合できなかった経緯（`dl.php`が0 byteを返す、ブラウザ取得は利用規約への同意を要する）も記録した。`hardware-bom.md`と`tbd-register.md`の「裏取りした」という記述も同時に訂正した。**あわせて判定基準③の説明文から「High Speed（25 MHz）」という未照合のclock値を削除した。判定の閾値10 MB/sはRevision 0から変えていない** | 自己レビュー、[SD Association Simplified Specifications](https://www.sdcard.org/downloads/pls/) |
| 2026-08-13 | 3 | **一次資料を入手して照合し、Revision 2で`保留`とした解釈を確定させた。**humanが利用規約に同意してPDFを取得した。**Revision 2の解釈は`manfid`を除きすべて正しかった**（`CSD_STRUCTURE`／`SD_SPEC`系／`SPEED_CLASS`／`UHS_SPEED_GRADE`／`AU_SIZE`／`OID`／`PNM`）。byte位置の対応も`AU_SIZE`と`preferred_erase_size`の一致で独立に裏付けた。判定基準⑤を`保留`から`合格`へ戻した。**ただし`manfid`＝`0x1b`＝Samsungだけは確定できない。**仕様書はMIDの登録簿を収録していないためである。**メーカー表示の正は引き続き現物printである。****あわせて③をinconclusiveとする根拠を強化した。**Section 4.13.3の表題が`Speed Grade Specification for UHS-I and UHS-II`であり、`U1`の10 MB/sがUHS bus modeに対する規定であることを一次資料で確認した。**一方で「hostがUHS-I非対応」は確定していないことを明記した**（capability registerがroot権限でも読めない） | [Part 1 Physical Layer Simplified Specification Version 9.10](https://www.sdcard.org/downloads/pls/)（2023-12-01）Table 4-45／4-47／4-52／5-3／5-19、Section 4.10.2.8／4.13.3／5.1 |
| 2026-08-13 | 4 | **raw deviceへの直接書込みを実施し、③の原因から一つを消した。**`O_DIRECT`でfile systemとpage cacheを迂回した結果は書込み7.4 MB/s、読出し20.3 MB/sである。**file systemを完全に外しても10 MB/sに届かないため、6.11 MB/sの主因はFAT32ではないと確定した。**当初挙げた3要素（card／host controller／file system）のうちfile systemが消え、**残るのはcardとhost controllerの分離だけになった。**これは別のhost controllerで同じcardを測らない限り進まない。**比較の限界も明記した**（2つの測定はfile systemの有無だけでなくI/O方式と対象範囲も異なるため、FAT32の寄与を厳密に切り出したのではなく上界を与えたにとどまる）。**あわせてcardの現状を更新した。**raw試験によりpartition tableとfile systemが失われている | 本記録のraw device試験 |
| 2026-08-13 | 5 | **自己レビューround 4で検出。Revision 4までは「SMARTはATA／NVMeの機能であり、SD cardには相当する標準interfaceが無い」と書いていたが、これは言い過ぎであった。**手元の仕様書を`SMART`で検索したところ**Section 8.4.7（SD Express CardのPower and Thermal Management）に該当があり**、hostが`SMART / Health Information Log`から得たcomposite temperatureを使ってよいと記されている。**SD ExpressはNVMe interfaceを持つためSMARTが利用できる。**主張の範囲を個体に限り、「この個体の接続経路にSMART相当のcommandが無い」へ改めた。**この個体はSD Expressではない**（`scr`はPhysical Layer Version 3.0X、hostは`sd high-speed`接続）ため、**④の判定`手段が存在しない`は変わらない。****一次資料を手元に置いた状態で、自分が以前に断定した否定命題を検索し直して見つけた誤りである** | [Part 1 Physical Layer Simplified Specification Version 9.10](https://www.sdcard.org/downloads/pls/) Section 8.4.7、自己レビュー |
| 2026-08-13 | 6 | **自己レビューround 6で検出した日付と基準commitの記述3件を直した。**(a) ヘッダの`実施日`が2026-08-12だけを挙げ、**raw device試験も2026-08-13に行ったことを落としていた。**(b) raw device試験の節に**日付が無く**、`f3`による検査（08-12）と読者が区別できなかった。(c) `Repository commit`が`6f387b6`のままで、rebase後のbase（`cbd6fa7`）と食い違って見えた。**これは測定時点のtreeを指す値であり誤りではない**が、その旨を明記した。測定はrepositoryの内容に依存しないため、rebaseは結果に影響しない | 自己レビュー |
| 2026-08-13 | 7 | **[PR #115](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/115)のCodeRabbit full reviewの指摘5件を反映した。**(a) `f3write`の出力を2〜29番だけ省略していた。**節自身が「出力全文を埋め込む」と書いており矛盾していたため、30行すべてを未加工で掲載した。**(b) raw device試験の**実行commandを記録していなかった**ため再現できなかった。`dd (coreutils) 9.4`の版とcommand全文、各optionの意図、`/dev/urandom`の事前測定を追記した。(c) **Revision 4が書いた「6.11 MB/sの主因はFAT32ではないと確定した」は過剰な断定であった。**同じ節が「上界を与えたにとどまる」と書いており、文書内で矛盾していた。`7.4 ÷ 6.11 ＝ 1.21`も**raw経路が21%速いことを示すだけでFAT32の寄与率ではない。**「FAT32だけでは10 MB/s未達を説明できない」へ改め、寄与率を算出しないことを明記した。`hardware-bom.md`と`tbd-register.md`へも波及させた。(d) **Revision 1以来**③の判定を`未達（inconclusive）`としていた。**`未達`は有効な条件での失敗を指すため、UHS mode条件外の測定に使うのは誤りである。**`判定不能（UHS mode条件外）`へ改め、測定値が基準値に達していないこととcardがU1要件を満たさないことは別である旨を明記した。(e) **Revision 1以来**検査範囲を「29.80 GiB全域」と書いていたが過大表現であった。同文書が`f3`はfile systemの空き領域だけを書くと明記しているため、「`f3`が書き込んだ全空き領域（29.80 GiB）」へ改め、3文書で表現を揃えた | [PR #115のCodeRabbit review](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/115) |
| 2026-08-13 | 8 | **[PR #115](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/115)のCodeRabbit再reviewの指摘3件のうち2件を反映した。**(a) **raw device試験の節が、unmountをいつ行ったか読み取れない構成だった。**`f3`のfile削除とunmountは実際にはraw試験の前に済ませてあるが、それを記した「事後処理」節を後ろに置いていたため、**mount中のpartitionへraw書込みをしたように読めた。**実施順序を同節の冒頭へ明示し、mountしたまま行っていないことと、その理由を書いた。**手順自体は変えていない。文書の構成の問題である。**(b) **Revision 1が書いた「偽造品ではない」は断定しすぎであった。**この検査が見たのはfile systemの空き領域だけで、metadata領域と予約領域は検査しておらず、`Samsung`というメーカー表示もregisterで裏付けられていない。**「容量詐称は確認されなかった」「検査範囲に読み書き不良は確認されなかった」の2点へ言い換え、両者が「偽造品でない」と同じではないことを明記した。**`hardware-bom.md`と`tbd-register.md`の表現も揃えた。**(c) 残る1件（「2026-08-13の結果を2026-08-12基準日で載せるな」）は反映していない。**指摘が前提とする基準日が誤っており、**実施日2026-08-13は実際の日付である**（未来日付ではない）。詳細はPRのthreadに記した | [PR #115のCodeRabbit review](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/115) |
| 2026-08-15 | 9 | **自己レビューで検出。Revision 7と8が、訂正した記述をどのRevisionが書いたのか明示していなかった。**この repository の慣行（[hardware-bom.md](hardware-bom.md)の「Revision 29が〜」「Revision 31は〜」）と揃っていない。訂正元をRevision 1（「偽造品ではない」「未達（inconclusive）」「29.80 GiB全域」）とRevision 4（「主因はFAT32ではないと確定した」）へ明示した。**帰属先はいずれもgit履歴で実在を確認しており、推測で番号を書いていない** | 自己レビュー |
| 2026-08-15 | 10 | **[PR #115](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/115)のCodeRabbit再reviewの指摘6件を反映した。**(a) `mkfs.vfat`／`udisksctl`／`parted`／`wipefs`のversionが無かった。package版で追記し、**`mkfs.vfat`が`--version`を受け付けないため package版を記した**旨も書いた。(b) **registerとbus modeの取得commandを記録しておらず、別のoperatorが観測値を再現できなかった。**`実行したcommand`節を新設し、識別情報・`ios`・`dmesg`・format・mount・後片付けのcommandを載せた。**`caps`／`caps2`がroot権限でも`EPERM`で取得できなかったことも明記した。**(c) **raw device試験の`dd`が失敗を検出できない構成だった。**`dd`の出力を`tail -1`へ渡す構成はpipelineで終了statusが`tail`のものになり、`pipefail`も`errexit`も付けていない。**この弱点を明記し、次回は`set -o pipefail`か終了statusの直接確認を行うこととした。**あわせて今回の結果が失敗でないことを、転送byte数が指定値と一致することから示した。**ただしこれは事後確認であって、commandが失敗を検出する仕組みを持っていたわけではない。**(d) **「page cacheでは19.15 MB/sを説明できない」は言い過ぎであった。**29.80 GiBが3.7 GiBのRAMを超えることが示すのは「全dataがcacheだけで処理された結果ではない」ことまでである。page cacheが一部に寄与していないことは示していない（uncached baselineもcache hit率も測っていない）。表現を弱め、raw読出しがcacheを外して20.3 MB/sであったことを併記した。(e) **「この判定でPiのbootに使ってよい」は、host固有の試験結果からdeployの可否を導いていた。**測定はx86_64 Ubuntu host上のRicoh SDHCI controller経由であり、**Piへのimage書き込み、Piでのboot、PiのSD host controllerとの互換性はいずれも未確認である。**節を`この判定が及ぶ範囲`へ改め、確認できたことと確認していないことを分けた。`hardware-bom.md`にも波及させた。(f) `hardware-bom.md`と`tbd-register.md`が2つの実施日を`2026-08-12`へ畳んでいた。`f3`による検査（08-12）とregister照合・raw device試験（08-13）を書き分けた | [PR #115のCodeRabbit review](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/115) |
| 2026-08-15 | 11 | **[PR #115](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/115)のCodeRabbit reviewの指摘を反映し、`repository検証`節を新設した。****この記録は[Version Record Template](../toolchains/version-record-template.md)の様式に倣うと書きながら、同templateの`Commands run`／`Expected result`／`Actual result`に相当する節を持っていなかった。**hardware試験の未実施理由は書いていたが、repository側の検証（format、lint、test、link検査）の結果がどこにも無かった。実行結果と、`markdownlint`が未導入で未実行であること、ESP32 buildと統合・回帰試験が`N/A`である理由を記録した。**あわせて、この端末へRustを導入した経緯と、本端末のversion recordが別途要ることも明記した** | [PR #115のCodeRabbit review](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/115)、[Version Record Template](../toolchains/version-record-template.md) |
| 2026-08-15 | 12 | **`manfid`の追跡を打ち切った。**Revision 3以来「別の一次資料が要る」としていたが、**2026-08-15に[SD-3C Licensees](https://www.sd-3c.com/Licensees.aspx)を確認したところ、同社もMIDと社名の対応表を公開していない**（ライセンシーの社名一覧のみ）。仕様書Ver 9.10も対応表を収録していない。**規格を定める側とMIDを割り当てる側の両方が公開していないため、この経路に見込みは無い。****ただし網羅的に探したわけではないので「一次資料が存在しない」とは書かない。**状態を`未照合`から`この経路では解ける見込みが無い`へ改めた。**非公式の実測集計が`0x1b`＝Samsung・OEM ID `534d`とし本個体の値と一致するが、著者自身が非公式と明記しているため根拠に採らない。****`SD-01`のメーカー表示の正は引き続き現物printである** | [SD-3C Licensees](https://www.sd-3c.com/Licensees.aspx)、[Part 1 Physical Layer Simplified Specification Version 9.10](https://www.sdcard.org/downloads/pls/) Section 5.1 |
| 2026-08-15 | 13 | **自己レビューで検出。Revision 12が「`manfid`から社名を引く一次資料は存在せず」と全称否定で書いていた。****確認したのは仕様書とSD-3Cの2つであり、網羅的に探したわけではない。**「規格を定める側と割り当てる側の両方が公開していないため、この経路に見込みは無い」へ改め、**「存在しないことを証明した」からではなく「見込みが無い」から打ち切る**という区別を明記した。状態名も`一次資料では解けない`から`この経路では解ける見込みが無い`へ揃えた。**これはRevision 5で同じ型の誤り（SMARTの全称否定）を直したばかりであり、繰り返している** | 自己レビュー |
| 2026-08-15 | 14 | **[PR #118](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/118)のCodeRabbit reviewの指摘2件を反映した。**(a) 見出し「MIDの対応表は公開されていない」が、**未確認の資料にも対応表が無いと読める表現だった。**後続の本文は非網羅的な確認であると断っていたが、**見出しがその限定を打ち消していた。**「確認した2つの資料はいずれもMIDの対応表を持たない」へ改めた。(b) [tbd-register.md](tbd-register.md)の`HW-TBD-015`行が`Samsung`を裏付けられない事実だけを記し、**照合を打ち切ったことを書いていなかった。**同じ行の[#117](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/117)が`manfid`照合を継続するIssueにも読めたため、**打ち切りの事実と、#117の範囲が耐久性・速度要因の分離・結果記録・close判断に限られることを明記した** | [PR #118のCodeRabbit review](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/118) |
| 2026-08-15 | 15 | **③の原因を特定した。判定は`判定不能`のまま変わらないが、理由が変わった。****Revision 14までは`U1`（`UHS_SPEED_GRADE`）の条件だけを見ており、同じcardが申告する`SPEED_CLASS`＝Class 10の測定条件を調べていなかった。**仕様書Table 4-61は**`Class 10`をHigh Speed Modeの40 MHzで測ると定め**、Table 4-62が`Pw min.`10 MB/secを課す。Section 4.13.1.8.1のApplication Noteは`Host needs to use higher frequency clock than that of measurement condition.`と明記する。**本測定のhostは`sd high-speed`の33 MHzであり、40 MHzに達しない。**したがって`U1`の条件（UHS mode）だけでなく**`Class 10`の条件も満たしていない。****この端末ではcardの速度を判定できない。別cardを用意しても解決しない。****あわせて対照測定を実施した。**別card（SanDisk `SU16G`、Class 2）を同じreaderで測ると読み出し8.7 MB/s（Samsungは19〜20 MB/s）で、**host controllerは固定の上限を課していない。**一方**書込みは2枚とも5〜6 MB/sで並ぶ**（規格上2階級違うのに同値）。block sizeを4K〜8Mで変えても差が出ず、**律速はcommand発行回数ではなく持続throughput側にある。****Revision 4が「当初挙げた3要素のうちfile systemが消えた」と書いた整理も、原因がhostのclock不足である以上、不完全であった** | [Part 1 Physical Layer Simplified Specification Version 9.10](https://www.sdcard.org/downloads/pls/) Table 4-61／4-62、Section 4.13.1.8.1、本記録の対照測定 |
| 2026-08-15 | 16 | **③の追跡を打ち切った。**Revision 15は原因（hostのclock不足）を特定したが、**「40 MHz超のhostで測り直す」を残作業として残していた。これをやめる。**(1) **③は何もblockしていない。**`HW-TBD-015`の`妨げる対象`は耐久性であり速度を含まない。`SD-01`の用途であるPiのbootとstorageで効くのは①②であり、両方とも合格している。(2) **作業指示は③に「検討」を求めていた**のであって確定を求めていない。**検討した結果が「この端末の測定条件では判定できない」であり、それが答えである。****測定結果と原因の記録は残し、残作業としては持ち越さない** | 作業指示、[tbd-register.md](tbd-register.md) `HW-TBD-015`の`妨げる対象` |
| 2026-08-15 | 17 | **`HW-TBD-015`をcloseした。**同行の`必要な根拠`は**`health checkの実施`だけ**であり、本記録がそれを満たしている。**`妨げる対象`にある「耐久性」は、同行が未解決だと妨げられる対象であって、測定すべき項目ではない。****2026-08-15までこれを「耐久性を別途測定する」と読み違え、同行をopenのまま残し、[#117](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/117)を不要に起票していた。**読み違えを訂正し、行を`解決済み項目`へ移した | [tbd-register.md](tbd-register.md)の`HW-TBD-015`原文（`必要な根拠`列）、[解決手順](tbd-register.md)1〜8 |
