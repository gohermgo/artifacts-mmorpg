use ratatui::{
    crossterm::event::{KeyEvent, KeyEventKind, KeyModifiers},
    prelude::*,
    widgets::{Block, Paragraph},
};

use core::ops::Deref;
use core::sync::atomic::AtomicBool;
use core::sync::atomic::Ordering::*;

use std::{sync::Arc, time::Duration};

#[derive(Clone, Debug, Default)]
pub struct StopFlag(Arc<AtomicBool>);

impl Deref for StopFlag {
    type Target = AtomicBool;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Ensures a true is set prior to dropping.
impl Drop for StopFlag {
    fn drop(&mut self) {
        self.0.store(true, Release);
    }
}

/// # The Artifacts MMORPG `ratatui` App!
///
/// Should allow downstream crates like the [`artifacts-mmorpg`] root binary
/// to configure this struct
#[derive(Debug)]
pub struct App {
    pub stop_flag: StopFlag,
    player_tracker: PlayerTrackerWidget,
    command_line_state: CommandLineState,
    pub cmd_tx: std::sync::mpsc::Sender<artifacts_core::UnmodifiedCommand>,
    // res_rx: std::sync::mpsc::Receiver<artifacts_core::ActionResponseDataSchema>,
}

impl App {
    pub fn new(
        player_name: String,
        cmd_tx: std::sync::mpsc::Sender<artifacts_core::UnmodifiedCommand>,
        // res_rx: std::sync::mpsc::Receiver<artifacts_core::ActionResponseDataSchema>,
    ) -> App {
        App {
            stop_flag: StopFlag::default(),
            player_tracker: PlayerTrackerWidget {
                player_name,
                health: HealthTrackerWidget::default(),
                cooldown: CooldownTrackerWidget {},
                position: PositionTrackerWidget {},
            },
            command_line_state: CommandLineState {
                line_input: CommandLineInput::default(),
                pending_buf: Vec::new(),
            },
            cmd_tx,
            // res_rx,
        }
    }
    /// chains together the call to [`draw`](App::draw) and [`handle_events`](App::handle_events)
    /// while repeatedly checking the [`stop_flag`](App::stop_flag) allowing for early termination!
    pub fn run_once_inner(
        &mut self,
        terminal: &mut ratatui::DefaultTerminal,
        player_state: &mut artifacts_core::PlayerTracker,
    ) -> std::io::Result<()> {
        terminal.try_draw(|frame| self.draw(frame, player_state))?;
        self.handle_events()?;

        Ok(())
    }

    pub fn draw(
        &mut self,
        frame: &mut Frame,
        player_state: &mut artifacts_core::PlayerTracker,
    ) -> std::io::Result<()> {
        let [player_tracker_area, command_line_area] = frame.area().layout(&Layout::vertical([
            Constraint::Fill(1),
            Constraint::Length(5),
        ]));
        frame.render_stateful_widget(&self.player_tracker, player_tracker_area, player_state);
        frame.render_stateful_widget(
            &CommandLineWidget {},
            command_line_area,
            &mut self.command_line_state,
        );
        Ok(())
    }

    pub fn handle_events(&mut self) -> std::io::Result<()> {
        if ratatui::crossterm::event::poll(Duration::from_millis(10))?
            && let event = ratatui::crossterm::event::read()?
        {
            if matches!(
                event,
                Event::Key(KeyEvent {
                    code: KeyCode::Esc,
                    kind: KeyEventKind::Press,
                    ..
                })
            ) {
                self.stop_flag.store(true, Release);
            }

            if self.command_line_state.handle_crossterm_event(&event) {
                let mut pending_command_line_iterator = core::iter::from_fn(|| {
                    self.command_line_state.pending_buf.pop().map(|value| {
                        value
                            .split_whitespace()
                            .map(<str>::to_string)
                            .collect::<Box<[String]>>()
                            .into_iter()
                    })
                })
                .flatten();
                let mut parser = artifacts_core::ArgParser::new(&mut pending_command_line_iterator);

                while let Some(cmd) = parser.parse_next_command() {
                    let repeat_count = match &cmd.modifier {
                        Some(artifacts_core::CommandModifier::Repeat(times)) => *times as usize,
                        _ => 1,
                    };

                    for _ in 0..repeat_count {
                        let cmd = artifacts_core::UnmodifiedCommand {
                            name: cmd.name.clone(),
                            arguments: cmd.arguments.clone(),
                        };
                        self.cmd_tx
                            .send(cmd.clone())
                            .expect("failed to send command")
                    }
                }

                tracing::info!("finished parsing {parser:?}");
                // parser
            }
        };

        Ok(())
        // unimplemented!("App::handle_events method!")
    }
}

// pub fn run_app(app: &mut App) -> std::io::Result<()> {
//     ratatui::run(|terminal| app.run_once_inner(terminal))
// }
pub use ratatui;
// pub fn parse_user_event(event: Event) -> Option<artifacts_core::Command> {}

#[derive(Debug)]
pub struct PlayerTrackerWidget {
    pub player_name: String,
    pub health: HealthTrackerWidget,
    pub cooldown: CooldownTrackerWidget,
    pub position: PositionTrackerWidget,
}

impl StatefulWidget for &PlayerTrackerWidget {
    type State = artifacts_core::PlayerTracker;
    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let block = ratatui::widgets::Block::new()
            .title(self.player_name.as_str())
            .borders(ratatui::widgets::Borders::all())
            .border_type(ratatui::widgets::BorderType::Rounded);

