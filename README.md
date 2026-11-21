# WildsSimRust

EXXXI 氏の [WildsSim](https://github.com/EXXXI/WildsSim) (MIT License) を Rust に移植したものです。
機能・画面・データ・ロジックの設計はすべて原本に従っています。

## 構成

- `crates/simmodel`
  - マスタデータの読み込み、検索 (microlp)、保存ファイルの入出力
  - `data/` に本番データ、`testdata/` にテスト用の小さなデータ
- `crates/wildssim`
  - egui/eframe の GUI
  - `data.rs` が本番データをバイナリに埋め込み、初回起動時に展開します

## 実行

```powershell
cargo run -p wildssim --release
```

## データフォルダ

次の順でデータフォルダを探します。

1. 環境変数 `WILDSSIM_DATA_DIR`
2. 実行ファイルと同じフォルダ (`MHWilds_SKILL.csv` があるか)
3. debug ビルド時のみ、ビルド元の `crates/simmodel/data`

どれも見つからない場合は、書き込める最初のフォルダに埋め込みデータを展開します。

1. 実行ファイルのフォルダの `WildsSimData`
2. `%LOCALAPPDATA%\WildsSim`
3. 一時フォルダの `WildsSimData`

展開先の `save/` が既にあるのに書き込みに失敗した場合は、
別フォルダへ逃げずにエラーで止まります。

保存ファイルは一時ファイルに書いてから置き換えます。

## テスト

```powershell
cargo test --workspace --all-targets
```

## 既知の制限

- ソルバが microlp のため、同点時の探索順が C# 版 (Google OR-Tools / SCIP) と異なります。
  固定装備での全件列挙は一致しますが、上位 N 件の内訳は一致しないことがあります。
- 追加スキル検索は、発動しないグループ・シリーズスキルの中間レベルを
  結果に含めません。C# 版は求解してから UI で隠していたため、件数は原本より少なくなります。
  表示される結果は同じです。
- 結果グリッドの列順保存 (`viewConfig.csv` の `グリッドの列順保存有無`) は未実装です。

## ライセンス

MIT License。原本の著作権表示を `LICENSE` に残しています。

## 謝辞

原本の [WildsSim](https://github.com/EXXXI/WildsSim) に感謝します。
原本の README に記載されているスペシャルサンクスの皆様にも感謝します。

## お問い合わせ

issue や PR への対応はしません。改造したい方はご自身でどうぞ。
