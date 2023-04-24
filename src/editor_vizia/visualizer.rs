use std::sync::Arc;

use nih_plug_vizia::vizia::{prelude::*, vg};

use crate::visualizer::{Visualizer, BUFFER_SIZE};

pub struct VisualizerView {
    pub visualizer: Arc<Visualizer>,
}

impl VisualizerView {
    pub fn new<L>(cx: &mut Context, visualizer: L) -> Handle<Self>
    where
        L: Lens<Target = Arc<Visualizer>>,
    {
        Self {
            visualizer: visualizer.get(cx),
        }
        .build(cx, |_cx| {})
    }
}

impl View for VisualizerView {
    fn element(&self) -> Option<&'static str> {
        Some("visualizer")
    }

    fn draw(&self, cx: &mut DrawContext, canvas: &mut Canvas) {
        let bounds = cx.bounds();
        if bounds.w == 0.0 || bounds.h == 0.0 {
            return;
        }

        let line_width = cx.style.dpi_factor as f32 * 1.5;
        let paint = vg::Paint::color(cx.font_color().cloned().unwrap_or_default().into())
            .with_line_width(line_width);

        let mut path = vg::Path::new();

        for i in 0..BUFFER_SIZE {
            let x = bounds.x + (bounds.w * (i as f32 / BUFFER_SIZE as f32));
            path.move_to(
                x,
                bounds.y + (bounds.h * (1.0 - self.visualizer.get(i).clamp(0.0, 1.0))),
            );

            path.line_to(x, bounds.y + bounds.h);
        }

        canvas.stroke_path(&mut path, &paint);
    }
}
