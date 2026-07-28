# Hardware-in-the-Loop test

このディレクトリには、実際のDeskCatハードウェアを動作させるhost側fixtureと手順を置く。

HIL testでは次を記録する。

- 必要なハードウェアと配線revision
- 電源と安全の前提条件
- firmwareとhostのversion
- 正確な手順
- 期待する機械可読の証拠
- cleanupと緊急停止動作

HILは通常のCIから分離し、明示的に準備したtest benchを必要とする。
