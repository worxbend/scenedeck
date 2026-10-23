//! Fitness tests for the module boundaries described in `CLAUDE.md`.
//!
//! Each boundary here was written down before it was checked, and at least
//! one of them had already been broken while the documentation still said
//! otherwise. Nothing failed when that happened, because a documented rule
//! that no check enforces is a suggestion. These tests turn the prose into
//! something that can be violated only on purpose.

#![cfg(test)]

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use syn::parse::discouraged::Speculative;
use syn::parse::{ParseStream, Parser};
use syn::spanned::Spanned;
use syn::visit::{self, Visit};

fn src_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
}

/// Production module files, including modules declared after test-only items.
fn source_files() -> Vec<PathBuf> {
    let root = src_root();
    let mut pending = vec![PathBuf::from("lib.rs"), PathBuf::from("main.rs")];
    let mut files = BTreeSet::new();
    while let Some(path) = pending.pop() {
        if files.contains(&path) {
            continue;
        }
        let source = std::fs::read_to_string(root.join(&path))
            .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
        let syntax = syn::parse_file(&source)
            .unwrap_or_else(|error| panic!("cannot parse {}: {error}", path.display()));
        if !test_only(&syntax.attrs) {
            pending.extend(
                module_files(&root.join(&path), &syntax)
                    .into_iter()
                    .map(|child| {
                        child
                            .strip_prefix(&root)
                            .expect("module stays under src/")
                            .to_path_buf()
                    }),
            );
            files.insert(path);
        }
    }
    assert!(!files.is_empty(), "found no production sources to check");
    files.into_iter().collect()
}

fn module_files(path: &Path, syntax: &syn::File) -> Vec<PathBuf> {
    let parent = path
        .parent()
        .expect("source has a parent directory")
        .to_path_buf();
    let stem = path.file_stem().expect("source has a file name");
    let directory = if ["lib", "main", "mod"].iter().any(|name| stem == *name) {
        parent.clone()
    } else {
        parent.join(stem)
    };
    let mut visitor = ModuleFiles {
        directory,
        path_directory: parent,
        files: Vec::new(),
    };
    visitor.visit_file(syntax);
    visitor.files
}

struct ModuleFiles {
    directory: PathBuf,
    path_directory: PathBuf,
    files: Vec<PathBuf>,
}

struct BoundaryVisitor<'a> {
    forbidden: &'a [&'a str],
    lines: BTreeSet<usize>,
}

fn test_only(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|attr| {
        attr.path().is_ident("cfg")
            && cfg_without_tests(&attr.parse_args::<syn::Meta>().expect("valid cfg predicate"))
                == Some(false)
    })
}

// Unknown features and target predicates remain unknown: only an expression
// proven false with `test = false` can remove production from the check.
fn cfg_without_tests(predicate: &syn::Meta) -> Option<bool> {
    match predicate {
        syn::Meta::Path(path) if path.is_ident("test") => Some(false),
        syn::Meta::List(list) if list.path.is_ident("not") => {
            let nested = list
                .parse_args::<syn::Meta>()
                .expect("one cfg(not) predicate");
            cfg_without_tests(&nested).map(|enabled| !enabled)
        }
        syn::Meta::List(list) if list.path.is_ident("all") || list.path.is_ident("any") => {
            let predicates = list
                .parse_args_with(
                    syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated,
                )
                .expect("valid cfg predicates");
            let states: Vec<_> = predicates.iter().map(cfg_without_tests).collect();
            let decisive = list.path.is_ident("any");
            if states.contains(&Some(decisive)) {
                Some(decisive)
            } else if states.iter().all(Option::is_some) {
                Some(!decisive)
            } else {
                None
            }
        }
        _ => None,
    }
}

macro_rules! visit_production {
    ($($method:ident($node:ident)),* $(,)?) => {
        $(fn $method(&mut self, node: &'ast syn::$node) {
            if !test_only(&node.attrs) {
                visit::$method(self, node);
            }
        })*
    };
}

