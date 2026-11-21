# WildsSim Rust port

EXXXI 氏の [WildsSim](https://github.com/EXXXI/WildsSim) (MIT License) を Rust に移植したものです。
機能・画面・データ・ロジックの設計はすべて原本に従っています。

AI の実験として作ったもので、本家の代わりになるものではありません。
原本を使える人は原本を使ってください。バグや仕様の疑問を原本の作者に問い合わせないでください。
こちら側の不具合報告は、このフォークの issue にどうぞ。

## 本家との違い

- 単一の exe だけで動きます (.NET 不要)
- GUI は OS のテーマ設定 (ライト・ダーク) に従います
- 起動時に必要なデータを埋め込みから展開するため、zip 解凍なしでも動きます

## 構成

- `crates/simmodel`
  - マスタデータの読み込み、検索 (microlp)、保存ファイルの入出力
  - `data/` に本番データ、`testdata/` にテスト用の小さなデータ
- `crates/wildssim`
  - egui/eframe の GUI
  - `data.rs` が本番データをバイナリに埋め込み、初回起動時に展開します

## 実行

```powershell
cd WildsSimRust
cargo run -p wildssim --release
```

リリースの zip からは `wildssim.exe` 1 つで動きます。

## データフォルダ

次の順でデータフォルダを探します。

1. 環境変数 `WILDSSIM_DATA_DIR`
2. 実行ファイルと同じフォルダ (`MHWilds_SKILL.csv` があるか)
3. debug ビルド時のみ、ビルド元の `crates/simmodel/data`

カレントディレクトリは探しません。別のデータが置かれたフォルダから
起動すると、保存先が知らないうちに切り替わるためです。

どれも見つからない場合は、書き込める最初のフォルダに埋め込みデータを展開します。

1. 実行ファイルのフォルダの `WildsSimData`
2. `%LOCALAPPDATA%\WildsSim`
3. 一時フォルダの `WildsSimData`

展開先の `MHWilds_*.csv` は起動のたびに埋め込みデータと比べ、違えば上書きします。
exe を差し替えればマスタも新しい版に追従します。
`conf/` と `save/` は無いときだけ作ります。
`WILDSSIM_DATA_DIR` のフォルダと、実行ファイルと同じフォルダのマスタは書き換えません。

展開先の `save/` が既にあるのに書き込みに失敗した場合は、
別フォルダへ逃げずにエラーで止まります。%TEMP% に逃げると
保存データが消えて見えるためです。

保存ファイルは一時ファイルに書いてから置き換えます。
保存中に異常終了しても、途中で切れたファイルは残りません。

## テスト

```powershell
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets
cargo run --release -p simmodel --example charm_bench -- 200
```

`charm_bench` は追加護石を n 件作ったデータで、下位互換護石の検出にかかる
起動時間を 1 スレッドと 4 スレッドで比較します。

このワークスペースを移動した直後は、`env!("CARGO_MANIFEST_DIR")` を埋め込んだ
古いテストバイナリが残り、`testdata` が見つからず失敗することがあります。
その場合は `cargo clean` してから再実行してください。

## 既知の制限

- ソルバが microlp のため、同点時の探索順が C# 版 (Google OR-Tools / SCIP) と異なります。
  固定装備での全件列挙は一致しますが、上位 N 件の内訳は一致しないことがあります。
- 追加スキル検索は、発動しないグループ・シリーズスキルの中間レベルを
  結果に含めません。C# 版は求解してから UI で隠していたため、件数は原本より少なくなります。
  表示される結果は同じです。
- GUI の手動操作は自動テストしていません。
- 結果グリッドの列順保存 (`viewConfig.csv` の `グリッドの列順保存有無`) は未実装です。

## ライセンス

MIT License。原本の著作権表示を `LICENSE` に残しています。