        let block_inner = block.inner(area);

        block.render(area, buf);

        let [upper, _lower] = block_inner.layout(&Layout::vertical([
            Constraint::Percentage(60),
            Constraint::Percentage(40),
        ]));

        let [health_tracker_area, upper] = upper.layout(&Layout::vertical([
            Constraint::Length(5),
            Constraint::Fill(1),
        ]));

        let health_bar_block = ratatui::widgets::Block::bordered().title("health");

        let health_bar_area = health_bar_block.inner(health_tracker_area);

        health_bar_block.render(health_tracker_area, buf);

        self.health.render(health_bar_area, buf, &mut state.health);

        let [cooldown_area, position_area] = upper.layout(&Layout::horizontal([
            Constraint::Fill(1),
            Constraint::Length(8),
        ]));

        self.cooldown
            .render(cooldown_area, buf, &mut state.cooldown);

        self.position
            .render(position_area, buf, &mut state.position);
    }
}

#[derive(Debug)]
pub struct PositionTrackerWidget {}

impl StatefulWidget for &PositionTrackerWidget {
    type State = artifacts_core::PositionTracker;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        if let Some(st) = state.active_position.as_ref() {
            ratatui::text::Text::from_iter(vec![
                format!("X: {:4}", st.position.x),
                format!("Y: {:4}", st.position.y),
            ])
            .render(area, buf);
        } else {
            ratatui::text::Text::raw("position-tracker").render(area, buf)
        }
    }
}

#[derive(Debug, Default)]
pub struct HealthTrackerWidget {}

impl StatefulWidget for &HealthTrackerWidget {
    type State = artifacts_core::HealthTracker;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        if let Some(artifacts_core::ActiveHealth {
            max_value,
            current_value,
        }) = state.active_health.as_ref()
        {
            let [health_label_area, bar_area] =
                area.layout(&Layout::vertical([Constraint::Ratio(1, 2); 2]));

            Text::raw(format!("{current_value:3} / {max_value:3}")).render(health_label_area, buf);

            let line_gauge_ratio = f64::max(1.0, *current_value as f64 / *max_value as f64);

            let line_gauge_filled_style = Style::default().light_green();
            let line_gauge_unfilled_style = Style::default().dim();

            ratatui::widgets::LineGauge::default()
                .ratio(line_gauge_ratio)
                .filled_symbol("I")
                .filled_style(line_gauge_filled_style)
                .unfilled_symbol("=")
                .unfilled_style(line_gauge_unfilled_style)
                .render(bar_area, buf);
        } else {
            ratatui::text::Text::raw("health-tracker").render(area, buf)
        }
    }
}

#[derive(Debug, Default)]
pub struct CooldownTrackerWidget {}

