//! Sidebar and OBS selector chrome for the main window.

use std::cell::Cell;
use std::rc::Rc;

use adw::prelude::*;
use gtk4::{
    Box as GtkBox, Button, DropDown, Image, Label, ListBox, Orientation, SelectionMode, StringList,
};
use i18n_embed_fl::fl;

use crate::app_info::{APP_ID, APP_NAME};
use crate::controller::command::AppCommand;
use crate::controller::state::{AppState, ObsStatus, Page};
use crate::domain::obs::ObsNamedList;
use crate::domain::output::OutputStatus;
use crate::infra::i18n::LANGUAGE_LOADER;
use crate::ui::navigation::NavigationContext;
use crate::ui::pages::live;
use crate::ui::widgets::status_bar;

pub(super) const NAV_PAGES: [Page; 8] = [
    Page::Live,
    Page::Mixer,
    Page::Stats,
    Page::Inventory,
    Page::Graph,
    Page::Doctor,
    Page::Settings,
    Page::Help,
];

#[derive(Clone)]
pub(super) struct HeaderSelectors {
    menu: gtk4::MenuButton,
    profiles: NamedSelector,
    scene_collections: NamedSelector,
}

impl HeaderSelectors {
    pub(super) fn build(nav: &NavigationContext) -> Self {
        let profiles = NamedSelector::build(
            &fl!(LANGUAGE_LOADER, "window-selector-profile-label"),
            &fl!(LANGUAGE_LOADER, "window-selector-profile-tooltip"),
        );
        profiles.connect_selection(nav, AppCommand::SetCurrentProfile);

        let scene_collections = NamedSelector::build(
            &fl!(LANGUAGE_LOADER, "window-selector-collection-label"),
            &fl!(LANGUAGE_LOADER, "window-selector-collection-tooltip"),
        );
        scene_collections.connect_selection(nav, AppCommand::SetCurrentSceneCollection);

        let content = GtkBox::builder()
            .orientation(Orientation::Vertical)
            .spacing(12)
            .margin_start(12)
            .margin_end(12)
            .margin_top(12)
            .margin_bottom(12)
            .build();
        content.append(&scene_collections.root);
        content.append(&profiles.root);
        let popover = gtk4::Popover::builder().child(&content).build();
        let menu = gtk4::MenuButton::builder()
            .icon_name("preferences-system-symbolic")
            .tooltip_text(fl!(LANGUAGE_LOADER, "header-obs-setup"))
            .popover(&popover)
            .sensitive(false)
            .build();
        Self {
            profiles,
            scene_collections,
            menu,
        }
    }

    pub(super) fn pack_into(&self, header: &adw::HeaderBar) {
        header.pack_start(&self.menu);
    }

    fn update_context(&self) {
        let profile = self
            .profiles
            .model
            .string(self.profiles.dropdown.selected())
            .map(|value| value.to_string())
            .unwrap_or_else(|| fl!(LANGUAGE_LOADER, "header-obs-unknown"));
        let collection = self
            .scene_collections
            .model
            .string(self.scene_collections.dropdown.selected())
            .map(|value| value.to_string())
            .unwrap_or_else(|| fl!(LANGUAGE_LOADER, "header-obs-unknown"));
        let context = fl!(
            LANGUAGE_LOADER,
            "header-obs-context",
            profile = profile,
            collection = collection
        );
        self.menu.set_tooltip_text(Some(&context));
        self.menu
            .update_property(&[gtk4::accessible::Property::Label(&context)]);
        self.menu.set_sensitive(
            self.profiles.model.n_items() > 0 || self.scene_collections.model.n_items() > 0,
        );
    }

    pub(super) fn update_profiles(&self, profiles: &ObsNamedList) {
        self.profiles.update(profiles);
        self.update_context();
    }

    pub(super) fn update_scene_collections(&self, collections: &ObsNamedList) {
        self.scene_collections.update(collections);
        self.update_context();
    }

    pub(super) fn clear(&self) {
        self.profiles.update(&ObsNamedList::default());
        self.scene_collections.update(&ObsNamedList::default());
        self.update_context();
    }
}

#[derive(Clone)]
struct NamedSelector {
    root: GtkBox,
    dropdown: DropDown,
    model: StringList,
    updating: Rc<Cell<bool>>,
}

