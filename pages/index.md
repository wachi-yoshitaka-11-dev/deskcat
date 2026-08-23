---
layout: home
title: DeskCat 公開ドキュメント
---

<figure class="dc-figure">
  <img src="assets/deskcat-concept.jpg" width="720" height="720"
       alt="DeskCatのコンセプトイメージ。机上に座る白い猫型ロボットが、顔の前面displayに目と口を表示している。">
  <figcaption>コンセプトイメージ（完成品の外観、部品構成、動作を示すものではありません）</figcaption>
</figure>

DeskCatは、ESP32とRaspberry Pi Zero Wで構成する、机上で静かに振る舞う猫型ペットロボットです。
{: .dc-lead}

> 正確なmodule、GPIO、電源、サーボ制限が確定するまで、未確認値を使った実機駆動は行いません。
{: .dc-notice}

開発の進行状況は[GitHub Issues](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues)で管理しており、このsiteでは複製しません。
{: .dc-note}

## はじめに

- [Project概要（repository README）](https://github.com/wachi-yoshitaka-11-dev/deskcat#readme)
- [Architecture（予定文書の一覧）](docs/architecture/README.md)
- [開発基盤計画](docs/planning/development-foundation-plan.md)
- [GitHub repository](https://github.com/wachi-yoshitaka-11-dev/deskcat)
- [GitHub Issues](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues)
{: .dc-cards}

## 開発方針と安全
{: .dc-tone-2}

- [Governance](docs/governance/README.md)
- [AI Agent Policy](docs/governance/ai-agent-policy.md)
- [Development Workflow](docs/governance/development-workflow.md)
- [Hardware Safety Policy](docs/governance/hardware-safety-policy.md)
- [Architecture Decision Records](docs/decisions/README.md)
{: .dc-cards .dc-tone-2}

## ハードウェアと通信
{: .dc-tone-3}

- [Hardware正本文書](docs/hardware/README.md)
- [Hardware TBD一覧](docs/hardware/tbd-register.md)
- [ESP32–Pi protocol](docs/protocol/README.md)
- [マイコン開発技術ガイド](docs/DeskCat_Microcontroller_Development_Guide.md)
{: .dc-cards .dc-tone-3}

## 開発環境と手順
{: .dc-tone-4}

- [Toolchainと端末profile](docs/toolchains/README.md)
- [Runbook](docs/runbooks/README.md)
- [Contribution方法](https://github.com/wachi-yoshitaka-11-dev/deskcat/blob/main/CONTRIBUTING.md)
- [Security](SECURITY.md)
{: .dc-cards .dc-tone-4}
