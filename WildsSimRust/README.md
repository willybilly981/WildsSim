# WildsSim Rust port

EXXXI 氏の [WildsSim](https://github.com/EXXXI/WildsSim) (MIT License) の Rust 移植です。
機能・画面・データ・ロジックの設計はすべて原本によるもので、この移植はそれを写したものです。
利用するときは、まず原本を使い、原本のリポジトリに感謝を伝えてください。
ロジックは `simmodel`、GUI は `wildssim` に分かれています。

AI を使った実験として作ったもので、本家の代わりにするつもりはありません。
不具合や仕様の疑問を、原本の作者に問い合わせないでください。
原本がダークモードに対応していないので、AI に任せてみたところ、一通り動くところまで来ました。これが経緯です。
更新は作者と AI の気分次第で、issue や PR への対応は約束しません。
GUI は OS のテーマ設定 (ライト・ダーク) に従います。

## 構成

- `crates/simmodel`
  - マスタデータの読み込み、検索 (microlp)、保存ファイルの入出力
  - `data/` に本番データ、`testdata/` にテスト用の小さなデータ
- `crates/wildssim`
  - egui/eframe の GUI
  - `data.rs` が本番データをバイナリに埋め込み、初回起動時に展開する

## 実行

```powershell
cd WildsSimRust
cargo run -p wildssim --release
```

データフォルダは次の順で探します。

1. 環境変数 `WILDSSIM_DATA_DIR`
2. 実行ファイルと同じフォルダ (`MHWilds_SKILL.csv` があるか)
3. debug ビルド時のみ、ビルド元の `crates/simmodel/data`

カレントディレクトリは探しません。別のデータが置かれたフォルダから
起動すると、保存先が知らないうちに切り替わるためです。

どれも見つからない場合は、次の順で書き込める最初のフォルダに
埋め込みデータを展開します。

1. 実行ファイルのフォルダの `WildsSimData`
2. `%LOCALAPPDATA%\WildsSim`
3. 一時フォルダの `WildsSimData`

展開先の `MHWilds_*.csv` は、起動のたびに埋め込みデータと比べ、
違えば上書きします。exe を新しくすれば、マスタも新しい版に変わる仕組みです。
`conf/` と `save/` は、無いときだけ作ります。
`WILDSSIM_DATA_DIR` のフォルダと、実行ファイルと同じフォルダに置いた
マスタは書き換えません。

保存ファイルは一時ファイルに書いてから置き換えます。
保存中に異常終了しても、途中で切れたファイルは残りません。

## テスト

```powershell
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets
cargo run --release -p simmodel --example charm_bench -- 200
```

`charm_bench` は追加護石を n 件作ったデータで、下位互換護石の検出にかかる
起動時間を 1 スレッドと 4 スレッドで比較する。

このワークスペースを移動した直後は、`env!("CARGO_MANIFEST_DIR")` を
埋め込んだ古いテストバイナリが残り、`testdata` が見つからず失敗することがあります。
その場合は `cargo clean` してから再実行してください。

## 既知の制限

- ソルバが microlp のため、同点時の探索順が C# 版 (Google OR-Tools / SCIP) と異なります。
  固定装備での全件列挙や追加スキル検索は一致しますが、上位 N 件の内訳は
  一致しないことがあります。
- GUI の手動操作は自動テストしていません。
- 結果グリッドの列順保存 (`viewConfig.csv` の `グリッドの列順保存有無`) は未実装です。

## ライセンス

MIT License。原本の著作権表示を `LICENSE` に残しています。
