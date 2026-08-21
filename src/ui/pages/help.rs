//! Help page — the onboarding guide and feature reference.
//!
//! The page is static text: every topic is an `adw::ExpanderRow` whose body is
//! a wrapped label, so the page reads as a scannable table of contents until a
//! topic is opened.  Some topics end with buttons that jump straight to the
//! page being described, which is the difference between a manual and a guide.
//!
//! Content lives in `i18n/en/scenedeck.ftl` under the `help-` prefix.  Bodies
//! are deliberately multi-line Fluent messages: one line per point.

use std::rc::Rc;

use adw::{prelude::*, ExpanderRow, PreferencesGroup, PreferencesPage};
use gtk4::{Align, Box as GtkBox, Button, Image, Label, ListBoxRow, Orientation};
use i18n_embed_fl::fl;

use crate::controller::state::Page;
use crate::infra::i18n::LANGUAGE_LOADER;
use crate::ui::navigation::NavigationContext;

/// One "jump to that page" button under a topic body.
struct TopicLink {
    label: String,
    page: Page,
}

fn link(label: String, page: Page) -> TopicLink {
    TopicLink { label, page }
}

pub(crate) fn build(nav: NavigationContext) -> (gtk4::Widget, Rc<dyn Fn()>) {
    let page = PreferencesPage::builder()
        .title(fl!(LANGUAGE_LOADER, "help-page-title"))
        .icon_name("help-browser-symbolic")
        .build();
    page.add_css_class("app-page");
    page.add_css_class("help-page");
    page.add_css_class("app-preferences-page");

    page.add(&hero_group());
    page.add(&getting_started_group(&nav));
    page.add(&connecting_group(&nav));
    page.add(&scenes_group(&nav));
    page.add(&operating_group(&nav));
    page.add(&inspecting_group(&nav));
    page.add(&personalising_group(&nav));

    // The guide never changes at runtime, so there is nothing to refresh; the
    // stack still expects a callback for every page.
    let refresh: Rc<dyn Fn()> = Rc::new(|| {});
    (page.upcast(), refresh)
}

// ── Groups ────────────────────────────────────────────────────────────────────

fn hero_group() -> PreferencesGroup {
    let group = PreferencesGroup::new();

    let hero = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(8)
        .margin_top(6)
        .margin_bottom(6)
        .build();
    hero.add_css_class("help-hero");

    let icon = Image::from_icon_name("help-browser-symbolic");
    icon.set_pixel_size(48);
    icon.set_halign(Align::Start);
    hero.append(&icon);

    let title = Label::builder()
        .label(fl!(LANGUAGE_LOADER, "help-hero-title"))
        .xalign(0.0)
        .wrap(true)
        .build();
    title.add_css_class("title-1");
    hero.append(&title);

    let description = Label::builder()
        .label(fl!(LANGUAGE_LOADER, "help-hero-description"))
        .xalign(0.0)
        .wrap(true)
        .build();
    description.add_css_class("body");
    hero.append(&description);

    let hint = Label::builder()
        .label(fl!(LANGUAGE_LOADER, "help-expand-hint"))
        .xalign(0.0)
        .wrap(true)
        .build();
    hint.add_css_class("dim-label");
    hint.add_css_class("caption");
    hero.append(&hint);

    group.add(&hero);
    group
}

fn getting_started_group(nav: &NavigationContext) -> PreferencesGroup {
    let group = titled_group(
        fl!(LANGUAGE_LOADER, "help-group-start-title"),
        fl!(LANGUAGE_LOADER, "help-group-start-description"),
    );

    group.add(&topic(
        nav,
        "help-about-symbolic",
        fl!(LANGUAGE_LOADER, "help-quickstart-title"),
        fl!(LANGUAGE_LOADER, "help-quickstart-subtitle"),
        fl!(LANGUAGE_LOADER, "help-quickstart-body"),
        vec![
            link(fl!(LANGUAGE_LOADER, "help-open-settings"), Page::Settings),
            link(fl!(LANGUAGE_LOADER, "help-open-inventory"), Page::Inventory),
        ],
        true,
    ));
    group.add(&topic(
        nav,
        "view-grid-symbolic",
        fl!(LANGUAGE_LOADER, "help-concepts-title"),
        fl!(LANGUAGE_LOADER, "help-concepts-subtitle"),
        fl!(LANGUAGE_LOADER, "help-concepts-body"),
        vec![],
        false,
    ));

    group
}

