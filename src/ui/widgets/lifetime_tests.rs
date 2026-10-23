//! Native regressions exercise real constructors and retained public handles.

use super::{audio_card, icon_picker, operating_panel, studio_controls, transport_bar};
use crate::domain::{audio::AudioInput, scene::Scene};
use crate::ui::{pages, test_support};
use gtk4::prelude::*;
use std::time::Duration;

fn weak_tree(widget: &impl IsA<gtk4::Widget>, refs: &mut Vec<glib::WeakRef<gtk4::Widget>>) {
    let widget = widget.upcast_ref::<gtk4::Widget>();
    refs.push(widget.downgrade());
    let mut child = widget.first_child();
    while let Some(widget) = child {
        child = widget.next_sibling();
        weak_tree(&widget, refs);
    }
}

fn assert_released(refs: Vec<glib::WeakRef<gtk4::Widget>>) {
    test_support::pump(Duration::from_millis(350));
    let retained: Vec<_> = refs
        .iter()
        .filter_map(glib::WeakRef::upgrade)
        .map(|widget| format!("{}:{}", widget.type_().name(), widget.widget_name()))
        .collect();
    assert!(
        retained.is_empty(),
        "detached widgets retained: {retained:?}"
    );
}

#[test]
#[ignore = "requires a native GTK display"]
fn native_repeated_audio_cards_and_icon_pickers_release_all_widgets() {
    test_support::native_test("ui::widgets::lifetime_tests::native_repeated_audio_cards_and_icon_pickers_release_all_widgets", || {
        let (nav, stack, _runtime, _events) = test_support::navigation();
        let mut refs = Vec::new();
        let input = AudioInput::new("Mic".into(), false, 1.0, 0.0);
        for _ in 0..128 {
            let card = audio_card::build(&input, nav.clone());
            weak_tree(&card.root, &mut refs);
            drop(card);
            let compact = audio_card::build_compact(&input, nav.clone(), true);
            weak_tree(&compact.root, &mut refs);
            drop(compact);
            let picker = icon_picker::build(None, "fixture", icon_picker::PickerDisplay::CurrentIcon, |_| {});
            weak_tree(&picker, &mut refs);
            drop(picker);
        }
        drop(nav);
        drop(stack);
        assert_released(refs);
    });
}

#[test]
#[ignore = "requires a native GTK display"]
fn native_owned_pages_release_roots_handles_and_timer_subtrees() {
    test_support::native_test(
        "ui::widgets::lifetime_tests::native_owned_pages_release_roots_handles_and_timer_subtrees",
        || {
            let (nav, stack, _runtime, _events) = test_support::navigation();
            nav.state.borrow_mut().scene_inventory.scenes = vec![Scene { id: "Main".into() }];
            nav.state.borrow_mut().scene_inventory.current_id = Some("Main".into());
            nav.state
                .borrow_mut()
                .set_topology_ready(Default::default());
            let mut refs = Vec::new();
            for _ in 0..8 {
                let (graph, refresh) = pages::graph::build(nav.clone());
                stack.add_named(&graph, Some("graph-fixture"));
                weak_tree(&graph, &mut refs);
                refresh();
                weak_tree(&graph, &mut refs);
                stack.remove(&graph);
                drop((graph, refresh));
                let (doctor, refresh) = pages::doctor::build(nav.clone());
                weak_tree(&doctor, &mut refs);
                drop((doctor, refresh));
                let (sources, refresh) = operating_panel::build(nav.clone());
                weak_tree(&sources, &mut refs);
                drop((sources, refresh));
                let (stats, refresh, popout) = pages::stats::build(nav.clone());
                weak_tree(&stats, &mut refs);
                refresh();
                drop((stats, refresh, popout));
                let live = pages::live::build(nav.clone());
                stack.add_named(&live.root, Some("live-fixture"));
                weak_tree(&live.root, &mut refs);
                stack.remove(&live.root);
                drop(live);
                for widget in [
                    transport_bar::build(nav.clone()),
                    studio_controls::toggle(nav.clone()).upcast(),
                    studio_controls::transition_row(nav.clone()).upcast(),
                ] {
                    weak_tree(&widget, &mut refs);
                    drop(widget);
                }
            }
            // Leave a page parented while tearing down the actual navigation owner.
            let live = pages::live::build(nav.clone());
            stack.add_named(&live.root, Some("teardown-fixture"));
            weak_tree(&stack, &mut refs);
            drop(live);
            drop(nav);
            drop(stack);
            assert_released(refs);
        },
    );
}

#[test]
#[ignore = "requires a native GTK display"]
fn native_external_row_names_remain_literal_without_markup_or_links() {
    test_support::native_test("ui::widgets::lifetime_tests::native_external_row_names_remain_literal_without_markup_or_links", || {
        let (nav, _stack, _runtime, _events) = test_support::navigation();
        let name = "Q&A <live> \"quote\" <b>bold</b> <a href=\"https://example.invalid\">link</a>";
        {
            let mut state = nav.state.borrow_mut();
            state.scene_inventory.scenes = vec![Scene { id: name.into() }, Scene { id: "Child <i>&</i>".into() }];
            state.scene_inventory.current_id = Some(name.into());
            state.scene_graph.edges.insert(name.into(), vec!["Child <i>&</i>".into()]);
            let graph = state.scene_graph.clone();
            state.set_topology_ready(graph);
        }
        let (graph, _graph_refresh) = pages::graph::build(nav.clone());
        let (doctor, _doctor_refresh) = pages::doctor::build(nav.clone());
        let (mixer, _mixer_refresh, _mixer_handle) = pages::mixer::build(nav);
        fn expand_relationships(widget: &gtk4::Widget) {
            if let Some(expander) = widget.downcast_ref::<gtk4::Expander>() {
                expander.set_expanded(true);
            }
            let mut child = widget.first_child();
            while let Some(current) = child {
                expand_relationships(&current);
                child = current.next_sibling();
            }
        }
        expand_relationships(&graph);
        for (page_name, page) in [("Graph", &graph), ("Doctor", &doctor), ("Mixer", &mixer)] {
            let mut refs = Vec::new();
            weak_tree(page, &mut refs);
            let widgets: Vec<_> = refs.iter().filter_map(glib::WeakRef::upgrade).collect();
            let rows: Vec<_> = widgets.iter().filter_map(|widget| widget.downcast_ref::<adw::PreferencesRow>()).collect();
            assert!(!rows.is_empty(), "{page_name} exposes no preference rows");
            assert!(rows.iter().all(|row| !row.property::<bool>("use-markup")));
            assert!(widgets.iter().filter_map(|widget| widget.downcast_ref::<gtk4::Label>())
                .any(|label| label.text().contains(name)), "literal external name missing from rendered labels");
        }
    });
}