impl<'ast> Visit<'ast> for ModuleFiles {
    visit_production! { visit_item_fn(ItemFn), visit_item_impl(ItemImpl) }

    fn visit_item_mod(&mut self, node: &'ast syn::ItemMod) {
        if test_only(&node.attrs) {
            return;
        }
        let explicit = node.attrs.iter().find_map(|attr| {
            if !attr.path().is_ident("path") {
                return None;
            }
            match &attr.meta {
                syn::Meta::NameValue(meta) => match &meta.value {
                    syn::Expr::Lit(literal) => match &literal.lit {
                        syn::Lit::Str(path) => Some(self.path_directory.join(path.value())),
                        _ => None,
                    },
                    _ => None,
                },
                _ => None,
            }
        });
        let directory = explicit.unwrap_or_else(|| self.directory.join(node.ident.to_string()));
        if node.content.is_some() {
            let parent_directory = std::mem::replace(&mut self.directory, directory.clone());
            let parent_path = std::mem::replace(&mut self.path_directory, directory);
            visit::visit_item_mod(self, node);
            self.directory = parent_directory;
            self.path_directory = parent_path;
        } else {
            let path = if node.attrs.iter().any(|attr| attr.path().is_ident("path")) {
                directory
            } else if directory.with_extension("rs").is_file() {
                directory.with_extension("rs")
            } else {
                directory.join("mod.rs")
            };
            self.files.push(path);
        }
    }
}

impl BoundaryVisitor<'_> {
    fn check_macro_tokens(&mut self, tokens: proc_macro2::TokenStream) {
        if let Ok(syntax) = syn::parse2::<syn::File>(tokens.clone()) {
            self.visit_file(&syntax);
        } else if let Ok(expressions) =
            syn::punctuated::Punctuated::<syn::Expr, syn::Token![,]>::parse_terminated
                .parse2(tokens.clone())
        {
            for expression in &expressions {
                self.visit_expr(expression);
            }
        } else {
            // Macro DSLs are not Rust expressions. Recover only paths using
            // syn's grammar; literals stay opaque and groups retain spans.
            let scan = |input: ParseStream<'_>| {
                while !input.is_empty() {
                    let fork = input.fork();
                    if let Ok(path) = fork.parse::<syn::Path>() {
                        input.advance_to(&fork);
                        self.visit_path(&path);
                    } else if let proc_macro2::TokenTree::Group(group) = input.parse()? {
                        self.check_macro_tokens(group.stream());
                    }
                }
                Ok(())
            };
            scan.parse2(tokens).expect("token stream can be traversed");
        }
    }

    fn check_path(&mut self, segments: &[String], span: proc_macro2::Span) {
        if self.forbidden.iter().any(|forbidden| {
            let prefix: Vec<_> = forbidden.trim_end_matches("::").split("::").collect();
            segments
                .iter()
                .map(String::as_str)
                .take(prefix.len())
                .eq(prefix)
                || *forbidden == "AppController" && segments.iter().any(|s| s == "AppController")
        }) {
            self.lines.insert(span.start().line);
        }
    }

    fn check_import(&mut self, prefix: Vec<String>, tree: &syn::UseTree) {
        let mut prefix = prefix;
        match tree {
            syn::UseTree::Path(path) => {
                prefix.push(path.ident.to_string());
                self.check_import(prefix, &path.tree);
            }
            syn::UseTree::Group(group) => {
                for branch in &group.items {
                    self.check_import(prefix.clone(), branch);
                }
            }
            syn::UseTree::Name(name) => {
                if name.ident != "self" {
                    prefix.push(name.ident.to_string());
                }
                self.check_path(&prefix, name.ident.span());
            }
            syn::UseTree::Rename(rename) => {
                if rename.ident != "self" {
                    prefix.push(rename.ident.to_string());
                }
                self.check_path(&prefix, rename.ident.span());
            }
            syn::UseTree::Glob(glob) => self.check_path(&prefix, glob.star_token.span),
        }
    }
}