impl StatefulWidget for &CooldownTrackerWidget {
    type State = artifacts_core::CooldownTracker;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        if let Some(st) = state.active_cooldown.as_ref() {
            let area = area.inner(Margin {
                horizontal: 0,
                vertical: 0,
            });
            let block_layout = Layout::vertical([Constraint::Fill(1), Constraint::Fill(1)]);
            let bar_layout = Layout::horizontal([Constraint::Length(10), Constraint::Fill(1)]);

            let [title_area, details_area] = area.layout(&block_layout);

            let [_title_padding, title_area] = title_area.layout(&bar_layout);

            let title_content = st.command_name.as_ref();

            tui_big_text::BigText::builder()
                .pixel_size(tui_big_text::PixelSize::Quadrant)
                .lines(vec![Line::raw(title_content)])
                .style(Style::default().red().italic())
                .build()
                .render(
                    title_area.inner(Margin {
                        horizontal: 0,
                        vertical: 1,
                    }),
                    buf,
                );
            // .render(
            //     title_area.inner(Margin {
            //         horizontal: 0,
            //         vertical: 2,
            //     }),
            //     buf,
            // );

            let [_unused_details_area, cd_bar_area] =
                details_area.layout(&Layout::vertical([Constraint::Ratio(1, 2); 2]));

            let [_unused_bar_area, gauge_area] = cd_bar_area.layout(&bar_layout);

            let ratio = f64::min(
                st.elapsed().as_seconds_f64() / st.total().as_seconds_f64(),
                1.,
            );

            let total = st.total().as_seconds_f64();
            let elapsed = f64::min(st.elapsed().as_seconds_f64(), total);

            ratatui::widgets::LineGauge::default()
                .ratio(ratio)
                .label(format!("{elapsed:.3} / {total:.3}",))
                .filled_symbol("I")
                .filled_style(Style::default().blue())
                .unfilled_symbol("=")
                .unfilled_style(Style::default().dim())
                .render(gauge_area, buf);
        } else {
            ratatui::text::Text::raw("cooldown-tracker").render(area, buf)
        }
    }
}

use ratatui::crossterm::event::{Event, KeyCode};

#[derive(Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum CommandLineInput {
    #[default]
    Empty,
    SingleCommand(String),
    SequencedCommands(Vec<String>),
}

impl CommandLineInput {
    pub fn push_char(&mut self, ch: char) {
        match self {
            CommandLineInput::Empty => {
                *self = CommandLineInput::SingleCommand(format!("{ch}"));
            }
            CommandLineInput::SingleCommand(command) => command.push(ch),
            CommandLineInput::SequencedCommands(commands) => {
                if let Some(command) = commands.last_mut() {
                    command.push(ch);
                }
            }
        }
    }
    /// Returns the `last` string in a sequence, or the `single` command contained
    pub fn as_string_mut(&mut self) -> Option<&mut String> {
        match self {
            CommandLineInput::Empty => None,
            CommandLineInput::SingleCommand(s) => Some(s),
            CommandLineInput::SequencedCommands(items) => items.last_mut(),
        }
    }
    pub fn pop_char(&mut self) -> Option<char> {
        match self {
            CommandLineInput::Empty => None,
            CommandLineInput::SingleCommand(command_line_buf) => {
                let ch_opt = command_line_buf.pop();
                let is_empty_after_pop = command_line_buf.is_empty();

                // guard: return to empty if after modification,
                //        the line-buf is empty
                if is_empty_after_pop {
                    *self = CommandLineInput::Empty;
                }

                ch_opt
            }
            CommandLineInput::SequencedCommands(command_list) => {
                let Some(mut last_popped) = command_list.pop() else {
                    // in this case the list is empty, return to empty-state
                    *self = CommandLineInput::Empty;
                    return None;
                };

                // this `?` represents an empty-string in the `cursor`
                // position!
                //
                // thus we return None (indicating that) and the popped
                // element is no longer tracked in
                let ch = last_popped.pop()?;

                // guard: reinsert if the string is not empty
                if !last_popped.is_empty() {
                    command_list.push(last_popped);
                }

                // guard: return to empty if list is empty
                if command_list.is_empty() {
                    *self = CommandLineInput::Empty;
                }

                Some(ch)
            }
        }
    }
    pub fn push_string(&mut self, command: impl Into<String>) {
        match self {
            CommandLineInput::Empty => {
                *self = CommandLineInput::SingleCommand(command.into());
            }
            CommandLineInput::SingleCommand(first_command) => {
                *self = CommandLineInput::SequencedCommands(vec![
                    core::mem::take(first_command),
                    command.into(),
                ]);
            }
            CommandLineInput::SequencedCommands(items) => items.push(command.into()),
        }
    }
    pub fn pop_string(&mut self) -> Option<String> {
        match self {
            CommandLineInput::Empty => None,
            CommandLineInput::SingleCommand(value) => {
                let return_value = core::mem::take(value);
                *self = CommandLineInput::Empty;
                Some(return_value)
            }
            CommandLineInput::SequencedCommands(items) => {
                let Some(return_value) = items.pop() else {
                    *self = CommandLineInput::Empty;
                    return None;
                };

                // set to single-command state here
                if items.len() == 1 {
                    *self = CommandLineInput::SingleCommand(core::mem::take(&mut items[0]));
                };

                Some(return_value)
            }
        }
    }
    pub fn command_count(&self) -> usize {
        match self {
            CommandLineInput::Empty => 0,
            CommandLineInput::SingleCommand(_) => 1,
            CommandLineInput::SequencedCommands(items) => items.len(),
        }
    }