fn connecting_group(nav: &NavigationContext) -> PreferencesGroup {
    let group = titled_group(
        fl!(LANGUAGE_LOADER, "help-group-connect-title"),
        fl!(LANGUAGE_LOADER, "help-group-connect-description"),
    );

    group.add(&topic(
        nav,
        "network-transmit-receive-symbolic",
        fl!(LANGUAGE_LOADER, "help-connect-local-title"),
        fl!(LANGUAGE_LOADER, "help-connect-local-subtitle"),
        fl!(LANGUAGE_LOADER, "help-connect-local-body"),
        vec![link(
            fl!(LANGUAGE_LOADER, "help-open-settings"),
            Page::Settings,
        )],
        false,
    ));
    group.add(&topic(
        nav,
        "network-server-symbolic",
        fl!(LANGUAGE_LOADER, "help-connect-remote-title"),
        fl!(LANGUAGE_LOADER, "help-connect-remote-subtitle"),
        fl!(LANGUAGE_LOADER, "help-connect-remote-body"),
        vec![link(
            fl!(LANGUAGE_LOADER, "help-open-settings"),
            Page::Settings,
        )],
        false,
    ));
    group.add(&topic(
        nav,
        "dialog-warning-symbolic",
        fl!(LANGUAGE_LOADER, "help-connect-trouble-title"),
        fl!(LANGUAGE_LOADER, "help-connect-trouble-subtitle"),
        fl!(LANGUAGE_LOADER, "help-connect-trouble-body"),
        vec![link(
            fl!(LANGUAGE_LOADER, "help-open-settings"),
            Page::Settings,
        )],
        false,
    ));

    group
}

fn scenes_group(nav: &NavigationContext) -> PreferencesGroup {
    let group = titled_group(
        fl!(LANGUAGE_LOADER, "help-group-scenes-title"),
        fl!(LANGUAGE_LOADER, "help-group-scenes-description"),
    );

    group.add(&topic(
        nav,
        "view-list-symbolic",
        fl!(LANGUAGE_LOADER, "help-scenes-hide-title"),
        fl!(LANGUAGE_LOADER, "help-scenes-hide-subtitle"),
        fl!(LANGUAGE_LOADER, "help-scenes-hide-body"),
        vec![
            link(fl!(LANGUAGE_LOADER, "help-open-inventory"), Page::Inventory),
            link(fl!(LANGUAGE_LOADER, "help-open-live"), Page::Live),
        ],
        true,
    ));
    group.add(&topic(
        nav,
        "view-sort-descending-symbolic",
        fl!(LANGUAGE_LOADER, "help-scenes-order-title"),
        fl!(LANGUAGE_LOADER, "help-scenes-order-subtitle"),
        fl!(LANGUAGE_LOADER, "help-scenes-order-body"),
        vec![link(
            fl!(LANGUAGE_LOADER, "help-open-inventory"),
            Page::Inventory,
        )],
        false,
    ));
    group.add(&topic(
        nav,
        "document-save-symbolic",
        fl!(LANGUAGE_LOADER, "help-scenes-registry-title"),
        fl!(LANGUAGE_LOADER, "help-scenes-registry-subtitle"),
        fl!(LANGUAGE_LOADER, "help-scenes-registry-body"),
        vec![link(
            fl!(LANGUAGE_LOADER, "help-open-inventory"),
            Page::Inventory,
        )],
        false,
    ));

    group
}

fn operating_group(nav: &NavigationContext) -> PreferencesGroup {
    let group = titled_group(
        fl!(LANGUAGE_LOADER, "help-group-operate-title"),
        fl!(LANGUAGE_LOADER, "help-group-operate-description"),
    );

    group.add(&topic(
        nav,
        "media-record-symbolic",
        fl!(LANGUAGE_LOADER, "help-live-title"),
        fl!(LANGUAGE_LOADER, "help-live-subtitle"),
        fl!(LANGUAGE_LOADER, "help-live-body"),
        vec![link(fl!(LANGUAGE_LOADER, "help-open-live"), Page::Live)],
        false,
    ));
    group.add(&topic(
        nav,
        "input-keyboard-symbolic",
        fl!(LANGUAGE_LOADER, "help-hotkeys-title"),
        fl!(LANGUAGE_LOADER, "help-hotkeys-subtitle"),
        fl!(LANGUAGE_LOADER, "help-hotkeys-body"),
        vec![link(
            fl!(LANGUAGE_LOADER, "help-open-settings"),
            Page::Settings,
        )],
        false,
    ));
    group.add(&topic(
        nav,
        "audio-volume-high-symbolic",
        fl!(LANGUAGE_LOADER, "help-audio-title"),
        fl!(LANGUAGE_LOADER, "help-audio-subtitle"),
        fl!(LANGUAGE_LOADER, "help-audio-body"),
        vec![link(fl!(LANGUAGE_LOADER, "help-open-mixer"), Page::Mixer)],
        false,
    ));
    group.add(&topic(
        nav,
        "media-playback-start-symbolic",
        fl!(LANGUAGE_LOADER, "help-outputs-title"),
        fl!(LANGUAGE_LOADER, "help-outputs-subtitle"),
        fl!(LANGUAGE_LOADER, "help-outputs-body"),
        vec![link(
            fl!(LANGUAGE_LOADER, "help-open-settings"),
            Page::Settings,
        )],
        false,
    ));

    group
}