impl<'ast> Visit<'ast> for BoundaryVisitor<'_> {
    visit_production! {
        visit_file(File),
        visit_item_const(ItemConst), visit_item_enum(ItemEnum),
        visit_item_fn(ItemFn), visit_item_foreign_mod(ItemForeignMod),
        visit_item_impl(ItemImpl), visit_item_macro(ItemMacro),
        visit_item_mod(ItemMod), visit_item_static(ItemStatic),
        visit_item_struct(ItemStruct), visit_item_trait(ItemTrait),
        visit_item_trait_alias(ItemTraitAlias), visit_item_type(ItemType),
        visit_item_union(ItemUnion),
        visit_impl_item_const(ImplItemConst), visit_impl_item_fn(ImplItemFn),
        visit_impl_item_macro(ImplItemMacro), visit_impl_item_type(ImplItemType),
        visit_trait_item_const(TraitItemConst), visit_trait_item_fn(TraitItemFn),
        visit_trait_item_macro(TraitItemMacro), visit_trait_item_type(TraitItemType),
        visit_foreign_item_fn(ForeignItemFn), visit_foreign_item_macro(ForeignItemMacro),
        visit_foreign_item_static(ForeignItemStatic), visit_foreign_item_type(ForeignItemType),
        visit_field(Field), visit_variant(Variant), visit_arm(Arm),
        visit_local(Local), visit_stmt_macro(StmtMacro), visit_field_value(FieldValue),
        visit_expr_array(ExprArray),
        visit_expr_assign(ExprAssign),
        visit_expr_async(ExprAsync),
        visit_expr_await(ExprAwait),
        visit_expr_binary(ExprBinary),
        visit_expr_block(ExprBlock),
        visit_expr_break(ExprBreak),
        visit_expr_call(ExprCall),
        visit_expr_cast(ExprCast),
        visit_expr_closure(ExprClosure),
        visit_expr_const(ExprConst),
        visit_expr_continue(ExprContinue),
        visit_expr_field(ExprField),
        visit_expr_for_loop(ExprForLoop),
        visit_expr_group(ExprGroup),
        visit_expr_if(ExprIf),
        visit_expr_index(ExprIndex),
        visit_expr_infer(ExprInfer),
        visit_expr_let(ExprLet),
        visit_expr_lit(ExprLit),
        visit_expr_loop(ExprLoop),
        visit_expr_macro(ExprMacro),
        visit_expr_match(ExprMatch),
        visit_expr_method_call(ExprMethodCall),
        visit_expr_paren(ExprParen),
        visit_expr_path(ExprPath),
        visit_expr_range(ExprRange),
        visit_expr_raw_addr(ExprRawAddr),
        visit_expr_reference(ExprReference),
        visit_expr_repeat(ExprRepeat),
        visit_expr_return(ExprReturn),
        visit_expr_struct(ExprStruct),
        visit_expr_try(ExprTry),
        visit_expr_try_block(ExprTryBlock),
        visit_expr_tuple(ExprTuple),
        visit_expr_unary(ExprUnary),
        visit_expr_unsafe(ExprUnsafe),
        visit_expr_while(ExprWhile),
        visit_expr_yield(ExprYield),
    }

    fn visit_item_use(&mut self, node: &'ast syn::ItemUse) {
        if !test_only(&node.attrs) {
            self.check_import(Vec::new(), &node.tree);
        }
    }

    fn visit_item_extern_crate(&mut self, node: &'ast syn::ItemExternCrate) {
        if !test_only(&node.attrs) {
            self.check_path(&[node.ident.to_string()], node.ident.span());
        }
    }

    fn visit_path(&mut self, path: &'ast syn::Path) {
        let segments: Vec<_> = path.segments.iter().map(|s| s.ident.to_string()).collect();
        self.check_path(&segments, path.span());
        visit::visit_path(self, path);
    }

    fn visit_macro(&mut self, node: &'ast syn::Macro) {
        self.visit_path(&node.path);
        self.check_macro_tokens(node.tokens.clone());
    }
}

