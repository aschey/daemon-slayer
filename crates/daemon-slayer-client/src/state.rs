use owo_colors::{AnsiColors, OwoColorize};
use serde::{Deserialize, Serialize};
use strum::EnumProperty;

#[derive(
    strum::Display, strum::EnumProperty, Debug, Clone, PartialEq, Eq, Serialize, Deserialize,
)]
pub enum State {
    #[strum(props(color = "green"))]
    Started,
    #[strum(props(color = "red"))]
    Stopped,
    #[strum(props(color = "blue"), serialize = "Not Installed")]
    NotInstalled,
}

impl State {
    pub fn pretty_print(&self) -> String {
        let val = self.to_string();
        let color: AnsiColors = self
            .get_str("color")
            .expect("Color prop should be set")
            .into();
        val.color(color).to_string()
    }
}
