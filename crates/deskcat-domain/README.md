# deskcat-domain

接触eventから表情と首の動きの意図を決め、時間でneutralへ戻す純粋domain library（[#491](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/491)）。

serial、file、network、GPIO、GUIへ依存させない。時計も読まず、時刻は呼び出し側から受け取る。

遷移と範囲は`src/lib.rs`のcrate doc、値は`src/mind.rs`の`Timing::MVP`を正本とする。ここへ再掲しない。

[#23](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/23)のローカル独り言候補と頻度制限は`src/local_speech.rs`に置く。頻度の運用値は`TBD`であり、呼び出し側が0でない最短間隔と単調時刻を渡す。`show_text`への変換と実LCDでの表示確認はこのcrateの範囲外である。
