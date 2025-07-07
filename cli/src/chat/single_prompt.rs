use std::{sync::mpsc, thread, time::Duration};

use color_eyre::Result;
use ratatui::{
    crossterm::event,
    style::{Color, Style, Stylize},
    text::Line,
    widgets::{Block, Padding, Paragraph, Widget, Wrap},
    DefaultTerminal, TerminalOptions, Viewport,
};
use throbber_widgets_tui::{Throbber, ThrobberState, BRAILLE_EIGHT_DOUBLE};

use crate::llm;

type LlmResult<T> = Result<T, llm::Error>;

/// Period between UI refreshes.
const TICK: Duration = Duration::from_millis(100);
const BORDER: u16 = 2;
const SPINNER_HEIGHT: u16 = 1;

// TODO: generate system prompt based on handbook and any specified solana repo
const SYSTEM_PROMPT: &str = "You are a helpful assistant";

struct PromptWorker {
    rx: mpsc::Receiver<LlmResult<String>>,
}

impl PromptWorker {
    fn start(prompt: String) -> Self {
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || tx.send(llm::execute(SYSTEM_PROMPT, &prompt)));
        Self { rx }
    }

    fn try_recv(&self) -> Option<LlmResult<String>> {
        self.rx.try_recv().ok()
    }
}

fn draw_prompt_and_spinner(f: &mut ratatui::Frame, prompt: &str, throbber_state: &ThrobberState) {
    let mut lines: Vec<_> = prompt
        .lines()
        .map(|line| Line::from(line).fg(Color::Yellow))
        .collect();

    lines.push(Line::from(
        '-'.to_string().repeat((f.area().width - BORDER) as usize),
    ));

    lines.push(
        Throbber::default()
            .throbber_style(Style::default().fg(Color::Blue))
            .throbber_set(BRAILLE_EIGHT_DOUBLE)
            .to_line(throbber_state),
    );

    let widget = Paragraph::new(lines)
        .block(Block::new().padding(Padding::horizontal(1)))
        .wrap(Wrap { trim: true });

    f.render_widget(widget, f.area());
}

fn display_answer(mut terminal: DefaultTerminal, prompt: &str, answer: &str) -> Result<()> {
    let required_height = prompt.lines().count() as u16 + answer.lines().count() as u16 + BORDER;

    terminal.insert_before(required_height, |buf| {
        let mut lines: Vec<_> = prompt
            .lines()
            .map(|line| Line::from(line).fg(Color::Yellow))
            .collect();

        lines.push(Line::from(
            '-'.to_string().repeat((buf.area.width - BORDER) as usize),
        ));

        lines.extend(tui_markdown::from_str(answer).lines);

        let widget = Paragraph::new(lines)
            .block(Block::new().padding(Padding::horizontal(1)))
            .wrap(Wrap { trim: true });

        widget.render(buf.area, buf);
    })?;

    Ok(())
}

fn is_ctrl_c_pressed() -> Result<bool> {
    if !event::poll(Duration::from_millis(1))? {
        return Ok(false);
    }

    match event::read()? {
        event::Event::Key(event::KeyEvent {
            code: event::KeyCode::Char('c'),
            modifiers: event::KeyModifiers::CONTROL,
            ..
        }) => Ok(true),
        _ => Ok(false),
    }
}

pub fn single_prompt(prompt: &str, plain_output: bool) -> Result<()> {
    if plain_output {
        println!("{}", llm::execute(SYSTEM_PROMPT, prompt)?);
        return Ok(());
    }

    let mut throbber_state = ThrobberState::default();
    let worker = PromptWorker::start(prompt.to_owned());

    let mut terminal = ratatui::init_with_options(TerminalOptions {
        viewport: Viewport::Inline(prompt.lines().count() as u16 + BORDER + SPINNER_HEIGHT),
    });

    loop {
        terminal.draw(|f| draw_prompt_and_spinner(f, prompt, &throbber_state))?;

        if let Some(answer) = worker.try_recv().transpose()? {
            display_answer(terminal, prompt, &answer)?;
            break;
        }

        throbber_state.calc_next();

        thread::sleep(TICK);

        if is_ctrl_c_pressed()? {
            break;
        }
    }

    ratatui::restore();

    Ok(())
}
