use std::{
    borrow::Cow,
    collections::BTreeMap,
    path::PathBuf,
    sync::{Arc, Mutex, MutexGuard},
};

use iced_baseview::{
    alignment::Horizontal,
    executor, open_parented,
    widget::{Button, Column, Container, Row, Rule, Slider, Text, TextInput},
    window::{WindowHandle, WindowQueue},
    Alignment, Application, Command, Element, Length, Settings,
};
use nih_plug::prelude::ParentWindowHandle;

use crate::{
    column,
    layers::{NoteLayer, VelocityLayer},
    loaded_sample::LoadedSample,
    map::Map,
    row,
    theme::{ButtonType, Theme},
    NihSamplerParams, TEST_SAMPLE,
};

pub const TEXT_SIZE: u16 = 16;

#[derive(Debug, Clone)]
pub enum Message {
    AddNoteLayer,
    SelectNoteLayer(usize),
    ChangeSelectedNoteLayerNote(i8),
    RemoveSelectedNoteLayer,

    AddVelocityLayer,
    SelectVelocityLayer(usize),
    ChangeSelectedVelocityLayerMaxVelocity(u8),
    RemoveSelectedVelocityLayer,

    OpenFilePicker,
    AddSampleLayers(Vec<PathBuf>),
    SelectSampleLayer(usize),
    RemoveSelectedSampleLayer,

    NoOp,
}
pub struct IcedEditor {
    params: Arc<NihSamplerParams>,
}

impl IcedEditor {
    /// little helper to reduce boilerplate
    fn note_map(&self) -> MutexGuard<Map<NoteLayer>> {
        self.params.note_map.lock().unwrap()
    }
}

impl Application for IcedEditor {
    type Executor = executor::Default;
    type Message = Message;
    type Flags = Arc<NihSamplerParams>;
    type Theme = Theme;

    fn new(flags: Self::Flags) -> (Self, Command<Self::Message>) {
        (Self { params: flags }, Command::none())
    }

    fn update(
        &mut self,
        _window: &mut WindowQueue,
        message: Self::Message,
    ) -> Command<Self::Message> {
        match message {
            Message::AddNoteLayer => self.note_map().vec.push(NoteLayer::new(40)),
            Message::AddVelocityLayer => self
                .note_map()
                .selected_mut()
                .unwrap()
                .velocity_map
                .vec
                .push(VelocityLayer::new(127)),
            Message::SelectNoteLayer(index) => self.note_map().select(index),
            Message::SelectVelocityLayer(index) => self
                .note_map()
                .selected_mut()
                .unwrap()
                .velocity_map
                .select(index),
            Message::SelectSampleLayer(index) => self
                .note_map()
                .selected_mut()
                .unwrap()
                .velocity_map
                .selected_mut()
                .unwrap()
                .samples
                .select(index),
            Message::OpenFilePicker => {
                return Command::perform(rfd::AsyncFileDialog::new().pick_files(), |opt| {
                    if let Some(files) = opt {
                        Message::AddSampleLayers(
                            files
                                .iter()
                                .map(|handle| handle.path().to_path_buf())
                                .collect(),
                        )
                    } else {
                        Message::NoOp
                    }
                })
            }
            Message::AddSampleLayers(paths) => {
                for path in paths {
                    self.note_map()
                        .selected_mut()
                        .unwrap()
                        .velocity_map
                        .selected_mut()
                        .unwrap()
                        .samples
                        .vec
                        .push(LoadedSample::new(path));
                }
            }
            Message::NoOp => (),
            Message::RemoveSelectedNoteLayer => self.note_map().remove_selected(),
            Message::RemoveSelectedVelocityLayer => self
                .note_map()
                .selected_mut()
                .unwrap()
                .velocity_map
                .remove_selected(),
            Message::RemoveSelectedSampleLayer => self
                .note_map()
                .selected_mut()
                .unwrap()
                .velocity_map
                .selected_mut()
                .unwrap()
                .samples
                .remove_selected(),
            Message::ChangeSelectedVelocityLayerMaxVelocity(new_max) => {
                self.note_map()
                    .selected_mut()
                    .unwrap()
                    .velocity_map
                    .selected_mut()
                    .unwrap()
                    .max_velocity = new_max
            }
            Message::ChangeSelectedNoteLayerNote(delta) => {
                let mut note_map = self.note_map();
                let mut note_layer = note_map.selected_mut().unwrap();
                note_layer.note = note_layer.note.saturating_add_signed(delta)
            }
        };
        Command::none()
    }

    fn view(&self) -> Element<'_, Self::Message, Self::Theme> {
        let mut note_layers = Column::new().width(Length::Fill).into();
        let mut velocity_layers = Column::new().width(Length::Fill).into();
        let mut sample_layers = Column::new().width(Length::Fill).into();

        // note layers
        note_layers = render_map(
            &self.note_map(),
            "Note layers",
            &render_note_layer,
            Message::AddNoteLayer,
        );

        // velocity layers
        if let Some(selected_note_layer) = self.note_map().selected() {
            velocity_layers = render_map(
                &selected_note_layer.velocity_map,
                "Velocity layers",
                &render_velocity_layer,
                Message::AddVelocityLayer,
            );

            // sample layers
            if let Some(selected_sample_layer) = selected_note_layer.velocity_map.selected() {
                sample_layers = render_map(
                    &selected_sample_layer.samples,
                    "Sample layers",
                    &render_sample_layer,
                    Message::OpenFilePicker,
                );
            }
        }

