// src/main.rs

use clap::Parser;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

mod parser;
mod resolver;
mod model;

use parser::parse_routes_in_file;
use resolver::resolve_load_children_path;
use model::RouteInfo;

/// CLI 引数定義
#[derive(Parser, Debug)]
#[command(
    name = "Angular Router Analyzer",
    version = "0.1.0",
    author = "あなたの名前 <your.email@example.com>",
    about = "Angular プロジェクトのルーティング設計を再帰的に解析して JSON 出力する CLI ツール"
)]
struct Cli {
    /// 解析対象の Angular プロジェクトルート
    /// 例: `--project-root C:/path/to/my-angular-project`
    #[arg(short = 'r', long = "project-root", value_name = "DIR")]
    project_root: PathBuf,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1) CLI 引数をパースし、プロジェクトルートを絶対パス化
    let cli = Cli::parse();
    let project_dir = cli.project_root.canonicalize()?; // 絶対化

    // 2) WalkDir で全ファイルを再帰的に探索し、
    //    ファイル名に「routing」または「routes」が含まれる .ts ファイルをメインルーティング候補として集める
    let mut main_routing_paths: Vec<PathBuf> = Vec::new();
    for entry in WalkDir::new(&project_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_type().is_file() && e.path().extension().map_or(false, |ext| ext == "ts")
        })
    {
        let path = entry.path();
        if let Some(fname) = path.file_name().and_then(|n| n.to_str()) {
            // 以下のいずれかにマッチするファイル名をメインルーティングとみなす
            if fname.ends_with("app-routing.module.ts")
                || fname.ends_with("app-routing.ts")
                || fname.ends_with("routing.module.ts")
                || fname.ends_with("routes.ts")
            {
                main_routing_paths.push(path.to_path_buf());
            }
        }
    }

    // メインルーティングファイルが見つからなければエラー
    if main_routing_paths.is_empty() {
        eprintln!("Error: メインのルーティングファイルが見つかりませんでした。");
        std::process::exit(1);
    }

    // 重複を除去
    main_routing_paths.sort();
    main_routing_paths.dedup();

    // 3) 見つけたメインルーティングファイルを順に解析
    let mut all_routes: Vec<RouteInfo> = Vec::new();
    for routing_path in main_routing_paths {
        println!("解析中: {:?}", routing_path);
        // 3-1) AST 解析して Vec<RouteInfo> を取得
        let mut top_routes = parse_routes_in_file(&routing_path)?;
        // 3-2) 各ルートに対して再帰的に子ルートを resolve
        for route in &mut top_routes {
            resolve_children_recursively(route, &project_dir)?;
        }
        // 3-3) 取得結果をマージ
        all_routes.extend(top_routes);
    }

    // 4) 最終的なルートツリーを JSON 化して標準出力
    let json = serde_json::to_string_pretty(&all_routes)?;
    println!("{}", json);

    Ok(())
}

/// 再帰的に children / loadChildren をたどって子ルートを解析する関数
fn resolve_children_recursively(
    route: &mut RouteInfo,
    project_root: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    // 1) children があれば、その子要素についても再帰的に処理
    for child in &mut route.children {
        resolve_children_recursively(child, project_root)?;
    }

    // 2) loadChildren があれば、その文字列をもとに実際のサブルーティングファイルを探し、再度 parse_routes_in_file を呼び出す
    if let Some(load_str) = &route.load_children {
        if let Some(child_routing_path) =
            resolve_load_children_path(load_str, &route.source_file, project_root)?
        {
            // サブルーティングファイルを AST 解析
            let mut sub_routes = parse_routes_in_file(&child_routing_path)?;
            // さらにそのサブルートでも同様に再帰的に処理
            for sub in &mut sub_routes {
                resolve_children_recursively(sub, project_root)?;
            }
            // 子ルートをマージ
            route.children.extend(sub_routes);
        }
    }

    Ok(())
}