impl NamedSelector {
    fn build(label: &str, tooltip: &str) -> Self {
        let root = GtkBox::builder()
            .orientation(Orientation::Vertical)
            .spacing(6)
            .valign(gtk4::Align::Center)
            .build();
        root.set_visible(false);
        root.add_css_class("header-selector");

        let caption = Label::builder()
            .label(label)
            .xalign(0.0)
            .valign(gtk4::Align::Center)
            .build();
        caption.add_css_class("caption");

        let model = StringList::new(&[]);
        let dropdown = DropDown::builder()
            .model(&model)
            .selected(gtk4::INVALID_LIST_POSITION)
            .sensitive(false)
            .build();
        dropdown.add_css_class("scenedeck-dropdown");
        dropdown.set_tooltip_text(Some(tooltip));
        dropdown.set_enable_search(true);
        let factory = gtk4::SignalListItemFactory::new();
        factory.connect_setup(|_, item| {
            if let Some(item) = item.downcast_ref::<gtk4::ListItem>() {
                item.set_child(Some(
                    &Label::builder()
                        .xalign(0.0)
                        .ellipsize(gtk4::pango::EllipsizeMode::End)
                        .max_width_chars(30)
                        .build(),
                ));
            }
        });
        factory.connect_bind(|_, item| {
            if let Some(item) = item.downcast_ref::<gtk4::ListItem>() {
                if let (Some(value), Some(label)) = (
                    item.item().and_downcast::<gtk4::StringObject>(),
                    item.child().and_downcast::<Label>(),
                ) {
                    label.set_label(&value.string());
                    label.set_tooltip_text(Some(&value.string()));
                }
            }
        });
        dropdown.set_factory(Some(&factory));
        dropdown.set_hexpand(true);
        dropdown.set_width_request(230);

        root.append(&caption);
        root.append(&dropdown);

        Self {
            root,
            dropdown,
            model,
            updating: Rc::new(Cell::new(false)),
        }
    }

    /// Dispatch `to_command` whenever the user picks a different entry.
    ///
    /// The guard prevents model updates originating in OBS from echoing back
    /// as user selection commands.
    fn connect_selection(&self, nav: &NavigationContext, to_command: fn(String) -> AppCommand) {
        let nav = nav.clone();
        let model = self.model.clone();
        let updating = self.updating.clone();
        self.dropdown.connect_selected_notify(move |dropdown| {
            if updating.get() {
                return;
            }
            if let Some(name) = model.string(dropdown.selected()) {
                nav.dispatch(to_command(name.to_string()));
            }
        });
    }

    fn update(&self, list: &ObsNamedList) {
        self.updating.set(true);

        let same_items = self.model.n_items() as usize == list.items.len()
            && list.items.iter().enumerate().all(|(index, item)| {
                self.model.string(index as u32).as_deref() == Some(item.as_str())
            });
        if !same_items {
            let additions: Vec<&str> = list.items.iter().map(String::as_str).collect();
            self.model.splice(0, self.model.n_items(), &additions);
        }

        let selected = list
            .current_index()
            .map(|idx| idx as u32)
            .unwrap_or(gtk4::INVALID_LIST_POSITION);

        let has_items = list.has_items();
        self.root.set_visible(has_items);
        self.dropdown.set_sensitive(has_items);
        self.dropdown.set_selected(selected);
        self.updating.set(false);
    }
}
#[derive(Clone)]
pub(super) struct SidebarControls {
    status_label: Label,
    live_icon: Image,
    connect_btn: Button,
}

impl SidebarControls {
    /// Re-applies connection status and button wording; the button names
    /// only the outputs a disconnect would stop, so output changes call this too.
    pub(super) fn apply_connection(&self, state: &AppState) {
        let model = sidebar_connection_model(
            &state.obs_status,
            &state.stream_status,
            &state.record_status,
        );
        self.status_label.set_text(&model.status_text);
        set_status_class(&self.status_label, model.css_class);
        apply_button_model(&self.connect_btn, model.button);
    }

    pub(super) fn sync_output_buttons(&self, state: &AppState) {
        self.set_streaming(state.stream_status.active);
        self.apply_connection(state);
    }

