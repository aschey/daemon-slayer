use std::io::Stdout;

use tui::{
    backend::CrosstermBackend,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Block, BorderType, Borders, List, ListItem, ListState},
    Frame,
};

pub(crate) struct StatefulList<'a> {
    state: ListState,
    items: Vec<ListItem<'a>>,
}

impl<'a> StatefulList<'a> {
    pub(crate) fn new() -> StatefulList<'a> {
        let mut state = ListState::default();
        state.select(Some(0));
        StatefulList {
            state,
            items: vec![],
        }
    }

    pub(crate) fn add_item(&mut self, item: ListItem<'a>) {
        let len = self.items.len();
        self.items.push(item);
        if let Some(selected) = self.state.selected() {
            if len > 0 && selected == len - 1 {
                self.next();
            }
        }
    }

    pub(crate) fn next(&mut self) {
        if let Some(selected) = self.state.selected() {
            if !self.items.is_empty() && selected < self.items.len() - 1 {
                self.state.select(Some(selected + 1));
            }
        }
    }

    pub(crate) fn previous(&mut self) {
        if let Some(selected) = self.state.selected() {
            if selected > 0 {
                self.state.select(Some(selected - 1));
            }
        }
    }

    pub(crate) fn render(&mut self, frame: &mut Frame<CrosstermBackend<Stdout>>, area: Rect) {
        let logs_list = List::new(self.items.clone())
            .block(
                Block::default()
                    .borders(Borders::all())
                    .border_type(BorderType::Rounded)
                    .title("Logs"),
            )
            .highlight_style(
                Style::default()
                    .bg(Color::LightGreen)
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            );
        frame.render_stateful_widget(logs_list, area, &mut self.state);
    }
}