    /// Returns a vector of strings each representing a command
    pub fn into_vec(self) -> Vec<String> {
        match self {
            CommandLineInput::Empty => vec![],
            CommandLineInput::SingleCommand(single_command) => vec![single_command],
            CommandLineInput::SequencedCommands(items) => items,
        }
    }
}

pub enum CommandLineEvent {
    EnqueueCommand(),
}

#[derive(Debug)]
struct CommandLineState {
    line_input: CommandLineInput,
    pending_buf: Vec<String>,
}

impl CommandLineState {
    fn handle_crossterm_event(&mut self, event: &Event) -> bool {
        let Event::Key(key_event) = event else {
            return false;
        };

        match key_event.code {
            KeyCode::Char(ch) if key_event.is_press() | key_event.is_repeat() => {
                self.line_input.push_char(ch);
                true
            }
            KeyCode::Backspace if key_event.is_press() | key_event.is_repeat() => {
                self.line_input.pop_char();
                true
            }
            KeyCode::Enter
                if key_event.is_press() && key_event.modifiers == KeyModifiers::SHIFT =>
            {
                // put an empty string onto the end, to continue the command
                self.line_input.push_string("");
                true
            }
            KeyCode::Enter if key_event.is_press() => {
                if key_event.modifiers.contains(KeyModifiers::CONTROL) {
                    // in this case, we must push an empty line to the buffer
                    self.line_input.push_string("");
                } else {
                    // in this case, we are sending all the lines we have
                    while let Some(command) = self.line_input.pop_string() {
                        self.pending_buf.push(command)
                    }
                }
                true
            }
            _ => false,
        }
    }
}

struct CommandLineWidget {}

impl StatefulWidget for &CommandLineWidget {
    type State = CommandLineState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let paragraph_text = match &state.line_input {
            CommandLineInput::Empty => Text::raw("> "),
            CommandLineInput::SingleCommand(command) => {
                Text::raw(format!("> {}", command.as_str()))
            }
            CommandLineInput::SequencedCommands(items) => {
                let mut line = Text::default();
                let element_count = items.len();
                for (index, element) in items.iter().enumerate() {
                    // last element
                    if index + 1 == element_count {
                        line.push_line(format!("> {element}"));
                    } else {
                        line.push_line(format!("| {element}"));
                    }
                }
                line
            }
        };
        Paragraph::new(paragraph_text)
            .block(Block::bordered().title("command"))
            .render(area, buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_line_buf() {
        let mut cmdbuf = CommandLineInput::default();

        std::assert_matches!(cmdbuf, CommandLineInput::Empty);

        for ch in "rest".chars() {
            cmdbuf.push_char(ch);
        }

        assert_eq!(cmdbuf, CommandLineInput::SingleCommand("rest".into()));

        cmdbuf.push_string("fight");

        assert_eq!(
            cmdbuf,
            CommandLineInput::SequencedCommands(vec!["rest".into(), "fight".into()])
        );

        let last_char = cmdbuf.pop_char().expect("failed to pop character");
        std::assert_matches!(last_char, 't');

        let last = cmdbuf.pop_string().expect("failed to pop previous command");
        assert_eq!(last, "figh");

        assert_eq!(cmdbuf, CommandLineInput::SingleCommand("rest".into()));

        let last_char = cmdbuf.pop_char().expect("failed to pop character");
        std::assert_matches!(last_char, 't');

        assert_eq!(cmdbuf, CommandLineInput::SingleCommand("res".into()));
    }
}