fn source_violations(source: &str, forbidden: &[&str]) -> Vec<(usize, String)> {
    let syntax = syn::parse_file(source).expect("architecture scanner requires valid Rust");
    let mut visitor = BoundaryVisitor {
        forbidden,
        lines: BTreeSet::new(),
    };
    visitor.visit_file(&syntax);
    source
        .lines()
        .enumerate()
        .filter(|(index, _)| visitor.lines.contains(&(index + 1)))
        .map(|(index, line)| (index + 1, line.to_string()))
        .collect()
}

/// Report every production line under `scope` that names a forbidden term.
///
/// `scope` is a path prefix relative to `src/`; an empty prefix means the
/// whole crate. `exempt` lists files allowed to break the rule, which is
/// how a boundary names its own gateway.
fn violations(scope: &str, forbidden: &[&str], exempt: &[&str]) -> Vec<String> {
    source_files()
        .into_iter()
        .filter(|path| path.starts_with(scope))
        .filter(|path| !exempt.iter().any(|allowed| path == Path::new(allowed)))
        .flat_map(|path| {
            let source =
                std::fs::read_to_string(src_root().join(&path)).expect("source should be readable");
            source_violations(&source, forbidden)
                .into_iter()
                .map(|(number, line)| format!("  {}:{}: {}", path.display(), number, line.trim()))
                .collect::<Vec<_>>()
        })
        .collect()
}

#[test]
fn scanner_resumes_after_test_only_syntax() {
    let cases = [
        (
            "import",
            "#[cfg(test)]\nuse crate::fixture;\nuse obws::Client;",
        ),
        (
            "field",
            "struct State {\n#[cfg(test)]\nfixture: usize,\nproduction: obws::Client,\n}",
        ),
        (
            "module",
            "#[cfg(test)]\nmod tests { use obws::Fixture; }\nuse obws::Client;",
        ),
        (
            "impl helper",
            "impl State {\n#[cfg(test)]\nfn fixture() {}\nfn production(_: obws::Client) {}\n}",
        ),
    ];
    let missed: Vec<_> = cases
        .into_iter()
        .filter(|(_, source)| source_violations(source, &["obws"]).len() != 1)
        .map(|(name, _)| name)
        .collect();
    assert!(
        missed.is_empty(),
        "missed production after test-only syntax: {missed:?}"
    );
}

#[test]
fn scanner_exempts_only_cfgs_that_cannot_ship_without_tests() {
    let source = r#"
#[cfg(all(test, feature = "fixtures"))]
use obws::Fixture;
#[cfg(any(test, all(test, unix)))]
use obws::OtherFixture;
#[cfg(not(not(test)))]
use obws::ThirdFixture;
#[cfg(not(test))]
use obws::Production;
#[cfg(any(test, feature = "production"))]
use obws::OptionalProduction;
#[cfg_attr(test, allow(dead_code))]
use obws::AlsoProduction;
#[cfg(unix)]
use obws::PlatformProduction;
"#;
    let lines: Vec<_> = source_violations(source, &["obws"])
        .into_iter()
        .map(|(line, _)| line)
        .collect();
    assert_eq!(lines, [9, 11, 13, 15]);
}