        Row::with_children(vec![note_layers, velocity_layers, sample_layers])
            .spacing(10)
            .padding(10)
            .into()
    }

    fn title(&self) -> String {
        "Nih Sampler".into()
    }

    fn theme(&self) -> Self::Theme {
        Default::default()
    }
}

fn centered_text<'a>(content: impl Into<Cow<'a, str>>) -> Element<'a, Message, Theme> {
    Text::new(content)
        .size(TEXT_SIZE)
        .width(Length::Fill)
        .horizontal_alignment(Horizontal::Center)
        .into()
}

fn render_map<'a, T: 'static + Send + Sync>(
    map: &Map<T>,
    label: &'static str,
    render: &dyn Fn(&T, usize, bool) -> Element<'a, Message, Theme>,
    add_message: Message,
) -> Element<'a, Message, Theme> {
    let mut col = Column::new().spacing(10).width(Length::Fill);

    col = col.push(centered_text(label));

    for (index, item) in map.vec.iter().enumerate() {
        col = col.push(render(&item, index, map.is_selected(index)));
    }

    let add_button = Button::new(centered_text("ADD"))
        .style(ButtonType::Add)
        .on_press(add_message)
        .width(Length::Fill);

    col = col.push(add_button);
    col.into()
}

fn render_note_layer<'a>(
    note_layer: &NoteLayer,
    index: usize,
    selected: bool,
) -> Element<'a, Message, Theme> {
    let mut button = Button::new(centered_text(note_layer.note.to_string()))
        .on_press(Message::SelectNoteLayer(index))
        .width(Length::Fill);

    if selected {
        button = button.style(ButtonType::Selected);
        let dec_button = Button::new(centered_text("-"))
            .width(Length::Fill)
            .on_press(Message::ChangeSelectedNoteLayerNote(-1));
        let inc_button = Button::new(centered_text("+"))
            .width(Length::Fill)
            .on_press(Message::ChangeSelectedNoteLayerNote(1));
        let remove_button = Button::new(centered_text("Remove"))
            .width(Length::Fill)
            .on_press(Message::RemoveSelectedNoteLayer);

        return column![button, row![dec_button, inc_button, remove_button]].into();
    }

    button.into()
}

fn render_velocity_layer<'a>(
    velocity_layer: &VelocityLayer,
    index: usize,
    selected: bool,
) -> Element<'a, Message, Theme> {
    let mut button = Button::new(centered_text(velocity_layer.max_velocity.to_string()))
        .on_press(Message::SelectVelocityLayer(index))
        .width(Length::Fill);

    if selected {
        button = button.style(ButtonType::Selected);
        let slider = Slider::new(
            0..=127u8,
            velocity_layer.max_velocity,
            Message::ChangeSelectedVelocityLayerMaxVelocity,
        )
        .width(Length::Fill);
        let remove_button = Button::new(centered_text("Remove"))
            .width(Length::Fill)
            .on_press(Message::RemoveSelectedVelocityLayer);
        return column![button, row![slider, remove_button]].into();
    }

    button.into()
}

fn render_sample_layer<'a>(
    sample_layer: &LoadedSample,
    index: usize,
    selected: bool,
) -> Element<'a, Message, Theme> {
    let mut button = Button::new(centered_text(
        sample_layer
            .path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string(),
    ))
    .on_press(Message::SelectSampleLayer(index))
    .width(Length::Fill);

    if selected {
        button = button.style(ButtonType::Selected);

        let remove_button = Button::new(centered_text("Remove"))
            .width(Length::Fill)
            .on_press(Message::RemoveSelectedSampleLayer);
        return column![button, row![remove_button]].into();
    }

    button.into()
}

pub struct MyHandle<T: 'static + Send>(WindowHandle<T>);
unsafe impl<T: Send> Send for MyHandle<T> {}

impl nih_plug::editor::Editor for IcedEditor {
    fn spawn(
        &self,
        parent: nih_plug::prelude::ParentWindowHandle,
        _context: std::sync::Arc<dyn nih_plug::prelude::GuiContext>,
    ) -> Box<dyn std::any::Any + Send> {
        let handle = open_parented::<IcedEditor, ParentWindowHandle>(
            &parent,
            Settings {
                window: iced_baseview::baseview::WindowOpenOptions {
                    size: iced_baseview::baseview::Size::new(700.0, 700.0),
                    scale: iced_baseview::baseview::WindowScalePolicy::ScaleFactor(1.0),
                    title: "dasdas".to_string(),
                },
                flags: self.params.clone(),
                iced_baseview: iced_baseview::settings::IcedBaseviewSettings {
                    ignore_non_modifier_keys: false,
                    always_redraw: true,
                },
            },
        );

        Box::new(MyHandle(handle))
    }

    fn size(&self) -> (u32, u32) {
        (700, 700)
    }

    fn set_scale_factor(&self, _factor: f32) -> bool {
        true
    }

    fn param_value_changed(&self, _id: &str, _normalized_value: f32) {
        todo!()
    }

    fn param_modulation_changed(&self, _id: &str, _modulation_offset: f32) {
        todo!()
    }

    fn param_values_changed(&self) {
        todo!()
    }
}