fn inspecting_group(nav: &NavigationContext) -> PreferencesGroup {
    let group = titled_group(
        fl!(LANGUAGE_LOADER, "help-group-inspect-title"),
        fl!(LANGUAGE_LOADER, "help-group-inspect-description"),
    );

    group.add(&topic(
        nav,
        "emblem-default-symbolic",
        fl!(LANGUAGE_LOADER, "help-doctor-title"),
        fl!(LANGUAGE_LOADER, "help-doctor-subtitle"),
        fl!(LANGUAGE_LOADER, "help-doctor-body"),
        vec![link(fl!(LANGUAGE_LOADER, "help-open-doctor"), Page::Doctor)],
        false,
    ));
    group.add(&topic(
        nav,
        "view-grid-symbolic",
        fl!(LANGUAGE_LOADER, "help-graph-title"),
        fl!(LANGUAGE_LOADER, "help-graph-subtitle"),
        fl!(LANGUAGE_LOADER, "help-graph-body"),
        vec![link(fl!(LANGUAGE_LOADER, "help-open-graph"), Page::Graph)],
        false,
    ));
    group.add(&topic(
        nav,
        "power-profile-performance-symbolic",
        fl!(LANGUAGE_LOADER, "help-stats-title"),
        fl!(LANGUAGE_LOADER, "help-stats-subtitle"),
        fl!(LANGUAGE_LOADER, "help-stats-body"),
        vec![link(fl!(LANGUAGE_LOADER, "help-open-stats"), Page::Stats)],
        false,
    ));

    group
}

fn personalising_group(nav: &NavigationContext) -> PreferencesGroup {
    let group = titled_group(
        fl!(LANGUAGE_LOADER, "help-group-personalise-title"),
        fl!(LANGUAGE_LOADER, "help-group-personalise-description"),
    );

    group.add(&topic(
        nav,
        "preferences-system-symbolic",
        fl!(LANGUAGE_LOADER, "help-appearance-title"),
        fl!(LANGUAGE_LOADER, "help-appearance-subtitle"),
        fl!(LANGUAGE_LOADER, "help-appearance-body"),
        vec![link(
            fl!(LANGUAGE_LOADER, "help-open-settings"),
            Page::Settings,
        )],
        false,
    ));
    group.add(&topic(
        nav,
        "folder-symbolic",
        fl!(LANGUAGE_LOADER, "help-files-title"),
        fl!(LANGUAGE_LOADER, "help-files-subtitle"),
        fl!(LANGUAGE_LOADER, "help-files-body"),
        vec![],
        false,
    ));
    group.add(&topic(
        nav,
        "input-keyboard-symbolic",
        fl!(LANGUAGE_LOADER, "help-shortcuts-title"),
        fl!(LANGUAGE_LOADER, "help-shortcuts-subtitle"),
        fl!(LANGUAGE_LOADER, "help-shortcuts-body"),
        vec![],
        false,
    ));

    group
}

// ── Building blocks ───────────────────────────────────────────────────────────

fn titled_group(title: String, description: String) -> PreferencesGroup {
    PreferencesGroup::builder()
        .title(title)
        .description(description)
        .build()
}

/// One expandable topic: an icon, a title, a one-line summary, and a body that
/// only appears once the row is opened.
///
/// `expanded` opens the row on first show — reserved for the two topics a new
/// user should not have to go looking for.
fn topic(
    nav: &NavigationContext,
    icon_name: &str,
    title: String,
    subtitle: String,
    body: String,
    links: Vec<TopicLink>,
    expanded: bool,
) -> ExpanderRow {
    let row = ExpanderRow::builder()
        .title(title)
        .subtitle(subtitle)
        .expanded(expanded)
        .build();
    row.add_css_class("help-topic");

    let icon = Image::from_icon_name(icon_name);
    icon.add_css_class("scenedeck-row-icon");
    row.add_prefix(&icon);

    row.add_row(&body_row(nav, &body, links));
    row
}

fn body_row(nav: &NavigationContext, body: &str, links: Vec<TopicLink>) -> ListBoxRow {
    let content = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(12)
        .margin_top(12)
        .margin_bottom(12)
        .margin_start(12)
        .margin_end(12)
        .build();
    content.add_css_class("help-topic-body");

    let text = Label::builder()
        .label(body)
        .xalign(0.0)
        .wrap(true)
        .selectable(true)
        .build();
    text.add_css_class("body");
    content.append(&text);

    if !links.is_empty() {
        let actions = GtkBox::builder()
            .orientation(Orientation::Horizontal)
            .spacing(8)
            .halign(Align::Start)
            .build();
        for TopicLink { label, page } in links {
            let button = Button::builder().label(label).build();
            button.add_css_class("pill");
            button.connect_clicked({
                let nav = nav.clone();
                move |_| nav.switch_to_page(page)
            });
            actions.append(&button);
        }
        content.append(&actions);
    }

    ListBoxRow::builder()
        .activatable(false)
        .selectable(false)
        .child(&content)
        .build()
}
