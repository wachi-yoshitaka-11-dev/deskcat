# microSD（SD-01）health check記録

> 状態: 実施済み
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
  util-linux (lsblk): 2.39.3
  検査用file system: FAT32（mkfs.vfat -F 32）。単一partitionへ再formatした
```

**`f3`の一時導入について。**[AGENTS.md](../../AGENTS.md)は「ツール導入は、対象Issue、端末profile、
人間の確認が揃った開発端末だけで行う」と定める。対象Issueは
[#114](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/114)、導入と撤去はhumanが実行した。
撤去の確認は下の「実測結果」に記載する。

## 実測結果

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
| `manfid`＝`0x1b`＝Samsung | **未照合。**仕様書はMIDを「SD-3C, LLCが管理・定義・割り当てる」8 bitの番号と規定するのみで、**どの番号がどのメーカーかという登録簿を収録していない**（Section 5.1）。**この仕様書では検証できない。**別の一次資料が要る |

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
これは事実である。**しかしこの測定は`U1`が規定する条件（UHS mode）で行っていないため、
「cardがU1要件を満たさない」という判定は下せない。**`未達`という語は有効な条件での
失敗を指すため、この表では使わない。**有効なUHS mode試験を行った場合にのみ`未達`と判定する。**

| # | 観点 | 基準 | 実測 | 判定 |
|---|---|---|---|---|
| ① | 容量詐称 | `Data LOST`＝0 sector | 0 sector（3区分とも0） | **合格** |
| ② | 読み書きの通し | 書込み総量と`Data OK`が一致 | 30 file・29.80 GiB・62,488,704 sectorが一致 | **合格** |
| ③ | 速度 ≥ 10 MB/s | `f3write`平均書込み速度 | **6.11 MB/s**（raw device直接でも**7.4 MB/s**） | **判定不能（UHS mode条件外）。**基準値には達していないが、`U1`が規定する条件での測定ではないため、cardがU1要件を満たさないことを意味しない |
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

**「たぶん正常」で通さない**（[AGENTS.md](../../AGENTS.md) 推測禁止）。次の3点が同時に成り立つ。

1. **`U1`の10 MB/sはUHS-I／UHS-II bus modeに対する規定であり、この測定はその条件を満たしていない。**
   **これは一次資料で確認した。**Physical Layer Simplified Specification Ver 9.10の
   **Section 4.13.3の表題は`Speed Grade Specification for UHS-I and UHS-II`**であり、
   Speed Grade 1（10MB/sec and above）の性能要件はこの節に置かれている。
   また`UHS_SPEED_GRADE`自体が**「UHS mode」のSpeed Gradeを示すfield**と定義されている
   （Section 4.10.2.8）。
   **この測定はUHS modeで行っていない。**`ios`の`timing spec`は`sd high-speed`、
   `signal voltage`は3.30 Vであり、`dmesg`も`mmc0: new high speed SDHC card`と記録している
   （UHS modeで初期化されていれば`ultra high speed`と表示される）。
   **規定の適用条件を満たさない測定で、cardが規定を満たさないとは言えない。**

   **ただし「hostがUHS-Iに対応していない」とまでは確定していない。**host controllerの
   capability registerを読めなかったため（`/sys/kernel/debug/mmc0/caps`はroot権限でも
   `EPERM`を返す）、**非対応なのかnegotiationが成立しなかっただけなのかを区別していない。**
   ③の判定には影響しない（どちらであってもUHS modeでないことに変わりはない）が、
   **「UHS-I対応readerを用意すれば解決する」とも断定しない。**
2. **一方で、host経路が6 MB/s台で頭打ちだとも言えない。**同じhost・同じcard・同じ
   file systemの読み出しが19.15 MB/sを記録している。搭載RAMは3.7 GiB
   （うちbuff/cache 1.2 GiB）に対し読んだのは29.80 GiBであり、
   **page cacheでは19.15 MB/sを説明できない。**
3. **書込みが遅い原因を、cardとhostとfile systemに分離できていない。**
   raw deviceへ直接書いても7.4 MB/sで10 MB/sに届かないため、
   **FAT32だけでは未達を説明できない**（上の「raw deviceへの直接書込み」）。
   **ただしこれは「FAT32が主因ではない」ことを示すものではない。**2つの測定は
   条件が複数異なり、寄与率を出せる比較になっていない。**cardの書込み特性、
   SDHCI controller（Ricoh `1180:E823`）の書込み経路、file systemの寄与のいずれも
   定量的に分離していない。**分離するには別のhost controllerで同じcardを測る必要がある。

**したがって「cardの劣化・不良」とも「cardは仕様どおり」とも結論しない。**

**raw device試験で分かったのは「file systemを外しても届かない」という一点である。**
card対hostの分離は、別のreaderを用意しない限りこの端末では進められない。

### この判定でPiのbootに使ってよいか

**使ってよい。**①②が示すのは、全域の読み書きが正しく、容量が公称どおりであることであり、
bootとstorageの用途で問題になるのはこの2点である。③はDeployの可否を左右しない
（書込み速度はimage書き込みに要する時間に影響するだけである）。

**ただし「速度を確認済み」とは書かない。**③は未達のままである。

### 実施しなかった項目

**skipを成功として記録しない。**

| 項目 | 状態 | 理由 |
|---|---|---|
| literalな全セクタの読み書き検査 | **実施しなかった** | `badblocks -w`はroot必須かつ完全破壊であり、①（容量詐称）を構造的に検出できない。①を優先して`f3`を選んだ。全セクタのliteralなカバーが要る場合は別途実施する |
| SMART相当の情報取得 | **手段が存在しない** | この個体の接続経路にSMART相当のcommandが無い（上記④）。実施可能だが行わなかった、ではない。**なおSD Express（NVMe interface）にはSMARTがあるが、この個体はSD Expressではない** |
| 耐久性（寿命）の評価 | **この記録の対象外** | 1回のhealth checkで書き込み寿命は判定できない。`HW-TBD-015`の`妨げる対象`のうち「耐久性」はこの記録では解決しない |
| 書込み速度の要因の定量的な分離 | **実施しなかった** | ③がinconclusiveである残りの理由。raw device試験で**FAT32だけでは未達を説明できない**ことは示したが、**card・host controller・file systemそれぞれの寄与率は出していない**（2つの測定は条件が複数異なる）。**分離には別のhost controllerで同じcardを測る必要がある** |
| UHS-I mode下での速度測定 | **実施しなかった** | 本端末の内蔵SDHCI readerが`sd high-speed`（3.30 V signaling）でしか動作せず、UHS-Iに入らない。**`U1`表示の妥当性を保証条件下で検証するには別のreaderが要る** |
| `manfid`＝`0x1b`がSamsungであることの確認 | **実施しなかった** | Physical Layer Simplified Specification Ver 9.10はMIDを「SD-3C, LLCが管理・定義・割り当てる」番号と規定するのみで、**登録簿を収録していない**（Section 5.1）。**この仕様書では検証できない。**別の一次資料が要る。**`SD-01`のメーカー表示の正は引き続き現物printである** |
| host controllerのUHS-I対応可否の確定 | **実施しなかった** | `/sys/kernel/debug/mmc0/caps`／`caps2`がroot権限でも`EPERM`を返すため、capability registerを読めなかった。**測定がUHS modeでないことは`ios`と`dmesg`で確定しているが、hostが非対応なのかnegotiationが成立しなかっただけなのかは区別していない** |

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
