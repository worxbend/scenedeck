//! Focused OBS scene-item inspection, independent from Program selection.

use std::rc::Rc;

use crate::ui::navigation::NavigationContext;

pub(crate) fn build(nav: NavigationContext) -> (gtk4::Widget, Rc<dyn Fn()>) {
    crate::ui::widgets::operating_panel::build(nav)
}
