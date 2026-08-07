# ADR-0005: 開発環境の標準OSを実機Linuxとする

> 状態: Accepted
> 日付: 2026-08-06

## 背景

DeskCatの開発端末について、[ADR-0002](0002-role-based-development-environments.md)と
[Machine Profiles](../toolchains/machine-profiles.md)は端末の**役割**を定義している。しかし、
その役割をどのOSで担うかは、どの文書にも規定がなかった。

[#5](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/5)でLinux x86_64からclean buildを取得した際に、
この欠落が判明した。[ESP32開発端末Setup](../runbooks/esp32-development-machine-setup.md)には
2026-07-27時点でWindowsとLinuxの両方の事前要件が候補として並んでおり、Windows側は
誰も実行していないまま残っていた。OS方針が未記録のままでは、次の問題が残る。

- どのOSで得た結果を検証根拠として扱えるのかが判断できない
- runbookの未実行手順と、実際に使うOSの手順を読み分けられない
- CIと開発端末のOS差から生じる不一致を、環境差なのか実際の不具合なのか切り分けられない

## 判断要因

- CIと開発端末のOSを揃えて、環境差による不一致を減らす
- 対象system `deskcatd` の動作環境と開発環境を近づける
- flashと実機試験に必要なUSBアクセスを確保する
- buildからflashまでを一台で完結させ、根拠の移送を減らす
- USBを必要としない作業では、再現可能で使い捨てできる環境を使う
- support対象のOSを増やしすぎず、runbookの検証負担を抑える

## 検討した選択肢

### Windowsを標準とする

追加の仮想化なしにGUIと開発を同居させられる。しかしCIが`ubuntu-24.04`であるため、
path、symlink、大文字小文字、改行の扱いの差がそのまま結果差になる。
実例として`scripts/validate-doc-links.ps1`は、同一repositoryの走査対象が
Windows（`core.symlinks=false`）で229件、Linux CIで243件と食い違った事実を記録しており、
この差を吸収するためにdigest比較を追加している。
また対象の`deskcatd`はRaspberry Pi OS（Linux）上で動作するため、host側のuserspaceも乖離する。

### Windows上の仮想環境（VM、Docker）で開発する

CIとOSを揃えられるが、flashが成立しない。
Hyper-V上のUbuntu VMでは`/sys/bus/usb/devices/`が空でUSB deviceが見えず、
Docker Desktop on WindowsもUSB passthroughを提供しない。
結果としてESP32 Flash / HILを別端末へ分離せざるを得ず、
buildとflashの根拠が常に二台にまたがる。

### 実機Linuxを標準とし、USB不要な作業ではDocker上のLinuxも使う

CIとOSが揃い、USBが直接見えるため一台でbuildとflashを完結できる。
文書検証やbuild-onlyのように実機を触らない作業は、containerで隔離して再現性を上げられる。
Linux環境の初期構築が必要になるが、対象範囲が明確で、runbookの検証対象も一つに絞れる。

## 決定

開発環境の標準OSを**実機のLinux**とする。USBを必要としない作業では、
Docker上のLinuxも使用する。

- Host Rust Development、ESP32 Build、ESP32 Flash / HIL、Docs / Reviewの各profileは、
  実機Linuxを前提とする。
- **ESP32 Flash / HILと実機試験は実機Linuxで実行する。** VMまたはcontainerで代替しない。
- USBを必要としない作業では、Docker上のLinuxを使ってよい。文書検証、host Rustのbuildとtest、
  ESP32のbuild-only検証が該当する。
- Docker、VM、実機のいずれで実行したかは、
  [Version Record](../toolchains/version-record-template.md)の既存field
  `Container / VM / native:`へ記録する。新しいfieldは追加しない。
- Windowsはsupport対象外とする。動作する可能性は否定しないが、
  DeskCatの検証根拠として扱わず、runbookの検証対象にも含めない。
- distributionとCPU architectureは本ADRで固定しない。CIは`ubuntu-24.04`のx86_64であり、
  [#5](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/5)のclean buildもx86_64で取得した。
  開発端末のdistribution、version、architectureは、実際に検証した端末のversion recordで記録する。
- macOSについては本ADRで判断しない。標準はLinuxであるため、macOSを標準として扱わないが、
  対象外と宣言するかどうかは必要が生じた時点で別途判断する。
- Raspberry Pi Runtime / Direct Build profileのOSは本ADRの対象外であり、
  引き続きRaspberry Pi OSとする。

## 影響

### 利点

- 開発端末とCIのOSが揃い、失敗が環境差か実際の不具合かを切り分けやすくなる。
- 一台でbuildからflashまで実行でき、根拠を二台にまたがらせずに済む。
- runbookが検証すべきOSが一つに絞られ、未実行の候補手順を残さずに済む。
- 対象の`deskcatd`が動作するLinuxと、開発時のuserspaceが近くなる。

### 欠点

- Windows端末では、DeskCatの検証根拠を作れない。
- 実機Linux環境の初期構築と保守が必要になる。
- GUIやOffice作業を同じ端末で行う場合、OSを分ける運用判断が必要になる。

### リスクと対策

| リスク | 対策 |
|---|---|
| Windows上のVMやDockerでflashを試みて失敗する | Flash / HILを実機Linux限定と明記し、Docker可否をprofileごとに示す |
| Dockerで得た結果を実機の根拠として扱う | version recordの`Container / VM / native:`を必須記録とし、profileごとに結果を区別する |
| CIのdistributionと開発端末のdistributionが乖離する | distributionを固定せず、version recordへ実測値を残し、差分は`Known differences from documented profile`へ記録する |
| Windows手順が未実行のまま文書に残り、実行可能と誤読される | runbookのWindows節を対象外と明記し、具体手順を残さない |
| 標準OS変更に伴い過去の検証根拠が無効化される | OSとarchitectureをversion recordの再確認項目として維持する |

## 検証

この決定は、次を満たすことで検証する。

- ESP32 Build profileのclean buildを、実機Linuxの手順から再現できる。
- `scripts/validate-doc-links.ps1`の`DIGEST`が、local Linuxと`ubuntu-24.04`のCIで一致する。
- ESP32のbuildとflashを、同一の実機Linux端末で連続して実行できる。
- version recordの`Container / VM / native:`が、提出されたすべての記録で埋まっている。
- runbookに、対象外と明記されていないWindows専用手順が残っていない。

## 置き換える決定

なし。[ADR-0002](0002-role-based-development-environments.md)の役割別開発環境は有効なままであり、
本ADRはその役割に対する標準OSを追加する。
