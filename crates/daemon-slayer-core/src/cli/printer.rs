use owo_colors::OwoColorize;
use unicode_width::UnicodeWidthStr;

#[derive(Default)]
pub struct Printer {
    max_label_width: usize,
    lines: Vec<(String, String)>,
}

impl Printer {
    pub fn with_line(mut self, label: impl Into<String>, text: impl Into<String>) -> Self {
        let label: String = label.into();
        let text: String = text.into();

        if label.width() > self.max_label_width {
            self.max_label_width = label.width();
        }

        self.lines.push((label, text));
        self
    }

    pub fn print(self) -> String {
        self.lines
            .into_iter()
            .map(|(label, value)| {
                let padding = self.max_label_width - label.width();
                format!("{}{}: {}", " ".repeat(padding), label.bold(), value)
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}
