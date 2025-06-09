// src/model.rs
use serde::Serialize;
use std::path::PathBuf;

/// 解析した各ルートの情報を保持する構造体
#[derive(Debug, Serialize)]
pub struct RouteInfo {
    /// ルートのパス (例: "home", "feature", ""(空文字) など)
    pub path: Option<String>,

    /// loadChildren で指定された文字列をそのまま格納
    /// 例: "() => import(\"./feature/feature.module\").then(m => m.FeatureModule)"
    pub load_children: Option<String>,

    /// このルート定義が書かれているソースファイルへの絶対パス
    pub source_file: PathBuf,

    /// 子ルート (children) があれば再帰的に格納
    pub children: Vec<RouteInfo>,
}
