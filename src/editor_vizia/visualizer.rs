use std::sync::Arc;

use nih_plug_vizia::vizia::{prelude::*, vg};

use std::sync::atomic::{AtomicUsize, Ordering};

use nih_plug::prelude::AtomicF32;

pub const BUFFER_SIZE: usize = 512;
pub struct Visualizer {
    pub visualizer: Arc<VisualizerData>,
}

// works like a ring buffer to avoid having to shift all the contents
pub struct VisualizerData {
    pub data: [AtomicF32; BUFFER_SIZE],
    pub current_index: AtomicUsize,
}

impl VisualizerData {
    pub fn new() -> Self {
        Self {
            data: std::array::from_fn(|_| AtomicF32::new(0.0)),
            current_index: AtomicUsize::new(0),
        }
    }

    pub fn store(&self, value: f32) {
        self.data[self.current_index.load(Ordering::Relaxed)].store(value, Ordering::Relaxed);
        self.current_index.store(
            (self.current_index.load(Ordering::Relaxed) + 1) % BUFFER_SIZE,
            Ordering::Relaxed,
        );
    }

    pub fn get(&self, index: usize) -> f32 {
        self.data[(self.current_index.load(Ordering::Relaxed) + index) % BUFFER_SIZE]
            .load(Ordering::Relaxed)
    }
}

impl Visualizer {
    pub fn new<L>(cx: &mut Context, visualizer: L) -> Handle<Self>
    where
        L: Lens<Target = Arc<VisualizerData>>,
    {
        Self {
            visualizer: visualizer.get(cx),
        }
        .build(cx, |_cx| {})
    }
}

impl View for Visualizer {
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
