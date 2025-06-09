use path_absolutize::Absolutize;
use std::fs;
use std::path::{Path, PathBuf};

/// `load_children_str` に含まれる `import("...")` の文字列部分を取り出し、
/// 親ファイル (`parent_file`) のディレクトリを基準にして、
/// 実際の routing ファイル (.ts) を探しに行く関数。
///
/// - `load_children_str`: parser.rs で文字列化しておいた loadChildren の式 (例: "() => import(\"./feature/feature.module\").then(m => m.FeatureModule)")
/// - `parent_file`: その式を持つ親ファイルのパス (例: `/proj/src/app/app-routing.module.ts`)
/// - `_project_root`: プロジェクトルート (例: `/proj`) - 修正: 未使用変数のため _ プレフィックスを追加
///
/// 戻り値:
/// - Ok(Some(path)) → 見つかった routing ファイルの絶対パス (PathBuf)
/// - Ok(None)       → 見つからなかった (子ルートなしとみなす)
pub fn resolve_load_children_path(
    load_children_str: &str,
    parent_file: &Path,
    _project_root: &Path,  // 修正: 未使用変数の警告を解消
) -> Result<Option<PathBuf>, Box<dyn std::error::Error>> {
    // 1) 文字列中の `import("...")` の部分から、ダブルクォート内のパスを抽出 (簡易実装)
    let import_path: Option<String> = {
        if let Some(start) = load_children_str.find("import(\"") {
            let sub = &load_children_str[start + "import(\"".len()..];
            if let Some(end_quote) = sub.find("\"") {
                Some(sub[..end_quote].to_string())
            } else {
                None
            }
        } else {
            None
        }
    };

    let import_path = match import_path {
        Some(p) => p,
        None => return Ok(None),
    };

    // 2) parent_file の親ディレクトリを基準にして相対パスを結合し、candidate_base を作成
    //    例: parent_file = /proj/src/app/app-routing.module.ts
    //        import_path = "./feature/feature.module"
    //    → candidate_base = /proj/src/app/feature/feature.module
    let parent_dir = parent_file
        .parent()
        .ok_or("親ファイルのディレクトリが取得できません")?;
    let candidate_base = parent_dir.join(import_path);

    // 3) 典型的なファイル名パターンをいくつか列挙
    let mut candidates: Vec<PathBuf> = Vec::new();

    // A) feature.module.ts を直接探す
    candidates.push(candidate_base.with_extension("ts"));

    // B) feature-routing.module.ts が隣にあるか探す
    if let Some(stem) = candidate_base.file_stem() {
        let routing_name = format!("{}-routing.module.ts", stem.to_string_lossy());
        let routing_path = candidate_base.with_file_name(routing_name);
        candidates.push(routing_path);
    }

    // C) candidate_base がディレクトリだったら、その中の <stem>-routing.module.ts を探す
    if candidate_base.is_dir() {
        if let Some(stem) = candidate_base.file_name() {
            let file_in_dir =
                candidate_base.join(format!("{}-routing.module.ts", stem.to_string_lossy()));
            candidates.push(file_in_dir);
        }
    }

    // 4) 列挙した候補を絶対パス化し、最初に存在するものを返却
    for cand in candidates {
        let abs = cand.absolutize()?.to_path_buf();
        if fs::metadata(&abs).is_ok() {
            return Ok(Some(abs));
        }
    }

    // 5) どれにも該当しなければ None
    Ok(None)
}