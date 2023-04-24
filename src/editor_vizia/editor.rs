#![allow(unused)]
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use nih_plug_vizia::{create_vizia_editor, ViziaState, ViziaTheming};

use nih_plug::prelude::*;
use nih_plug_vizia::vizia::prelude::*;
use nih_plug_vizia::widgets::*;

use crate::{
    visualizer::{self, Visualizer},
    NihSamplerParams, ThreadMessage,
};

use super::visualizer::VisualizerView;

#[derive(Lens)]
struct Data {
    params: Arc<NihSamplerParams>,
    producer: Arc<Mutex<rtrb::Producer<ThreadMessage>>>,
    debug: String,
    visualizer: Arc<Visualizer>,
}

#[derive(Clone)]
enum AppEvent {
    OpenFilePicker,
    LoadSample(PathBuf),
    RemoveSample(PathBuf),
}

impl Model for Data {
    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|app_event, _| match app_event {
            AppEvent::OpenFilePicker => {
                cx.spawn(|cx_proxy| {
                    if let Some(paths) = rfd::FileDialog::new().pick_files() {
                        for path in paths {
                            cx_proxy.emit(AppEvent::LoadSample(path));
                        }
                    }
                });
            }
            AppEvent::LoadSample(path) => {
                self.debug = format!("loading: {path:?}");
                if let Err(e) = self
                    .producer
                    .lock()
                    .unwrap()
                    .push(ThreadMessage::LoadSample(path.clone()))
                {
                    self.debug = e.to_string();
                }
            }
            AppEvent::RemoveSample(path) => {
                self.debug = format!("removing: {path:?}");
                if let Err(e) = self
                    .producer
                    .lock()
                    .unwrap()
                    .push(ThreadMessage::RemoveSample(path.clone()))
                {
                    self.debug = e.to_string();
                }
            }
        });
    }
}

pub fn default_state() -> Arc<ViziaState> {
    ViziaState::new(|| (700, 700))
}

pub fn create(
    params: Arc<NihSamplerParams>,
    editor_state: Arc<ViziaState>,
    producer: Arc<Mutex<rtrb::Producer<ThreadMessage>>>,
    visualizer: Arc<Visualizer>,
) -> Option<Box<dyn Editor>> {
    create_vizia_editor(editor_state, ViziaTheming::Custom, move |cx, _| {
        cx.add_theme(include_str!("theme.css"));

        Data {
            params: params.clone(),
            producer: producer.clone(),
            debug: "nothing".into(),
            visualizer: visualizer.clone(),
        }
        .build(cx);

        ResizeHandle::new(cx);
        VStack::new(cx, |cx| {
            HStack::new(cx, |cx| {
                Label::new(cx, "Nih Sampler").id("logo");
                VisualizerView::new(cx, Data::visualizer).id("visualizer");
            })
            .class("top-bar");

            VStack::new(cx, |cx| {
                // Label::new(cx, Data::debug).overflow(Overflow::Hidden);
                Label::new(cx, "Settings").class("heading");
                GenericUi::new(cx, Data::params).id("settings-container");

                HStack::new(cx, |cx| {
                    Label::new(cx, "Samples").class("heading");

                    Button::new(
                        cx,
                        |cx| cx.emit(AppEvent::OpenFilePicker),
                        |cx| Label::new(cx, "Add Sample(s)"),
                    )
                    .id("add-sample-button");
                })
                .height(Auto)
                .col_between(Stretch(1.0));

                ScrollView::new(cx, 0.0, 0.0, false, true, |cx| {
                    List::new(
                        cx,
                        Data::params.map(|params| params.sample_list.lock().unwrap().clone()),
                        |cx, index, item| {
                            HStack::new(cx, |cx| {
                                Label::new(
                                    cx,
                                    &item
                                        .get(cx)
                                        .file_name()
                                        .unwrap_or_default()
                                        .to_string_lossy()
                                        .to_string(),
                                );
                                Label::new(cx, "Remove").class("remove-label").on_press(
                                    move |cx| cx.emit(AppEvent::RemoveSample(item.get(cx).clone())),
                                );
                            })
                            .class("sample");
                        },
                    )
                    .class("vert-list")
                    .class("sample-list");
                })
                .class("sample-scrollview");
            })
            .class("main-body")
            .class("vert-list");
        })
        .id("container");
    })
}
fn param_row<L, Params, P, FMap>(cx: &mut Context, label: &str, params: L, params_to_param: FMap)
where
    L: Lens<Target = Params> + Clone,
    Params: 'static,
    P: Param + 'static,
    FMap: Fn(&Params) -> &P + Copy + 'static,
{
    HStack::new(cx, |cx| {
        Label::new(cx, label).class("param-label");
        ParamSlider::new(cx, params, params_to_param);
    })
    .class("row");
}
