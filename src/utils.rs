use iced_baseview::widget::{Row, Column};
use iced_baseview::Element;

#[macro_export]
macro_rules! row {
    () => (
        Row::new()
    );
    ($($x:expr),+ $(,)?) => (
        Row::with_children(vec![$(Element::from($x)),+])
    );
}
#[macro_export]
macro_rules! column {
    () => (
        Column::new()
    );
    ($($x:expr),+ $(,)?) => (
        Column::with_children(vec![$(Element::from($x)),+])
    );
}