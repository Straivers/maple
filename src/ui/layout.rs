//! Code for defining and calculating layouts.
//!
//! All units are held in device-independent and DPI-scaled pixels.

use crate::{
    gfx::{Canvas, Color, Draw, DrawStyled},
    px::Px,
    shapes::Rect,
};

use super::{
    tree::{Index, Tree},
    widgets::{Widget, WidgetTree},
};

#[derive(Clone, Debug)]
pub struct Region {
    rect: Rect,
    color: Color,
}

impl Draw<Region> for Canvas {
    fn draw(&mut self, shape: &Region) {
        self.draw_styled(&shape.rect, shape.color);
    }
}

pub type LayoutTree = Tree<Region>;

pub fn compute_layout(
    widgets: &WidgetTree,
    root: Index<Widget>,
    area: Rect,
    output: &mut LayoutTree,
) -> Index<Region> {
    let widget = widgets.get(root);
    let children = widgets.children(root);

    match widget {
        Widget::Panel(panel) => {
            let max_area = Rect::new(
                area.x(),
                area.y(),
                panel.max_extent.width.min(area.width()),
                panel.max_extent.height.min(area.height()),
            );

            let mut child_regions = vec![];

            let mut max_child_width = Px(0);

            let mut child_area = Rect::new(
                max_area.x() + panel.margin,
                max_area.y() + panel.margin,
                max_area.width() - 2 * panel.margin,
                max_area.height() - 2 * panel.margin,
            );

            for child in children {
                let region = compute_layout(widgets, *child, child_area, output);

                let r = output.get(region);
                *child_area.y_mut() += r.rect.height() + panel.margin;
                *child_area.height_mut() -= r.rect.height() + panel.margin;

                if max_child_width < r.rect.width() {
                    max_child_width = r.rect.width();
                }

                child_regions.push(region);
            }

            assert!(child_area.height() <= max_area.height());

            let panel_rect = Rect::new(
                area.x(),
                area.y(),
                panel
                    .min_extent
                    .width
                    .max(max_child_width + 2 * panel.margin),
                panel.min_extent.height.max(child_area.y() - area.y()),
            );

            output
                .add(
                    &Region {
                        rect: panel_rect,
                        color: panel.color,
                    },
                    &child_regions,
                )
                .unwrap()
        }
    }
}
