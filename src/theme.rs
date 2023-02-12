use iced_baseview::{
    widget::{application, button, container, scrollable, slider, text, text_input},
    Background, Color, Vector,
};

#[derive(Default)]
pub struct Theme;

const BACKGROUND_COLOR: Color = Color::from_rgb(0.04, 0.04, 0.04);
const GRAY: Color = Color::from_rgb(0.09, 0.09, 0.09);
const ACCENT_COLOR: Color = Color::from_rgb(0.85, 1.0, 0.45);
const BORDER_RADIUS: f32 = 7.0;

fn mul(mut color: Color, val: f32) -> Color {
    color.r *= val;
    color.g *= val;
    color.b *= val;

    color
}

#[derive(Default, Debug, Clone)]
pub enum ButtonType {
    Selected,
    #[default]
    Unselected,
    Add,
}
impl button::StyleSheet for Theme {
    type Style = ButtonType;

    fn active(&self, style: &Self::Style) -> button::Appearance {
        match style {
            ButtonType::Selected => button::Appearance {
                shadow_offset: Vector::default(),
                background: Some(Background::Color(ACCENT_COLOR)),
                border_radius: BORDER_RADIUS,
                border_width: 0.0,
                border_color: Color::BLACK,
                text_color: Color::BLACK,
            },
            ButtonType::Unselected => button::Appearance {
                shadow_offset: Vector::default(),
                background: Some(Background::Color(GRAY)),
                border_radius: BORDER_RADIUS,
                border_width: 0.0,
                border_color: Color::BLACK,
                text_color: Color::WHITE,
            },
            ButtonType::Add => button::Appearance {
                shadow_offset: Vector::default(),
                background: Some(Background::Color(ACCENT_COLOR)),
                border_radius: BORDER_RADIUS,
                border_width: 0.0,
                border_color: Color::BLACK,
                text_color: Color::BLACK,
            },
        }
    }

    fn hovered(&self, style: &Self::Style) -> button::Appearance {
        let mut active = self.active(style);
        if let Some(Background::Color(c)) = active.background {
            let multiplier = match style {
                ButtonType::Selected | ButtonType::Add=> 0.85,
                ButtonType::Unselected => 1.5,
            };
            active.background = Some(Background::Color(mul(c, multiplier)));
        }
        active
    }

    fn pressed(&self, style: &Self::Style) -> button::Appearance {
        let mut hovered = self.hovered(style);
        if let Some(Background::Color(c)) = hovered.background {
            let multiplier = match style {
                ButtonType::Selected | ButtonType::Add => 0.85,
                ButtonType::Unselected => 1.5,
            };
            hovered.background = Some(Background::Color(mul(c, multiplier)));
        }
        hovered
    }

    // fn disabled(&self, style: &Self::Style) -> button::Appearance {
    //     let active = self.active(style);

    //     button::Appearance {
    //         shadow_offset: Vector::default(),
    //         background: active.background.map(|background| match background {
    //             Background::Color(color) => Background::Color(Color {
    //                 a: color.a * 0.5,
    //                 ..color
    //             }),
    //         }),
    //         text_color: Color {
    //             a: active.text_color.a * 0.5,
    //             ..active.text_color
    //         },
    //         ..active
    //     }
    // }
}

impl text_input::StyleSheet for Theme {
    type Style = ();

    fn active(&self, _style: &Self::Style) -> text_input::Appearance {
        text_input::Appearance {
            background: Background::Color(Color::WHITE),
            border_radius: BORDER_RADIUS,
            border_width: 0.0,
            border_color: Color::BLACK,
        }
    }

    fn focused(&self, style: &Self::Style) -> text_input::Appearance {
        self.active(style)
    }

    fn placeholder_color(&self, _style: &Self::Style) -> Color {
        Color::new(0.5, 0.5, 0.5, 1.0)
    }

    fn value_color(&self, _style: &Self::Style) -> Color {
        Color::BLACK
    }

    fn selection_color(&self, _style: &Self::Style) -> Color {
        mul(ACCENT_COLOR, 0.7)
    }
}

impl scrollable::StyleSheet for Theme {
    type Style = ();

    fn active(&self, _style: &Self::Style) -> scrollable::Scrollbar {
        scrollable::Scrollbar {
            background: Some(Background::Color(mul(BACKGROUND_COLOR, 1.5))),
            border_radius: BORDER_RADIUS,
            border_width: 0.0,
            border_color: Color::BLACK,
            scroller: scrollable::Scroller {
                color: ACCENT_COLOR,
                border_radius: BORDER_RADIUS,
                border_width: 0.0,
                border_color: Color::BLACK,
            },
        }
    }

    fn hovered(&self, style: &Self::Style) -> scrollable::Scrollbar {
        let mut active = self.active(style);
        active.scroller.color = mul(active.scroller.color, 0.85);
        active
    }

    fn dragging(&self, style: &Self::Style) -> scrollable::Scrollbar {
        let mut hovered = self.hovered(style);
        hovered.scroller.color = mul(hovered.scroller.color, 0.85);
        hovered
    }
}

impl text::StyleSheet for Theme {
    type Style = ();

    fn appearance(&self, _style: Self::Style) -> text::Appearance {
        text::Appearance { color: None }
    }
}

impl container::StyleSheet for Theme {
    type Style = ();

    fn appearance(&self, _style: &Self::Style) -> container::Appearance {
        container::Appearance {
            text_color: None,
            background: None,
            border_radius: BORDER_RADIUS,
            border_width: 0.0,
            border_color: Color::BLACK,
        }
    }
}

impl slider::StyleSheet for Theme {
    type Style = ();

    fn active(&self, _style: &Self::Style) -> slider::Appearance {
        slider::Appearance {
            rail_colors: (BACKGROUND_COLOR, BACKGROUND_COLOR),
            handle: slider::Handle {
                shape: slider::HandleShape::Circle { radius: 7.0 },
                color: ACCENT_COLOR,
                border_width: 1.0,
                border_color: BACKGROUND_COLOR,
            },
        }
    }

    fn hovered(&self, style: &Self::Style) -> slider::Appearance {
        self.active(style)
    }

    fn dragging(&self, style: &Self::Style) -> slider::Appearance {
        self.active(style)
    }
}

impl application::StyleSheet for Theme {
    type Style = ();

    fn appearance(&self, _style: &Self::Style) -> application::Appearance {
        application::Appearance {
            background_color: BACKGROUND_COLOR,
            text_color: Color::WHITE,
        }
    }
}
