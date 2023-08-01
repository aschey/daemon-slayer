use owo_colors::OwoColorize;
use unicode_width::UnicodeWidthStr;

#[derive(Default)]
pub struct Printer {
    max_label_width: usize,
    entries: Vec<Entry>,
}

struct Entry {
    label: String,
    label_width: usize,
    lines: Vec<String>,
}

impl Printer {
    pub fn with_line(self, label: impl Into<String>, text: impl Into<String>) -> Self {
        self.with_multi_line(label, vec![text])
    }

    pub fn with_multi_line<T: Into<String>>(
        mut self,
        label: impl Into<String>,
        text: Vec<T>,
    ) -> Self {
        let label: String = label.into();

        let label_width = std::str::from_utf8(&strip_ansi_escapes::strip(&label).unwrap())
            .unwrap()
            .width();

        self.max_label_width = self.max_label_width.max(label_width);

        self.entries.push(Entry {
            label,
            label_width,
            lines: text.into_iter().map(Into::into).collect(),
        });
        self
    }

    pub fn extend_from(mut self, printer: Printer) -> Self {
        for line in printer.entries {
            self.max_label_width = self.max_label_width.max(line.label_width);
            self.entries.push(line);
        }
        self
    }

    pub fn print(self) -> String {
        self.entries
            .into_iter()
            .map(|entry| {
                entry
                    .lines
                    .into_iter()
                    .enumerate()
                    .map(|(i, line)| {
                        if i == 0 {
                            let padding = self.max_label_width - entry.label_width;
                            format!("{}{}: {line}", " ".repeat(padding), entry.label.bold())
                        } else {
                            format!("{}  {}", " ".repeat(self.max_label_width), line)
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}
