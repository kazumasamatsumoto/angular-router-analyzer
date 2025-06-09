use swc_common::{sync::Lrc, FileName, SourceMap};
use swc_ecma_ast::*;
use swc_ecma_parser::{lexer::Lexer, Parser as SwcParser, StringInput, Syntax, TsConfig};
use swc_ecma_visit::{Visit, VisitWith};
use std::collections::HashMap;

use std::fs;
use std::path::Path;

use crate::model::RouteInfo;

/// AST をトラバースしてルート定義を抽出するための Visitor
struct RouteVisitor {
    /// この Visitor が解析対象としているファイル (PathBuf)
    source_file: std::path::PathBuf,
    /// 見つかった RouteInfo を格納する Vec
    pub routes: Vec<RouteInfo>,
    /// 変数名とその配列リテラルのマッピング
    route_variables: HashMap<String, Vec<ObjectLit>>,
    /// デバッグ用のカウンタ
    debug_call_count: usize,
    debug_router_module_count: usize,
}

impl RouteVisitor {
    fn new(source_file: std::path::PathBuf) -> Self {
        RouteVisitor {
            source_file,
            routes: Vec::new(),
            route_variables: HashMap::new(),
            debug_call_count: 0,
            debug_router_module_count: 0,
        }
    }

    /// ObjectLit (例: `{ path: "home", loadChildren: () => import("…") }`) を受け取り
    /// RouteInfo 構造体を構築して返すヘルパーメソッド
    fn parse_route_object(&self, obj_lit: &ObjectLit) -> RouteInfo {
        let mut path: Option<String> = None;
        let mut load_children: Option<String> = None;
        let mut children: Vec<RouteInfo> = Vec::new();

        println!("  → ルートオブジェクトを解析中: {} プロパティ", obj_lit.props.len());

        for prop in &obj_lit.props {
            if let PropOrSpread::Prop(boxed_prop) = prop {
                if let Prop::KeyValue(KeyValueProp { key, value }) = &**boxed_prop {
                    if let PropName::Ident(ident) = key {
                        let key_name = ident.sym.to_string();
                        println!("    → プロパティ発見: {}", key_name);
                        
                        match key_name.as_str() {
                            "path" => {
                                if let Expr::Lit(Lit::Str(Str { value: s, .. })) = &**value {
                                    path = Some(s.to_string());
                                    println!("      → path: {}", s);
                                }
                            }
                            "loadChildren" => {
                                if let Expr::Arrow(ArrowExpr { body, .. }) = &**value {
                                    load_children = Some(format!("{:?}", body));
                                    println!("      → loadChildren: {:?}", body);
                                }
                            }
                            "children" => {
                                if let Expr::Array(arr_lit) = &**value {
                                    println!("      → children配列: {} 要素", arr_lit.elems.len());
                                    for elem in &arr_lit.elems {
                                        if let Some(elem_ref) = elem {
                                            if let Expr::Object(child_obj) = &*elem_ref.expr {
                                                let child_route = self.parse_route_object(child_obj);
                                                children.push(child_route);
                                            }
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        RouteInfo {
            path,
            load_children,
            source_file: self.source_file.clone(),
            children,
        }
    }

    /// 配列からルート情報を抽出する
    fn extract_routes_from_array(&self, arr_lit: &ArrayLit) -> Vec<RouteInfo> {
        let mut routes = Vec::new();
        println!("    → ルート配列発見: {} 要素", arr_lit.elems.len());
        
        for (i, elem) in arr_lit.elems.iter().enumerate() {
            if let Some(expr_and_span) = elem {
                println!("      → ルート要素 {}: {:?}", i, expr_and_span.expr);
                if let Expr::Object(obj_lit_inner) = &*expr_and_span.expr {
                    let route_info = self.parse_route_object(obj_lit_inner);
                    println!("      → ルート情報生成: path={:?}, loadChildren={:?}", route_info.path, route_info.load_children);
                    routes.push(route_info);
                }
            }
        }
        routes
    }
}

impl Visit for RouteVisitor {
    /// 変数宣言をキャッチして、routesという名前の配列を記録する
    fn visit_var_decl(&mut self, var_decl: &VarDecl) {
        for declarator in &var_decl.decls {
            if let Pat::Ident(BindingIdent { id, .. }) = &declarator.name {
                let var_name = id.sym.to_string();
                
                if var_name == "routes" || var_name.contains("route") {
                    println!("  → 変数宣言発見: {}", var_name);
                    
                    if let Some(init_expr) = &declarator.init {
                        if let Expr::Array(arr_lit) = &**init_expr {
                            println!("    → routes配列定義発見: {} 要素", arr_lit.elems.len());
                            
                            let mut route_objects = Vec::new();
                            for elem in &arr_lit.elems {
                                if let Some(expr_and_span) = elem {
                                    if let Expr::Object(obj_lit) = &*expr_and_span.expr {
                                        route_objects.push(obj_lit.clone());
                                    }
                                }
                            }
                            
                            if !route_objects.is_empty() {
                                let count = route_objects.len(); // 先に長さを保存
                                self.route_variables.insert(var_name.clone(), route_objects);
                                println!("    → 変数 '{}' に {} 個のルートオブジェクトを保存", var_name, count);
                            }
                        }
                    }
                }
            }
        }
        
        // 子ノードも訪問
        var_decl.visit_children_with(self);
    }

    fn visit_call_expr(&mut self, call: &CallExpr) {
        self.debug_call_count += 1;
        
        if let Callee::Expr(expr) = &call.callee {
            if let Expr::Member(MemberExpr { obj, prop, .. }) = &**expr {
                if let Expr::Ident(obj_ident) = &**obj {
                    let obj_name = obj_ident.sym.to_string();
                    
                    if obj_name == "RouterModule" {
                        self.debug_router_module_count += 1;
                        println!("  → RouterModuleの呼び出し発見: {}", self.debug_router_module_count);
                        
                        if let MemberProp::Ident(prop_ident) = prop {
                            let method_name = prop_ident.sym.to_string();
                            println!("    → メソッド: {}", method_name);
                            
                            if method_name == "forRoot" || method_name == "forChild" {
                                println!("    → forRoot/forChild発見! 引数数: {}", call.args.len());
                                
                                if call.args.len() == 1 {
                                    match &*call.args[0].expr {
                                        // 直接配列リテラルの場合
                                        Expr::Array(arr_lit) => {
                                            let extracted_routes = self.extract_routes_from_array(arr_lit);
                                            self.routes.extend(extracted_routes);
                                        }
                                        // 変数参照の場合
                                        Expr::Ident(ident) => {
                                            let var_name = ident.sym.to_string();
                                            println!("    → 変数参照発見: {}", var_name);
                                            
                                            if let Some(route_objects) = self.route_variables.get(&var_name) {
                                                println!("    → 変数 '{}' からルート定義を取得: {} 個", var_name, route_objects.len());
                                                for obj_lit in route_objects {
                                                    let route_info = self.parse_route_object(obj_lit);
                                                    println!("      → ルート情報生成: path={:?}, loadChildren={:?}", route_info.path, route_info.load_children);
                                                    self.routes.push(route_info);
                                                }
                                            } else {
                                                println!("    → 警告: 変数 '{}' の定義が見つかりません", var_name);
                                            }
                                        }
                                        other => {
                                            println!("    → 第一引数が配列でも変数でもありません: {:?}", other);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        call.visit_children_with(self);
    }
}

pub fn parse_routes_in_file(file_path: &Path) -> Result<Vec<RouteInfo>, Box<dyn std::error::Error>> {
    println!("ファイル解析開始: {:?}", file_path);
    
    let src = fs::read_to_string(file_path)?;
    println!("ファイルサイズ: {} bytes", src.len());
    
    let cm: Lrc<SourceMap> = Default::default();

    let fm = cm.new_source_file(
        FileName::Real(file_path.to_path_buf()), 
        src
    );

    // TypeScript構文でパースする設定
    let syntax = Syntax::Typescript(TsConfig {
        tsx: false,
        decorators: true,
        dts: false,
        no_early_errors: true,
        disallow_ambiguous_jsx_like: true,
    });

    let lexer = Lexer::new(
        syntax,
        Default::default(), // es version
        StringInput::from(&*fm),
        None,
    );

    let mut parser = SwcParser::new_from(lexer);

    let module = parser.parse_module().map_err(|e| {
        eprintln!("Parsing error in {:?}: {:?}", file_path, e);
        format!("Parse error: {:?}", e)
    })?;

    println!("パース成功。モジュール解析開始...");

    let mut visitor = RouteVisitor::new(file_path.to_path_buf());
    visitor.visit_module(&module);

    println!("解析完了:");
    println!("  - 総CallExpr数: {}", visitor.debug_call_count);
    println!("  - RouterModule呼び出し数: {}", visitor.debug_router_module_count);
    println!("  - 発見された変数数: {}", visitor.route_variables.len());
    println!("  - 発見されたルート数: {}", visitor.routes.len());

    Ok(visitor.routes)
}