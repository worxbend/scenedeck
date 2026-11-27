use super::scanner::{
    line_budget, module_files, production_line_count, ratchet, source_violations, src_root,
    Forbidden,
};

fn prefixes<'a>(names: &[&'a str]) -> Vec<Forbidden<'a>> {
    names.iter().copied().map(Forbidden::PathPrefix).collect()
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
        .filter(|(_, source)| source_violations(source, &prefixes(&["obws"])).len() != 1)
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
    let lines: Vec<_> = source_violations(source, &prefixes(&["obws"]))
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
            source_violations(source, &prefixes(&["crate::obs"])).len(),
            1,
            "{source}"
        );
    }
    let fixtures = r##"
items! { #[cfg(test)] use crate::obs::Fixture; }
fn explain() { log!("crate::obs::connect()", r#"obws::Client"#); }
"##;
    assert!(source_violations(fixtures, &prefixes(&["crate::obs", "obws"])).is_empty());
}

#[test]
fn scanner_exempts_nested_test_expressions_and_preserves_production_locations() {
    let source =
        "fn build() {\n#[cfg(test)]\n{ crate::obs::fixture(); }\ncrate::obs::connect();\n}";
    assert_eq!(
        source_violations(source, &prefixes(&["crate::obs"])),
        [(4, "crate::obs::connect();".to_string())]
    );
}

#[test]
fn scanner_expands_grouped_imports_before_checking_aliases() {
    let cases = [
        (Forbidden::PathPrefix("obws"), "use obws::{self as protocol, Client as Backend};"),
        (Forbidden::PathPrefix("gtk4"), "use gtk4::{self as toolkit, Widget};"),
        (Forbidden::PathPrefix("adw::"), "use adw::{self as toolkit, Application};"),
        (Forbidden::PathPrefix("glib::"), "use glib::{self as toolkit, MainContext};"),
        (Forbidden::PathPrefix("gio::"), "use gio::{self as toolkit, Action};"),
        (Forbidden::PathPrefix("crate::storage"), "use crate::{domain::Scene, storage::{self as disk, config::AppConfig}};"),
        (Forbidden::AnySegment("AppController"), "use crate::{controller::{app_controller::AppController as Controller}, domain::Scene};"),
        (Forbidden::PathPrefix("crate::obs"), "use crate::{domain::obs::ObsNamedList, obs::{self as backend, client::ObsClient as Client}};"),
        (Forbidden::PathPrefix("crate::obs"), "use crate::{obs::*};"),
        (Forbidden::PathPrefix("crate::obs"), "fn connect() { crate::obs::client::ObsClient::new(); }"),
    ];
    for (forbidden, source) in cases {
        assert_eq!(
            source_violations(source, &[forbidden]),
            [(1, source.to_string())],
            "{source}"
        );
    }
    assert!(source_violations(
        "use crate::domain::obs::ObsNamedList;",
        &prefixes(&["crate::obs"])
    )
    .is_empty());
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
        &prefixes(&[
            "obws",
            "crate::storage",
            "AppController",
            "crate::obs",
            "gtk4",
            "adw::",
        ]),
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
    assert!(source_violations(fixtures, &prefixes(&["obws"])).is_empty());
    assert!(source_violations("#![cfg(test)]\nuse obws::Client;", &prefixes(&["obws"])).is_empty());
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
fn ratchet_rejects_a_raised_count_and_an_unlisted_file() {
    let pattern = "fn lock_or_recover";
    let listed = ratchet("infra", pattern, &[("infra/sync.rs", 1)]);
    assert!(listed.is_empty(), "{listed:?}");
    let raised = ratchet("infra", pattern, &[("infra/sync.rs", 0)]);
    assert!(
        raised.iter().any(|hit| hit.contains("infra/sync.rs")),
        "{raised:?}"
    );
    let unlisted = ratchet("infra", pattern, &[]);
    assert!(
        unlisted.iter().any(|hit| hit.contains("infra/sync.rs")),
        "{unlisted:?}"
    );
}

#[test]
fn production_lines_skip_interleaved_test_only_items() {
    let source = [
        "use std::fmt;",
        "/// Documented fixture.",
        "#[cfg(test)]",
        "use crate::fixture;",
        "struct State {",
        "    count: usize,",
        "}",
        "impl State {",
        "    #[cfg(test)]",
        "    fn fixture() {}",
        "    fn production() {}",
        "}",
        "#[cfg(test)]",
        "mod tests {",
        "    fn helper() {}",
        "}",
        "fn after_tests() {}",
    ]
    .join("\n");
    assert_eq!(production_line_count(&source), 8);
}

#[test]
fn line_budget_rejects_a_listed_file_over_its_ceiling_and_any_file_over_the_cap() {
    let sync =
        production_line_count(&std::fs::read_to_string(src_root().join("infra/sync.rs")).unwrap());
    assert!(line_budget("infra/sync.rs", &[], sync).is_empty());
    let over_cap = line_budget("infra/sync.rs", &[], sync - 1);
    assert!(
        over_cap.iter().any(|hit| hit.contains("infra/sync.rs")),
        "{over_cap:?}"
    );
    let listed = line_budget("infra/sync.rs", &[("infra/sync.rs", sync)], 0);
    assert!(listed.is_empty(), "{listed:?}");
    let raised = line_budget("infra/sync.rs", &[("infra/sync.rs", sync - 1)], usize::MAX);
    assert!(
        raised.iter().any(|hit| hit.contains("infra/sync.rs")),
        "{raised:?}"
    );
}
