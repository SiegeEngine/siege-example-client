
use super::Point;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum TextColor {
    Black = 0,
    White = 1,
    Red = 2,
    Gray = 3,
    Gold = 4,
    Green = 5,
    Blue = 6,
    Lavender = 7
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum Font {
    Main = 0,
    Mono = 1,
    Title = 2,
    Fantasy = 3,
}

#[derive(Debug, Clone)]
pub struct TextLine {
    pub ui_coordinates: Point,
    pub lineheight: u8,
    pub color: TextColor,
    pub outline: Option<TextColor>,
    pub font: Font,
    pub alpha: u8,
    pub text: String,
}