#[test]
fn scanner_checks_visible_macro_paths_but_not_macro_literals() {
    let cases = [
        "fn build() { callback!(crate::obs::connect()); }",
        "fn build() { log!(error = %crate::obs::status(), \"failed\"); }",
        "macro_rules! build { () => {{ crate::obs::connect() }}; }",
        "items! { use crate::{obs::client::ObsClient as Backend}; }",
    ];
    for source in cases {
        assert_eq!(
            source_violations(source, &["crate::obs"]).len(),
            1,
            "{source}"
        );
    }
    let fixtures = r##"
items! { #[cfg(test)] use crate::obs::Fixture; }
fn explain() { log!("crate::obs::connect()", r#"obws::Client"#); }
"##;
    assert!(source_violations(fixtures, &["crate::obs", "obws"]).is_empty());
}

#[test]
fn scanner_exempts_nested_test_expressions_and_preserves_production_locations() {
    let source =
        "fn build() {\n#[cfg(test)]\n{ crate::obs::fixture(); }\ncrate::obs::connect();\n}";
    assert_eq!(
        source_violations(source, &["crate::obs"]),
        [(4, "crate::obs::connect();".to_string())]
    );
}

#[test]
fn scanner_expands_grouped_imports_before_checking_aliases() {
    let cases = [
        ("obws", "use obws::{self as protocol, Client as Backend};"),
        ("gtk4", "use gtk4::{self as toolkit, Widget};"),
        ("adw::", "use adw::{self as toolkit, Application};"),
        ("glib::", "use glib::{self as toolkit, MainContext};"),
        ("gio::", "use gio::{self as toolkit, Action};"),
        ("crate::storage", "use crate::{domain::Scene, storage::{self as disk, config::AppConfig}};"),
        ("AppController", "use crate::{controller::{app_controller::AppController as Controller}, domain::Scene};"),
        ("crate::obs", "use crate::{domain::obs::ObsNamedList, obs::{self as backend, client::ObsClient as Client}};"),
        ("crate::obs", "use crate::{obs::*};"),
        ("crate::obs", "fn connect() { crate::obs::client::ObsClient::new(); }"),
    ];
    for (forbidden, source) in cases {
        assert_eq!(
            source_violations(source, &[forbidden]),
            [(1, source.to_string())],
            "{source}"
        );
    }
    assert!(source_violations("use crate::domain::obs::ObsNamedList;", &["crate::obs"]).is_empty());
}

#[test]
fn scanner_does_not_treat_comments_or_literals_as_dependencies_or_cfgs() {
    let source = r##"
/* use obws::Client;
   crate::storage::AppConfig */
/// AppController and obws::Client are described here.
const HELP: &str = "#[cfg(test)] use obws::Client; crate::obs::connect()";
const BYTES: &[u8] = br#"gtk4::Widget and adw::Application"#;
fn obws_status() {}
use obws::Client; // This import, not the text above, violates the rule.
"##;
    let offenders = source_violations(
        source,
        &[
            "obws",
            "crate::storage",
            "AppController",
            "crate::obs",
            "gtk4",
            "adw::",
        ],
    );
    assert_eq!(
        offenders,
        [(
            8,
            "use obws::Client; // This import, not the text above, violates the rule.".to_string()
        )]
    );
}

#[test]
fn scanner_exempts_test_only_imports_fields_variants_and_associated_items() {
    let fixtures = r#"
#[cfg(test)] use obws::Client;
#[cfg(test)] extern crate obws as protocol;
struct State { #[cfg(test)] client: obws::Client }
enum StateKind { #[cfg(test)] Fixture(obws::Client), Production }
impl State { #[cfg(test)] fn fixture(_: obws::Client) {} }
trait Factory { #[cfg(test)] fn fixture() -> obws::Client; }
#[cfg(test)] impl obws::Fixture for State {}
#[cfg(test)] mod tests { use obws::Client; }
fn production() {}
"#;
    assert!(source_violations(fixtures, &["obws"]).is_empty());
    assert!(source_violations("#![cfg(test)]\nuse obws::Client;", &["obws"]).is_empty());
}

#[test]
fn scanner_module_discovery_skips_test_only_out_of_line_fixtures() {
    let source = r#"
#[cfg(test)] #[path = "fixture.rs"] mod tests;
#[path = "production.rs"] mod production;
mod nested {
    #[cfg(test)] mod fixture;
    #[path = "worker.rs"] mod worker;
}
"#;
    let parent = src_root().join("ui");
    let syntax = syn::parse_file(source).unwrap();
    assert_eq!(
        module_files(&parent.join("page.rs"), &syntax),
        [
            parent.join("production.rs"),
            parent.join("page/nested/worker.rs"),
        ]
    );
}

#[test]
fn ui_never_reaches_the_obs_adapter_directly() {
    let offenders = violations("ui", &["crate::obs"], &[]);
    assert!(
        offenders.is_empty(),
        "UI must dispatch an AppCommand through NavigationContext, not call src/obs/ directly:\n{}",
        offenders.join("\n")
    );
}

/// The OBS WebSocket library stays inside the OBS adapter.
///
/// Keeping `obws` in one module is what makes the protocol replaceable and
/// keeps protocol details out of code that decides what the app should do.
/// This had already been broken once: the controller's event loop matched
/// raw `obws` event variants and carried its own copy of the mapping that
/// `obs/mapper.rs` already owned.
#[test]
fn only_the_obs_adapter_imports_obws() {
    let offenders = violations("", &["obws"], &[]);
    let offenders: Vec<String> = offenders
        .into_iter()
        .filter(|hit| !hit.starts_with("  obs/"))
        .collect();

    assert!(
        offenders.is_empty(),
        "`obws` may only be used inside src/obs/. Wrap it in `ObsClient`, \
         `ObsEventStream`, or a `mapper` function and hand a domain type \
         across the boundary instead:\n{}",
        offenders.join("\n")
    );
}

/// Domain types stay free of GTK and of the OBS protocol.
///
/// `src/domain/` holds what the app is about — scenes, roles, audio levels,
/// output state. If a widget type or a protocol type reaches in there, the
/// concepts stop being testable without a display or a running OBS, and
/// the app's own vocabulary starts being shaped by its dependencies.
#[test]
fn domain_types_depend_on_neither_gtk_nor_obs() {
    let offenders = violations("domain", &["gtk4", "adw::", "glib::", "gio::", "obws"], &[]);

    assert!(
        offenders.is_empty(),
        "src/domain/ must not depend on GTK or on `obws`; convert at the \
         adapter boundary instead:\n{}",
        offenders.join("\n")
    );
}

/// Services stay free of GTK.
///
/// `src/services/` is pure logic over domain snapshots — dB conversion,
/// graph edge classification, the Doctor's checks. Keeping GTK out is what
/// lets those rules be tested by calling them, with no display, no main
/// loop, and no widget tree to assemble first.
#[test]
fn services_hold_no_gtk() {
    let offenders = violations("services", &["gtk4", "adw::", "glib::", "gio::"], &[]);

    assert!(
        offenders.is_empty(),
        "src/services/ must not depend on GTK. A service decides what is \
         true; a widget decides how to show it:\n{}",
        offenders.join("\n")
    );
}

/// Neither the domain nor the services know how anything is stored.
///
/// `src/storage/` owns the serde representations, the file layout, and the
/// keyring. Its types convert to domain snapshots at the boundary. If the
/// dependency ran the other way, changing the shape of `config.json` would
/// mean editing the rules of the application.
#[test]
fn domain_and_services_do_not_reach_into_storage() {
    let mut offenders = violations("domain", &["crate::storage"], &[]);
    offenders.extend(violations("services", &["crate::storage"], &[]));

    assert!(
        offenders.is_empty(),
        "src/domain/ and src/services/ must not depend on src/storage/. \
         Convert a storage type into a domain snapshot and pass that \
         instead:\n{}",
        offenders.join("\n")
    );
}

/// Pages and widgets reach the controller only through `NavigationContext`.
///
/// `NavigationContext` exposes exactly two things: switch to a page, and
/// dispatch a command. That narrow surface is what keeps the pages from
/// growing their own opinions about connection lifecycles or OBS calls.
///
/// `window.rs` and `navigation.rs` are exempt because they are where the
/// application is assembled: one builds the controller, the other wraps it.
#[test]
fn pages_reach_the_controller_only_through_navigation() {
    let offenders = violations(
        "ui",
        &["AppController"],
        &["ui/window.rs", "ui/navigation.rs"],
    );

    assert!(
        offenders.is_empty(),
        "UI pages and widgets must go through `NavigationContext` \
         (`switch_to_page` / `dispatch`) rather than naming \
         `AppController` directly:\n{}",
        offenders.join("\n")
    );
}