    fn set_streaming(&self, streaming: bool) {
        if streaming {
            self.live_icon
                .add_css_class("scenedeck-sidebar-live-icon-streaming");
        } else {
            self.live_icon
                .remove_css_class("scenedeck-sidebar-live-icon-streaming");
        }
    }
}

pub(super) fn build(nav: &NavigationContext) -> (adw::NavigationPage, ListBox, SidebarControls) {
    let list = ListBox::builder()
        .selection_mode(SelectionMode::Single)
        .vexpand(true)
        .build();
    list.add_css_class("navigation-sidebar");
    list.add_css_class("scenedeck-sidebar-list");

    // Output events tint this icon while streaming, so keep its handle before
    // constructing the rows instead of recovering it from the widget tree.
    let live_icon = Image::from_icon_name(Page::Live.icon_name());
    live_icon.add_css_class("scenedeck-sidebar-live-icon");

    for page in NAV_PAGES {
        let icon = if page == Page::Live {
            live_icon.clone()
        } else {
            Image::from_icon_name(page.icon_name())
        };
        let row = adw::ActionRow::builder()
            .title(page.title())
            .activatable(true)
            .build();
        row.add_prefix(&icon);
        list.append(&row);
    }

    if let Some(row) = list.row_at_index(0) {
        list.select_row(Some(&row));
    }

    let status_label = Label::builder()
        .label(ObsStatus::Disconnected.label())
        .xalign(0.0)
        .wrap(true)
        .build();
    status_label.add_css_class("obs-disconnected");

    let connect_btn = sidebar_button(
        SIDEBAR_CONNECT_ICON,
        &fl!(LANGUAGE_LOADER, "window-connect-btn-connect"),
        "suggested-action",
        true,
        {
            let nav = nav.clone();
            move |button: &Button| {
                let status = nav.state.borrow().obs_status.clone();
                match status {
                    ObsStatus::Disconnected | ObsStatus::Error(_) => {
                        nav.dispatch(AppCommand::Connect);
                    }
                    ObsStatus::Connected { .. } => {
                        live::handle_disconnect(button, &nav);
                    }
                    ObsStatus::Connecting => nav.dispatch(AppCommand::Disconnect),
                }
            }
        },
    );

    let footer = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(8)
        .margin_top(12)
        .margin_bottom(12)
        .margin_start(12)
        .margin_end(12)
        .build();
    footer.add_css_class("sidebar-obs-footer");
    footer.append(&status_label);
    footer.append(&connect_btn);

    let sidebar_content = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .vexpand(true)
        .hexpand(true)
        .build();
    sidebar_content.add_css_class("scenedeck-sidebar");
    let navigation_scroll = gtk4::ScrolledWindow::builder()
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .vscrollbar_policy(gtk4::PolicyType::Automatic)
        .min_content_height(48)
        .vexpand(true)
        .child(&list)
        .build();
    sidebar_content.append(&navigation_scroll);
    sidebar_content.append(&footer);

    let sidebar_header = adw::HeaderBar::builder().show_title(false).build();
    sidebar_header.add_css_class("scenedeck-sidebar-header");
    sidebar_header.pack_start(&build_brand());
    let sidebar_toolbar = adw::ToolbarView::new();
    sidebar_toolbar.add_css_class("scenedeck-sidebar-toolbar");
    sidebar_toolbar.add_top_bar(&sidebar_header);
    sidebar_toolbar.set_content(Some(&sidebar_content));

    let nav_page = adw::NavigationPage::builder()
        .title(APP_NAME)
        .child(&sidebar_toolbar)
        .build();

    (
        nav_page,
        list,
        SidebarControls {
            status_label,
            live_icon,
            connect_btn,
        },
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SidebarButtonModel {
    label: String,
    sensitive: bool,
    suggested: bool,
    destructive: bool,
}

struct SidebarConnectionModel {
    status_text: String,
    css_class: &'static str,
    button: SidebarButtonModel,
}

fn sidebar_connection_model(
    status: &ObsStatus,
    stream: &OutputStatus,
    record: &OutputStatus,
) -> SidebarConnectionModel {
    let (status_text, label, sensitive, suggested, destructive) = match status {
        ObsStatus::Connecting => (
            fl!(LANGUAGE_LOADER, "window-status-connecting"),
            fl!(LANGUAGE_LOADER, "window-connect-btn-connecting"),
            false,
            false,
            false,
        ),
        ObsStatus::Connected { obs_version } => (
            fl!(
                LANGUAGE_LOADER,
                "window-status-connected",
                version = obs_version.clone()
            ),
            live::disconnect_label(stream, record),
            true,
            false,
            live::disconnect_stops_outputs(stream, record),
        ),
        ObsStatus::Disconnected => (
            fl!(LANGUAGE_LOADER, "window-status-disconnected"),
            fl!(LANGUAGE_LOADER, "window-connect-btn-connect"),
            true,
            true,
            false,
        ),
        ObsStatus::Error(error) => (
            fl!(
                LANGUAGE_LOADER,
                "window-status-error",
                error = error.clone()
            ),
            fl!(LANGUAGE_LOADER, "window-connect-btn-retry"),
            true,
            true,
            false,
        ),
    };

    SidebarConnectionModel {
        status_text,
        css_class: status.css_class(),
        button: SidebarButtonModel {
            label,
            sensitive,
            suggested,
            destructive,
        },
    }
}

fn apply_button_model(button: &Button, model: SidebarButtonModel) {
    set_button_label(button, &model.label);
    button.set_sensitive(model.sensitive);
    if model.suggested {
        button.add_css_class("suggested-action");
    } else {
        button.remove_css_class("suggested-action");
    }
    if model.destructive {
        button.add_css_class("destructive-action");
    } else {
        button.remove_css_class("destructive-action");
    }
}

fn set_status_class(label: &Label, new_class: &str) {
    for class in status_bar::CONNECTION_CSS_CLASSES {
        label.remove_css_class(class);
    }
    label.add_css_class(new_class);
}

const SIDEBAR_CONNECT_ICON: &str = "nf-md-lan-connect-symbolic";

fn button_content(icon_name: &str, label: &str) -> GtkBox {
    let content = GtkBox::builder()
        .orientation(Orientation::Horizontal)
        .spacing(8)
        .halign(gtk4::Align::Center)
        .build();
    content.append(&Image::from_icon_name(icon_name));
    let label = Label::new(Some(label));
    label.set_wrap(true);
    label.set_justify(gtk4::Justification::Center);
    label.add_css_class("scenedeck-button-label");
    content.append(&label);
    content
}

fn set_button_label(button: &Button, text: &str) {
    let Some(label) = button
        .child()
        .and_then(|content| content.last_child())
        .and_then(|child| child.downcast::<Label>().ok())
    else {
        button.set_label(text);
        return;
    };
    label.set_text(text);
}

fn build_brand() -> GtkBox {
    let brand = GtkBox::builder()
        .orientation(Orientation::Horizontal)
        .spacing(8)
        .valign(gtk4::Align::Center)
        .build();
    brand.add_css_class("scenedeck-brand");

    let logo = Image::from_icon_name(APP_ID);
    logo.set_pixel_size(22);
    logo.add_css_class("scenedeck-brand-logo");

    let name = Label::builder().label(APP_NAME).xalign(0.0).build();
    name.add_css_class("scenedeck-brand-name");

    brand.append(&logo);
    brand.append(&name);
    brand
}

fn sidebar_button(
    icon_name: &str,
    label: &str,
    css_class: &str,
    sensitive: bool,
    on_click: impl Fn(&Button) + 'static,
) -> Button {
    let button = Button::builder()
        .halign(gtk4::Align::Fill)
        .hexpand(true)
        .sensitive(sensitive)
        .build();
    button.set_child(Some(&button_content(icon_name, label)));
    button.add_css_class(css_class);
    button.connect_clicked(move |button| on_click(button));
    button
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::output::OutputRunState;

    #[test]
    #[ignore = "requires a disposable GTK display; no OBS or keyring access"]
    fn native_selector_readbacks_retain_model_rows() {
        crate::ui::test_support::native_test(
            "ui::sidebar::tests::native_selector_readbacks_retain_model_rows",
            || {
                let selector = NamedSelector::build("Fixture", "Fixture");
                let list = ObsNamedList {
                    items: vec!["First".into(), "Other".into()],
                    current: Some("First".into()),
                };
                selector.update(&list);
                let first = selector.model.item(0).unwrap();
                let changes = Rc::new(Cell::new(0));
                selector.model.connect_items_changed({
                    let changes = changes.clone();
                    move |_, _, _, _| changes.set(changes.get() + 1)
                });
                selector.update(&list);
                assert_eq!(
                    changes.get(),
                    0,
                    "identical readback must not splice the model"
                );
                selector.dropdown.set_selected(1);
                selector.update(&list);
                assert_eq!(
                    selector.dropdown.selected(),
                    0,
                    "readback must correct an optimistic selection"
                );
                assert_eq!(selector.model.item(0).unwrap(), first);
                assert_eq!(changes.get(), 0);
                selector.update(&ObsNamedList {
                    current: Some("Other".into()),
                    ..list
                });
                assert_eq!(selector.dropdown.selected(), 1);
                assert_eq!(changes.get(), 0);
                selector.update(&ObsNamedList {
                    items: vec!["New".into()],
                    current: Some("New".into()),
                });
                assert_eq!(changes.get(), 1);
                assert_eq!(selector.model.string(0).as_deref(), Some("New"));
                selector.update(&ObsNamedList::default());
                assert_eq!(changes.get(), 2);
                assert!(!selector.root.is_visible());
            },
        );
    }

    #[test]
    #[ignore = "requires a disposable native GTK display"]
    fn native_obs_setup_popover_follows_the_selected_palette() {
        crate::ui::test_support::native_test(
            "ui::sidebar::tests::native_obs_setup_popover_follows_the_selected_palette",
            || {
                use crate::domain::appearance::{MotionLevel, ThemeId, ThemeMode, ThemePreference};
                use crate::ui::theme::ThemeManager;

                let (nav, _stack, _runtime, _events) = crate::ui::test_support::navigation();
                let selectors = HeaderSelectors::build(&nav);
                let names = ObsNamedList {
                    items: vec!["Untitled".into(), "Other".into()],
                    current: Some("Untitled".into()),
                };
                selectors.update_profiles(&names);
                selectors.update_scene_collections(&names);
                let header = adw::HeaderBar::new();
                selectors.pack_into(&header);
                let root = GtkBox::new(Orientation::Vertical, 0);
                root.append(&header);
                root.append(&Label::new(Some("Settings")));
                let window = gtk4::Window::builder()
                    .default_width(600)
                    .default_height(400)
                    .child(&root)
                    .build();
                window.add_css_class("scenedeck-root");
                window.present();
                let popover = selectors.menu.popover().unwrap();
                for (mode, expected) in [
                    (ThemeMode::Dark, [46_u8, 52, 64]),
                    (ThemeMode::Light, [246, 248, 251]),
                    (ThemeMode::Dark, [46, 52, 64]),
                ] {
                    let preference = ThemePreference {
                        selected_theme: Some(ThemeId::new("nord")),
                        mode,
                        motion: MotionLevel::Off,
                        ..Default::default()
                    };
                    assert!(ThemeManager::apply(&preference).is_ok());
                    // Theme swap drops allocation. Popup on a 0x0 window: no monitor, garbage size, BadAlloc.
                    assert!(
                        crate::ui::test_support::pump_until(
                            std::time::Duration::from_secs(5),
                            || { window.width() > 0 && selectors.menu.width() > 0 }
                        ),
                        "setup button never got allocated"
                    );
                    selectors.menu.popup();
                    crate::ui::test_support::pump(std::time::Duration::from_millis(100));
                    assert!(popover.is_mapped());
                    let contents = popover.first_child().expect("native popover contents");
                    assert_eq!(contents.css_name(), "contents");
                    let point = contents
                        .compute_point(
                            &popover,
                            &gtk4::graphene::Point::new(
                                contents.width() as f32 / 2.0,
                                (contents.height() - 8) as f32,
                            ),
                        )
                        .unwrap();
                    let snapshot = gtk4::Snapshot::new();
                    gtk4::WidgetPaintable::new(Some(&popover)).snapshot(
                        &snapshot,
                        f64::from(popover.width()),
                        f64::from(popover.height()),
                    );
                    let node = snapshot.to_node().expect("popover render node");
                    let texture = popover
                        .native()
                        .unwrap()
                        .renderer()
                        .unwrap()
                        .render_texture(
                            &node,
                            Some(&gtk4::graphene::Rect::new(
                                0.0,
                                0.0,
                                popover.width() as f32,
                                popover.height() as f32,
                            )),
                        );
                    let mut downloader = gtk4::gdk::TextureDownloader::new(&texture);
                    downloader.set_format(gtk4::gdk::MemoryFormat::R8g8b8a8);
                    let (pixels, stride) = downloader.download_bytes();
                    let x = (point.x() * texture.width() as f32 / popover.width() as f32) as usize;
                    let y =
                        (point.y() * texture.height() as f32 / popover.height() as f32) as usize;
                    let offset = y * stride + x * 4;
                    if let Some(directory) = std::env::var_os("SCENEDECK_THEME_CAPTURE_DIR") {
                        let path = std::path::PathBuf::from(directory)
                            .join(format!("obs-setup-{}.png", mode.as_str()));
                        texture.save_to_png(path).unwrap();
                    }
                    let actual = &pixels[offset..offset + 3];
                    assert!(
                        actual.iter().zip(expected).all(|(a, e)| a.abs_diff(e) <= 1),
                        "{mode:?} setup popover painted {actual:?}, expected theme card {expected:?}"
                    );
                }
                popover.popdown();
                window.close();
            },
        );
    }

    #[test]
    fn connection_model_covers_every_state() {
        let idle = OutputStatus::inactive();
        let live_out = OutputStatus::new(true, OutputRunState::Active);
        let connecting = sidebar_connection_model(&ObsStatus::Connecting, &idle, &idle);
        assert_eq!(connecting.css_class, "obs-connecting");
        assert!(!connecting.button.sensitive);
        assert!(!connecting.button.suggested);
        assert!(!connecting.button.destructive);

        let connected = sidebar_connection_model(
            &ObsStatus::Connected {
                obs_version: "30.1.2".to_string(),
            },
            &live_out,
            &idle,
        );
        assert_eq!(connected.css_class, "obs-connected");
        assert!(connected.button.sensitive);
        assert!(connected.button.destructive);
        assert!(!connected.button.suggested);
        assert!(connected.status_text.contains("30.1.2"));

        let disconnected = sidebar_connection_model(&ObsStatus::Disconnected, &idle, &idle);
        assert_eq!(disconnected.css_class, "obs-disconnected");
        assert!(disconnected.button.sensitive);
        assert!(disconnected.button.suggested);
        assert!(!disconnected.button.destructive);

        let failed =
            sidebar_connection_model(&ObsStatus::Error("refused".to_string()), &idle, &idle);
        assert_eq!(failed.css_class, "obs-error");
        assert!(failed.button.sensitive);
        assert!(failed.button.suggested);
        assert!(!failed.button.destructive);
        assert!(failed.status_text.contains("refused"));
    }

    #[test]
    fn connected_button_names_only_running_outputs() {
        let connected = ObsStatus::Connected {
            obs_version: "32.2.2".to_string(),
        };
        let idle = OutputStatus::inactive();
        let running = OutputStatus::new(true, OutputRunState::Active);
        let starting = OutputStatus::new(false, OutputRunState::Starting);
        let button = |stream: &OutputStatus, record: &OutputStatus| {
            sidebar_connection_model(&connected, stream, record).button
        };

        let plain = button(&idle, &idle);
        assert_eq!(
            plain.label,
            fl!(LANGUAGE_LOADER, "window-connect-btn-disconnect")
        );
        assert!(!plain.destructive);

        for (stream, record, key) in [
            (&running, &idle, "window-stop-stream-disconnect-label"),
            (&starting, &idle, "window-stop-stream-disconnect-label"),
            (&idle, &running, "window-stop-record-disconnect-label"),
            (&running, &running, "window-stop-disconnect-label"),
        ] {
            let model = button(stream, record);
            assert_eq!(model.label, LANGUAGE_LOADER.get(key));
            assert!(model.destructive);
        }
    }

    #[test]
    fn every_connection_css_class_is_cleared_before_replacement() {
        for status in [
            ObsStatus::Connecting,
            ObsStatus::Connected {
                obs_version: String::new(),
            },
            ObsStatus::Disconnected,
            ObsStatus::Error(String::new()),
        ] {
            assert!(
                status_bar::CONNECTION_CSS_CLASSES.contains(&status.css_class()),
                "{:?} uses a class the widgets never clear",
                status
            );
        }
    }
}
