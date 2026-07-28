---
layout: default
title: DeskCat 公開ドキュメント
---

# DeskCat

DeskCatは、ESP32とRaspberry Pi Zero WHで構成する、机上で静かに振る舞う猫型ペットロボットです。

現在は開発基盤の整備とハードウェア特定を進めています。正確なmodule、GPIO、電源、サーボ制限が確定するまで、未確認値を使った実機駆動は行いません。

## はじめに

- [Project概要（repository README）](https://github.com/wachi-yoshitaka-11-dev/deskcat#readme)
- [Architecture](docs/architecture/README.md)
- [開発基盤計画](docs/planning/development-foundation-plan.md)
- [GitHub repository](https://github.com/wachi-yoshitaka-11-dev/deskcat)
- [GitHub Issues](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues)

## 開発方針と安全

- [Governance](docs/governance/README.md)
- [AI Agent Policy](docs/governance/ai-agent-policy.md)
- [Development Workflow](docs/governance/development-workflow.md)
- [Hardware Safety Policy](docs/governance/hardware-safety-policy.md)
- [Architecture Decision Records](docs/decisions/README.md)

## ハードウェアと通信

- [Hardware正本文書](docs/hardware/README.md)
- [Hardware TBD一覧](docs/hardware/tbd-register.md)
- [ESP32–Pi protocol](docs/protocol/README.md)
- [マイコン開発技術ガイド](docs/DeskCat_Microcontroller_Development_Guide.md)

## 開発環境と手順

- [Toolchainと端末profile](docs/toolchains/README.md)
- [Runbook](docs/runbooks/README.md)
- [Contribution方法](https://github.com/wachi-yoshitaka-11-dev/deskcat/blob/main/CONTRIBUTING.md)
- [Security](SECURITY.md)

## 文書の正本

このsiteは、[ADR-0003](docs/decisions/0003-public-documentation-publishing.md)に従ってrepository内のMarkdownから生成されます。技術情報の正本はrepositoryであり、PagesやWiki上で独立した仕様を管理しません。
