use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use syn::parse::discouraged::Speculative;
use syn::parse::{ParseStream, Parser};
use syn::spanned::Spanned;
use syn::visit::{self, Visit};

pub(super) fn src_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
}

fn read_source(path: &Path) -> String {
    std::fs::read_to_string(src_root().join(path))
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()))
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
        let source = read_source(&path);
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

pub(super) fn module_files(path: &Path, syntax: &syn::File) -> Vec<PathBuf> {
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

/// How a boundary names a dependency it refuses.
#[derive(Clone, Copy)]
pub(super) enum Forbidden<'a> {
    /// The path starts with this `::` prefix. A trailing `::` is ignored.
    PathPrefix(&'a str),
    /// Any segment of the path is exactly this name.
    AnySegment(&'a str),
}

impl Forbidden<'_> {
    fn matches(self, segments: &[String]) -> bool {
        match self {
            Self::PathPrefix(prefix) => {
                let prefix: Vec<_> = prefix.trim_end_matches("::").split("::").collect();
                segments
                    .iter()
                    .map(String::as_str)
                    .take(prefix.len())
                    .eq(prefix)
            }
            Self::AnySegment(name) => segments.iter().any(|segment| segment == name),
        }
    }
}

struct BoundaryVisitor<'a> {
    forbidden: &'a [Forbidden<'a>],
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
        if self
            .forbidden
            .iter()
            .any(|forbidden| forbidden.matches(segments))
        {
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

pub(super) fn source_violations(source: &str, forbidden: &[Forbidden<'_>]) -> Vec<(usize, String)> {
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
/// whole crate. `exempt_prefixes` are path prefixes, also relative to
/// `src/`, for the gateway files a boundary is allowed to name. An exact
/// file matches itself. `Path::starts_with` uses whole components, so
/// `inventory.rs` is not covered by the prefix `inventory`.
pub(super) fn violations(
    scope: &str,
    forbidden: &[Forbidden<'_>],
    exempt_prefixes: &[&str],
) -> Vec<String> {
    source_files()
        .into_iter()
        .filter(|path| path.starts_with(scope))
        .filter(|path| {
            !exempt_prefixes
                .iter()
                .any(|prefix| path.starts_with(prefix))
        })
        .flat_map(|path| {
            let source = read_source(&path);
            source_violations(&source, forbidden)
                .into_iter()
                .map(|(number, line)| format!("  {}:{}: {}", path.display(), number, line.trim()))
                .collect::<Vec<_>>()
        })
        .collect()
}

/// Count literal `pattern` matches under `scope`.
///
/// A file over its ceiling fails. A file with matches and no ceiling fails
/// too: a new file must be added to the list on purpose. Zero matches are
/// not a new file, so a count that falls to zero is allowed. This counts
/// source text, not lines and not syntax.
pub(super) fn ratchet(scope: &str, pattern: &str, ceilings: &[(&str, usize)]) -> Vec<String> {
    assert!(!pattern.is_empty(), "ratchet pattern must not be empty");
    source_files()
        .into_iter()
        .filter(|path| path.starts_with(scope))
        .filter_map(|path| {
            let source = read_source(&path);
            let count = source.matches(pattern).count();
            if count == 0 {
                return None;
            }
            let key = path.to_str().expect("source path is utf-8");
            match ceilings.iter().find(|(name, _)| *name == key) {
                Some((_, ceiling)) if count <= *ceiling => None,
                Some((_, ceiling)) => Some(format!(
                    "  {key}: {count} occurrences of {pattern:?}, ceiling {ceiling}"
                )),
                None => Some(format!(
                    "  {key}: {count} occurrences of {pattern:?}, no ceiling"
                )),
            }
        })
        .collect()
}
