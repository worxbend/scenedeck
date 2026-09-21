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
use crate::domain::output::{OutputRunState, OutputStatus};
use crate::infra::i18n::LANGUAGE_LOADER;
use crate::ui::navigation::NavigationContext;
use crate::ui::widgets::status_bar;

pub(super) const NAV_PAGES: [Page; 8] = [
    Page::Live,
    Page::Mixer,
    Page::Graph,
    Page::Inventory,
    Page::Doctor,
    Page::Settings,
    Page::Help,
    Page::Stats,
];

#[derive(Clone)]
pub(super) struct HeaderSelectors {
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

        Self {
            profiles,
            scene_collections,
        }
    }

    pub(super) fn pack_into(&self, header: &adw::HeaderBar) {
        header.pack_start(&self.scene_collections.root);
        header.pack_start(&self.profiles.root);
    }

    pub(super) fn update_profiles(&self, profiles: &ObsNamedList) {
        self.profiles.update(profiles);
    }

    pub(super) fn update_scene_collections(&self, collections: &ObsNamedList) {
        self.scene_collections.update(collections);
    }

    pub(super) fn clear(&self) {
        self.profiles.update(&ObsNamedList::default());
        self.scene_collections.update(&ObsNamedList::default());
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
            .orientation(Orientation::Horizontal)
            .spacing(6)
            .valign(gtk4::Align::Center)
            .build();
        root.set_visible(false);
        root.add_css_class("header-selector");

        let caption = Label::builder()
            .label(label)
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
        dropdown.set_width_request(170);

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

        let additions: Vec<&str> = list.items.iter().map(String::as_str).collect();
        self.model.splice(0, self.model.n_items(), &additions);

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
    stream_btn: Button,
    record_btn: Button,
    connect_btn: Button,
}

impl SidebarControls {
    pub(super) fn apply_connection(&self, status: &ObsStatus) {
        let model = sidebar_connection_model(status);
        self.status_label.set_text(&model.status_text);
        set_status_class(&self.status_label, model.css_class);
        apply_button_model(&self.connect_btn, model.button);
    }

