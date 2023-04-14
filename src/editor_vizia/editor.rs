#![allow(unused)]
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use nih_plug_vizia::{create_vizia_editor, ViziaState, ViziaTheming};

use nih_plug::prelude::*;
use nih_plug_vizia::vizia::prelude::*;
use nih_plug_vizia::widgets::*;

use crate::{NihSamplerParams, ThreadMessage};

#[derive(Lens)]
struct Data {
    params: Arc<NihSamplerParams>,
    producer: Arc<Mutex<rtrb::Producer<ThreadMessage>>>,
    debug: String,
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
) -> Option<Box<dyn Editor>> {
    create_vizia_editor(editor_state, ViziaTheming::Custom, move |cx, _| {
        cx.add_theme(include_str!("theme.css"));
        Data {
            params: params.clone(),
            producer: producer.clone(),
            debug: "nothing".into(),
        }
        .build(cx);

        ResizeHandle::new(cx);

        Label::new(cx, Data::debug);

        VStack::new(cx, |cx| {
            HStack::new(cx, |cx| {
                Label::new(cx, "MIDI Note").class("param-label");
                ParamSlider::new(cx, Data::params, |params| &params.note);
            })
            .class("param-row");

            HStack::new(cx, |cx| {
                Label::new(cx, "Min Velocity").class("param-label");
                ParamSlider::new(cx, Data::params, |params| &params.min_velocity);
            })
            .class("param-row");

            HStack::new(cx, |cx| {
                Label::new(cx, "Max Velocity").class("param-label");
                ParamSlider::new(cx, Data::params, |params| &params.max_velocity);
            })
            .class("param-row");

            HStack::new(cx, |cx| {
                Label::new(cx, "Min Volume").class("param-label");
                ParamSlider::new(cx, Data::params, |params| &params.min_volume);
            })
            .class("param-row");

            HStack::new(cx, |cx| {
                Label::new(cx, "Max Volume").class("param-label");
                ParamSlider::new(cx, Data::params, |params| &params.max_volume);
            })
            .class("param-row");
        })
        .class("params-list");

        // Display all the loaded samples
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

                        Button::new(
                            cx,
                            move |cx| cx.emit(AppEvent::RemoveSample(item.get(cx).clone())),
                            |cx| Label::new(cx, "Remove"),
                        );
                    });
                },
            );
        });

        Button::new(
            cx,
            |cx| cx.emit(AppEvent::OpenFilePicker),
            |cx| Label::new(cx, "Add Sample(s)"),
        );
    })
}
