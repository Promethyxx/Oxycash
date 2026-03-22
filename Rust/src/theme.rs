// oxycash-rs - theme.rs
// Mapping of core/theme.py
/// A color in "#RRGGBB" format
pub type Color = &'static str;

pub struct ThemePalette {
    pub bg:          Color,
    pub bg2:         Color,
    pub card:        Color,
    pub card_border: Color,
    pub text:        Color,
    pub text2:       Color,
    pub text3:       Color,
    pub red:         Color,
    pub teal:        Color,
    pub gold:        Color,
    pub amber:       Color,
    pub brown:       Color,
    pub green:       Color,
    pub danger:      Color,
    pub blue:        Color,
    pub purple:      Color,
}

pub const DARK: ThemePalette = ThemePalette {
    bg:          "#0D0D0D",
    bg2:         "#131311",
    card:        "#161614",
    card_border: "#222220",
    text:        "#E8E4DE",
    text2:       "#9A9690",
    text3:       "#4A4744",
    red:         "#D96459",
    teal:        "#85CDCA",
    gold:        "#E8A87C",
    amber:       "#F2D388",
    brown:       "#C7B198",
    green:       "#7BC47F",
    danger:      "#E05555",
    blue:        "#6FA8DC",
    purple:      "#B794D6",
};

pub const LIGHT: ThemePalette = ThemePalette {
    bg:          "#F5F4F0",
    bg2:         "#ECEAE4",
    card:        "#E4E2DC",
    card_border: "#D0CEC8",
    text:        "#2A2520",
    text2:       "#6A6560",
    text3:       "#A0A098",
    red:         "#B8453A",
    teal:        "#4A8E8A",
    gold:        "#C97B4B",
    amber:       "#C9A84C",
    brown:       "#8E7E6A",
    green:       "#4A9E4E",
    danger:      "#C0453A",
    blue:        "#4A7DB8",
    purple:      "#8B5CA6",
};

pub struct Theme {
    pub is_dark: bool,
}

impl Theme {
    pub fn new(dark: bool) -> Self {
        Self { is_dark: dark }
    }

    pub fn palette(&self) -> &'static ThemePalette {
        if self.is_dark { &DARK } else { &LIGHT }
    }

    pub fn toggle(&mut self) {
        self.is_dark = !self.is_dark;
    }
}
