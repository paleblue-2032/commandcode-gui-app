# Command Code GUI

[Command Code](https://commandcode.ai) のデスクトップクライアントです。Rust と
[egui](https://github.com/emilk/egui)、
[Agent Client Protocol](https://agentclientprotocol.com/) (ACP) で実装しています。

アプリは `commandcode acp` を起動し、stdio 経由で ACP を話します。そのため既存の
CLI ログインと、ターミナルクライアントと同じセッションをそのまま利用できます。

## 機能

- markdown とシンタックスハイライト付きのストリーミングチャット
- ツールコールカード、実行ステータス、生の入出力、インライン diff
- `session/request_permission` に基づく権限ダイアログ
- `~/.commandcode/projects` のセッション履歴（プロジェクト別にグループ化）
- セッションバーからモデル・reasoning effort・モードを切り替え
- コンテキスト使用量インジケーター
- ACP が公開するスラッシュコマンド用のコマンドパレット
- ダーク / ライトテーマ、アクセントカラーとフォントスケールの設定
- 公式 Command Code ロゴとアプリアイコン
- レスポンシブレイアウト:
  - 620 px 未満でサイドバーを折りたたみ
  - 1080 px 未満でツール詳細をオーバーレイ表示
  - 最小ウィンドウサイズは 420×360 で、タイル型 WM でも 50/50 に分割できる

## 動作要件

- `PATH` に Command Code CLI があり、ログイン済みであること（`commandcode login`）
- ソースからビルドする場合は Rust 1.95 以上
- 同梱の flake により NixOS/Nix を直接サポート

## 実行

Nix を使う場合:

```bash
nix run
```

開発時:

```bash
nix develop -c cargo run
```

または、グラフィックスライブラリと Rust ツールチェーンが揃っていれば:

```bash
cargo run
```

デフォルトのエージェントコマンド:

```text
commandcode --no-auto-update acp
```

`--no-auto-update` は Nix ストアへの書き込みを避けるためのものです。バイナリや
引数は設定画面で変更し、その後「エージェントを再接続」を実行してください。

### flake からインストール

リポジトリを flake の input に追加します:

```nix
commandcode-gui-app = {
  url = "github:paleblue-2032/commandcode-gui-app";
  inputs.nixpkgs.follows = "nixpkgs";
};
```

home-manager にパッケージを追加します:

```nix
home.packages = [
  inputs.commandcode-gui-app.packages.${pkgs.system}.default
];
```

パッケージには公式アイコンとデスクトップエントリが含まれるため、アプリケーション
ランチャーにも表示されます。

## キーボード

| キー | 動作 |
| --- | --- |
| `Enter` | 送信 |
| `Shift+Enter` | 改行 |
| `Ctrl+N` | 新しいセッション |
| `Ctrl+K` | コマンドパレット |
| `Ctrl+,` | 設定 |
| `Esc` | 現在のターンを停止 / ダイアログを閉じる |

セッションの準備が完了すると、入力欄に自動でフォーカスします。

## 設定

設定は以下に保存されます:

```text
~/.config/commandcode-gui/settings.json
```

主な項目:

- `agent_binary` / `agent_args`
- `default_cwd`
- `theme`: `system`、`dark`、`light`
- `accent_color`: RGB 配列（例: `[232, 181, 102]`）
- `font_scale`
- `show_thoughts`
- `permission_policy`: `ask`、`auto_allow_reads`、`yolo`

CJK フォントは `COMMANDCODE_GUI_CJK_FONT` が設定されていればそこから、なければ
`fc-match "Noto Sans CJK JP"` から読み込みます。Nix パッケージと devShell では
これが自動的に設定されます。

## 開発

```bash
nix develop -c cargo fmt --check
nix develop -c cargo clippy --all-targets -- -D warnings
nix develop -c cargo test
```

ACP のエンドツーエンド・スモークテスト（実際にセッションを作成し、応答を要求します）:

```bash
nix develop -c cargo run --example acp_smoke
```
