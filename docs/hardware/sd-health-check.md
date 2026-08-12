# microSD（SD-01）health check記録

> 状態: 実施済み
> 判定: `Partial`
> 実施日: 2026-08-12（registerの読み方の一次資料照合は2026-08-13）
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

**④の性質。**SMARTはATA／NVMeの機能であり、SD cardには相当する標準interfaceが無い。
eMMCの`EXT_CSD`が持つhealth statusはeMMC固有であってSD cardには適用されない。
したがって④は**「実施しなかった」のではなく「取得手段が存在しない」**である。
この区別を結果欄に書く。

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
「解釈」はregisterのbit定義に基づく読み方であり、**一次資料と照合していない**（後述）。

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
④はSD cardにSMART相当の標準interfaceが無いことから判定している。

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

**hostはUHS-Iで駆動していない。**`timing spec`は`sd high-speed`であり、
`signal voltage`は3.30 Vである。UHS-Iは1.8 V signalingを必須とするため、
**このhostではcardがUHS-I mode（`U1`の保証が成立する条件）で動作していない。**
これは判定基準③の但し書きが想定した条件そのものである。

**`clock`値からbus帯域の上限を導かない。**この記録の作成時に一度
「33 MHz × 4 bit ＝ 約16.5 MB/s が上限」と書いたが、**後述の読み出し実測19.15 MB/sが
この値を超えており、導出が誤っていた。**`ios`が示す`clock`は取得時点の値であって、
転送中の実効clockと同一である保証がない。**したがってこの記録では、
bus帯域の上限を計算で示さず、実測値だけを根拠に用いる。**

### f3の出力

`docs/`配下にMarkdown以外の証拠fileを置かない規則のため、出力全文をここへ埋め込む。
`f3write`の`Creating file N.h2w ... OK!`は30行あり、**2〜29番は同一形式のため省略した**
（省略した旨をここに明記する）。それ以外は未加工である。

`f3write /media/<user>/F3TEST/`:

```text
F3 write 8.0
Copyright (C) 2010 Digirati Internet LTDA.
This is free software; see the source for copying conditions.

Free space: 29.80 GB
Creating file 1.h2w ... OK!
（2.h2w〜29.h2w も同じく OK!。省略）
Creating file 30.h2w ... OK!
Free space: 0.00 Byte
Average writing speed: 6.11 MB/s
```

`f3read /media/<user>/F3TEST/`（**こちらは全行を掲載する。判定基準①②の根拠であるため**）:

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

### 事後処理

- 書いた`*.h2w` 30 fileを削除し、cardを空の状態へ戻した（削除後の使用量16 KiB）
- partitionをunmountした
- **cardのpartition構成は検査時のまま（単一FAT32 partition、label `F3TEST`）である。**
  検査前に入っていたRockchip向けUbuntu 18.04 imageは、全域を検査対象にするため消去した
  （消去前に内容を提示し、humanが「消えてもよい」と判断した）。
  **Piで使うimageの書き込みは本記録の範囲外である**
- **一時導入した`f3`をhumanが撤去した**（`apt remove --purge f3` ＋ `apt autoremove`）。
  撤去後に`command -v f3write`が空を返し、`dpkg -l f3`にpackage登録が無いことを確認した
  （[AGENTS.md](../../AGENTS.md)「出所不明のskill、plugin、rule、スクリプトを自動導入しない」
  および作業指示「一時導入したtoolは作業後に撤去する」）

## 判定

**`Partial`。**①②⑤は基準を満たしたが、③が未達で原因を切り分けられず、④は取得手段が無い。

| # | 観点 | 基準 | 実測 | 判定 |
|---|---|---|---|---|
| ① | 容量詐称 | `Data LOST`＝0 sector | 0 sector（3区分とも0） | **合格** |
| ② | 読み書きの通し | 書込み総量と`Data OK`が一致 | 30 file・29.80 GiB・62,488,704 sectorが一致 | **合格** |
| ③ | 速度 ≥ 10 MB/s | `f3write`平均書込み速度 | **6.11 MB/s** | **未達（inconclusive）** |
| ④ | SMART相当 | 取得手段の有無を確認 | **手段が存在しない** | 確認済み |
| ⑤ | 識別情報 | 現物printと矛盾しない | `32`／`microSDHC`／`U1`はregisterと一致（**読み方を仕様書Ver 9.10と照合済み**）。**`Samsung`だけは未裏付け**（MIDの登録簿が仕様書に無い） | **合格（1項目のみ未裏付け）** |

**偽造品ではない。**29.80 GiB全域へ位置依存dataを書いて読み戻し、
`Corrupted`／`Slightly changed`／`Overwritten`のいずれも0 sectorであった。
容量が公称どおりであること、および検査範囲に読み書き不良が無いことを確認した。

### ③をinconclusiveとする理由

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
3. **書込みが遅い原因を、cardとhostとfile systemに分離していない。**FAT32のmetadata更新、
   SDHCI controller（Ricoh `1180:E823`）の書込み経路、cardのnon-UHS時の書込み特性が
   分離できていない。**分離するにはUHS-I対応readerでの再測定か、raw deviceへの
   直接書込みによるfile system分の切り分けが要る。いずれも本記録では実施していない。**

**したがって「cardの劣化・不良」とも「cardは仕様どおり」とも結論しない。**

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
| SMART相当の情報取得 | **手段が存在しない** | SD cardにSMART相当の標準interfaceが無い（上記④）。実施可能だが行わなかった、ではない |
| 耐久性（寿命）の評価 | **この記録の対象外** | 1回のhealth checkで書き込み寿命は判定できない。`HW-TBD-015`の`妨げる対象`のうち「耐久性」はこの記録では解決しない |
| 書込み速度が遅い原因の分離 | **実施しなかった** | ③がinconclusiveである直接の原因。card・host controller・FAT32のどれが効いているかを分けるには、**UHS-I対応readerでの再測定**か、**raw deviceへの直接書込みによるfile system分の切り分け**が要る。前者は該当readerを所持していない、後者はroot権限を要し本作業の範囲外とした |
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
