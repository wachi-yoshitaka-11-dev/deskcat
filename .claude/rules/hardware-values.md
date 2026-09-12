---
paths:
  - "docs/hardware/**"
  - "docs/protocol/**"
  - "hardware/**"
  - "tests/hil/**"
---

# ハードウェアの値と安全に触れるとき

**推測禁止とハードウェア安全の規則は `AGENTS.md` が常時持っている。ここへ写さない。**
この rule が足すのは、この path でだけ要る次の3点である。

1. **[Hardware Safety Policy](../../docs/governance/hardware-safety-policy.md)を開く。**
   安全要件の5項目、要求する根拠の水準（判定に効く数か、資格として求める数か）、
   一次資料の定義は、**すべて同 policy が正本である。**`AGENTS.md` の一覧だけで判断しない。
2. **[TBD台帳](../../docs/hardware/tbd-register.md)で対象項目の状態と Owner を確認する。**
   **gate を開くのは人間の承認である。**AI の判断で開かない。
3. **一般値で開始する場合も、採った値と、それが暫定であることと、確定させる手段を記録する。**
