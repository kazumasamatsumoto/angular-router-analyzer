# Angular Router Analyzer

Angular プロジェクトのルーティング設計を再帰的に解析して JSON 出力する CLI ツールです。

## 概要

Angular Router Analyzer は、大規模な Angular プロジェクトのルーティング構造を自動解析し、以下の情報を抽出します：

- 📍 **ルートパス**: 各ルートの定義されたパス
- 🔗 **Lazy Loading**: `loadChildren` による遅延読み込み設定
- 📁 **ソースファイル**: ルート定義が記述されているファイル
- 🌳 **階層構造**: 親子関係を含むルート構造

## 特徴

- ✅ **TypeScript/JavaScript 対応**: SWC パーサーを使用した高速解析
- ✅ **変数参照解決**: `const routes = [...]` + `RouterModule.forRoot(routes)` パターンに対応
- ✅ **再帰的解析**: `loadChildren` で指定されたモジュールも自動で追跡
- ✅ **マルチファイル対応**: プロジェクト全体のルーティングファイルを一括解析
- ✅ **JSON 出力**: 解析結果を構造化された JSON で出力

## インストール

### ローカルインストール

```bash
# プロジェクトルートで実行
cargo install --path .
```

### 依存関係

- Rust 1.70.0 以上
- Cargo

## 使用方法

### 基本的な使用方法

```bash
angular-router-analyzer --project-root /path/to/your/angular/project
```

### 例

```bash
# Windows の場合
angular-router-analyzer --project-root C:\projects\my-angular-app

# Linux/macOS の場合
angular-router-analyzer --project-root ~/projects/my-angular-app

# 相対パスも使用可能
angular-router-analyzer --project-root ../my-angular-app
```

### オプション

```bash
angular-router-analyzer [OPTIONS] --project-root <DIR>

Options:
  -r, --project-root <DIR>  解析対象の Angular プロジェクトルート
  -h, --help               ヘルプ情報を表示
  -V, --version            バージョン情報を表示
```

## 出力形式

解析結果は JSON 形式で標準出力に出力されます：

```json
[
  {
    "path": "home",
    "load_children": null,
    "source_file": "/project/src/app/app-routing.module.ts",
    "children": []
  },
  {
    "path": "feature",
    "load_children": "import('./feature/feature.module').then(m => m.FeatureModule)",
    "source_file": "/project/src/app/app-routing.module.ts",
    "children": [
      {
        "path": "sub-feature",
        "load_children": null,
        "source_file": "/project/src/app/feature/feature-routing.module.ts",
        "children": []
      }
    ]
  }
]
```

### フィールド説明

| フィールド | 型 | 説明 |
|-----------|----|----|
| `path` | `string \| null` | ルートのパス（例: "home", "feature/:id"） |
| `load_children` | `string \| null` | 遅延読み込みの設定（import文の文字列表現） |
| `source_file` | `string` | このルート定義が記述されているファイルの絶対パス |
| `children` | `array` | 子ルートの配列（再帰構造） |

## 対応ファイルパターン

ツールは以下のパターンのファイルを自動検出します：

- `**/*-routing.module.ts`
- `**/*-routes.ts`
- `**/app-routing.module.ts`
- `**/app-routing.ts`
- `**/routes.ts`

## 対応ルーティングパターン

### 1. 基本的なルート定義

```typescript
const routes: Routes = [
  { path: 'home', component: HomeComponent },
  { path: 'about', component: AboutComponent }
];

@NgModule({
  imports: [RouterModule.forRoot(routes)],
  exports: [RouterModule]
})
export class AppRoutingModule { }
```

### 2. Lazy Loading

```typescript
const routes: Routes = [
  {
    path: 'feature',
    loadChildren: () => import('./feature/feature.module').then(m => m.FeatureModule)
  }
];
```

### 3. 子ルート

```typescript
const routes: Routes = [
  {
    path: 'parent',
    component: ParentComponent,
    children: [
      { path: 'child1', component: Child1Component },
      { path: 'child2', component: Child2Component }
    ]
  }
];
```

## 実用例

### 1. ルート構造の可視化

```bash
# 解析結果をファイルに保存
angular-router-analyzer --project-root ./my-app > routes.json

# jq を使用して整形
angular-router-analyzer --project-root ./my-app | jq '.'
```

### 2. 特定の情報の抽出

```bash
# loadChildren を使用しているルートのみ抽出
angular-router-analyzer --project-root ./my-app | jq '.[] | select(.load_children != null)'

# パス一覧を取得
angular-router-analyzer --project-root ./my-app | jq -r '.[].path // "root"'
```

### 3. ルート数の統計

```bash
# 総ルート数
angular-router-analyzer --project-root ./my-app | jq 'length'

# ファイル別ルート数
angular-router-analyzer --project-root ./my-app | jq 'group_by(.source_file) | map({file: .[0].source_file, count: length})'
```

## トラブルシューティング

### よくある問題

1. **ファイルが見つからない**
   ```
   Error: メインのルーティングファイルが見つかりませんでした。
   ```
   - プロジェクトルートの指定が正しいか確認
   - ルーティングファイルが期待されるパターンで命名されているか確認

2. **パース エラー**
   ```
   Parsing error in "file.ts": SyntaxError
   ```
   - TypeScript の構文エラーがないか確認
   - サポートされていない構文を使用していないか確認

3. **空の結果**
   ```
   []
   ```
   - `RouterModule.forRoot()` または `RouterModule.forChild()` を使用しているか確認
   - ルート定義が配列リテラルまたは変数で定義されているか確認

### デバッグ

詳細なデバッグ情報が必要な場合は、ソースコードの `println!` 文のコメントアウトを外してビルドしてください。

## 制限事項

- 動的に生成されるルート定義は解析できません
- 複雑な条件分岐を含むルート定義は部分的にしか解析されない場合があります
- ES6 import/export の静的解析のみ対応（dynamic import は文字列として保存）

## 開発

### ビルド

```bash
cargo build --release
```

### テスト

```bash
cargo test
```

### 開発版の実行

```bash
cargo run -- --project-root /path/to/test/project
```

## ライセンス

[ライセンス情報をここに記載]

## 貢献

Issue や Pull Request をお待ちしています。

## 更新履歴

### v0.1.0
- 初期リリース
- 基本的なルート解析機能
- Lazy Loading 対応
- JSON 出力機能