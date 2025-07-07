use std::{sync::mpsc, thread, time::Duration};

use color_eyre::Result;
use ratatui::{
    crossterm::event,
    layout::{Constraint, Layout},
    style::{Color, Style, Stylize},
    text::Text,
    widgets::{Block, Paragraph, Wrap},
    TerminalOptions, Viewport,
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
    let [prompt_area, throbber_area] = Layout::default()
        .constraints([
            Constraint::Length(prompt.lines().count() as u16 + BORDER),
            Constraint::Length(SPINNER_HEIGHT),
        ])
        .areas(f.area());

    let prompt_widget = Paragraph::new(prompt)
        .block(Block::bordered().title("You").blue())
        .wrap(Wrap { trim: true });

    // convert throbber to a span and prefix with a space to align with prompt text
    let throbber_span = Throbber::default()
        .throbber_style(Style::default().fg(Color::Blue))
        .throbber_set(BRAILLE_EIGHT_DOUBLE)
        .to_symbol_span(throbber_state);
    let mut throbber_text = Text::raw(" ");
    throbber_text.push_span(throbber_span);

    f.render_widget(prompt_widget, prompt_area);
    f.render_widget(throbber_text, throbber_area);
}

fn display_answer(answer: &str) -> Result<()> {
    let mut terminal = ratatui::init_with_options(TerminalOptions {
        viewport: Viewport::Inline(answer.lines().count() as u16),
    });
    terminal.draw(|f| {
        f.render_widget(
            Paragraph::new(tui_markdown::from_str(answer)).wrap(Wrap { trim: true }),
            f.area(),
        )
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
            display_answer(&answer)?;
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