    pub(super) fn sync_output_buttons(&self, state: &AppState) {
        let connected = matches!(state.obs_status, ObsStatus::Connected { .. });
        apply_button_model(
            &self.stream_btn,
            sidebar_output_button_model(
                &state.stream_status,
                connected,
                fl!(LANGUAGE_LOADER, "window-sidebar-start-stream"),
                fl!(LANGUAGE_LOADER, "window-sidebar-stop-stream"),
            ),
        );
        apply_button_model(
            &self.record_btn,
            sidebar_output_button_model(
                &state.record_status,
                connected,
                fl!(LANGUAGE_LOADER, "window-sidebar-start-recording"),
                fl!(LANGUAGE_LOADER, "window-sidebar-stop-recording"),
            ),
        );
        self.set_streaming(state.stream_status.active);
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
            move |_: &Button| {
                let status = nav.state.borrow().obs_status.clone();
                match status {
                    ObsStatus::Disconnected | ObsStatus::Error(_) => {
                        nav.dispatch(AppCommand::Connect);
                    }
                    ObsStatus::Connected { .. } | ObsStatus::Connecting => {
                        nav.dispatch(AppCommand::Disconnect);
                    }
                }
            }
        },
    );

    let stream_btn = sidebar_button(
        SIDEBAR_STREAM_ICON,
        &fl!(LANGUAGE_LOADER, "window-sidebar-start-stream"),
        "sidebar-output-button",
        false,
        {
            let nav = nav.clone();
            move |button: &Button| crate::ui::pages::live::handle_stream_output_toggle(button, &nav)
        },
    );

    let record_btn = sidebar_button(
        SIDEBAR_RECORD_ICON,
        &fl!(LANGUAGE_LOADER, "window-sidebar-start-recording"),
        "sidebar-output-button",
        false,
        {
            let nav = nav.clone();
            move |button: &Button| crate::ui::pages::live::handle_record_output_toggle(button, &nav)
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
    footer.append(&stream_btn);
    footer.append(&record_btn);
    footer.append(&connect_btn);

    let sidebar_content = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .vexpand(true)
        .hexpand(true)
        .build();
    sidebar_content.add_css_class("scenedeck-sidebar");
    sidebar_content.append(&list);
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
            stream_btn,
            record_btn,
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

fn sidebar_output_button_model(
    status: &OutputStatus,
    connected: bool,
    start_label: String,
    stop_label: String,
) -> SidebarButtonModel {
    if status.state.is_transitioning() {
        return SidebarButtonModel {
            label: match status.state {
                OutputRunState::Starting => fl!(LANGUAGE_LOADER, "window-sidebar-output-starting"),
                OutputRunState::Stopping => fl!(LANGUAGE_LOADER, "window-sidebar-output-stopping"),
                OutputRunState::Reconnecting => {
                    fl!(LANGUAGE_LOADER, "window-sidebar-output-reconnecting")
                }
                _ => fl!(LANGUAGE_LOADER, "window-sidebar-output-working"),
            },
            sensitive: false,
            suggested: false,
            destructive: status.active,
        };
    }

    if status.active {
        SidebarButtonModel {
            label: stop_label,
            sensitive: connected,
            suggested: false,
            destructive: connected,
        }
    } else {
        SidebarButtonModel {
            label: start_label,
            sensitive: connected,
            suggested: connected,
            destructive: false,
        }
    }
}

struct SidebarConnectionModel {
    status_text: String,
    css_class: &'static str,
    button: SidebarButtonModel,
}

fn sidebar_connection_model(status: &ObsStatus) -> SidebarConnectionModel {
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
            fl!(LANGUAGE_LOADER, "window-connect-btn-disconnect"),
            true,
            false,
            true,
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

const SIDEBAR_STREAM_ICON: &str = "nf-md-broadcast-symbolic";
const SIDEBAR_RECORD_ICON: &str = "nf-md-record-circle-symbolic";
const SIDEBAR_CONNECT_ICON: &str = "nf-md-lan-connect-symbolic";

fn button_content(icon_name: &str, label: &str) -> GtkBox {
    let content = GtkBox::builder()
        .orientation(Orientation::Horizontal)
        .spacing(8)
        .halign(gtk4::Align::Center)
        .build();
    content.append(&Image::from_icon_name(icon_name));
    let label = Label::new(Some(label));
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

    fn output_status(active: bool, state: OutputRunState) -> OutputStatus {
        OutputStatus {
            active,
            state,
            detail: None,
        }
    }

    #[test]
    fn connection_model_covers_every_state() {
        let connecting = sidebar_connection_model(&ObsStatus::Connecting);
        assert_eq!(connecting.css_class, "obs-connecting");
        assert!(!connecting.button.sensitive);
        assert!(!connecting.button.suggested);
        assert!(!connecting.button.destructive);

        let connected = sidebar_connection_model(&ObsStatus::Connected {
            obs_version: "30.1.2".to_string(),
        });
        assert_eq!(connected.css_class, "obs-connected");
        assert!(connected.button.sensitive);
        assert!(connected.button.destructive);
        assert!(!connected.button.suggested);
        assert!(connected.status_text.contains("30.1.2"));

        let disconnected = sidebar_connection_model(&ObsStatus::Disconnected);
        assert_eq!(disconnected.css_class, "obs-disconnected");
        assert!(disconnected.button.sensitive);
        assert!(disconnected.button.suggested);
        assert!(!disconnected.button.destructive);

        let failed = sidebar_connection_model(&ObsStatus::Error("refused".to_string()));
        assert_eq!(failed.css_class, "obs-error");
        assert!(failed.button.sensitive);
        assert!(failed.button.suggested);
        assert!(!failed.button.destructive);
        assert!(failed.status_text.contains("refused"));
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

    #[test]
    fn output_button_model_reflects_connection_and_output_state() {
        assert_eq!(
            sidebar_output_button_model(
                &output_status(false, OutputRunState::Inactive),
                false,
                "Start Stream".to_string(),
                "Stop Stream".to_string(),
            ),
            SidebarButtonModel {
                label: "Start Stream".to_string(),
                sensitive: false,
                suggested: false,
                destructive: false,
            }
        );
        assert_eq!(
            sidebar_output_button_model(
                &output_status(false, OutputRunState::Inactive),
                true,
                "Start Stream".to_string(),
                "Stop Stream".to_string(),
            ),
            SidebarButtonModel {
                label: "Start Stream".to_string(),
                sensitive: true,
                suggested: true,
                destructive: false,
            }
        );
        assert_eq!(
            sidebar_output_button_model(
                &output_status(true, OutputRunState::Active),
                true,
                "Start Stream".to_string(),
                "Stop Stream".to_string(),
            ),
            SidebarButtonModel {
                label: "Stop Stream".to_string(),
                sensitive: true,
                suggested: false,
                destructive: true,
            }
        );
        assert_eq!(
            sidebar_output_button_model(
                &output_status(false, OutputRunState::Starting),
                true,
                "Start Stream".to_string(),
                "Stop Stream".to_string(),
            ),
            SidebarButtonModel {
                label: "Starting…".to_string(),
                sensitive: false,
                suggested: false,
                destructive: false,
            }
        );
    }
}
