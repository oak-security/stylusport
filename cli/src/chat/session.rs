use std::{cmp, sync::mpsc, time::Duration};

use color_eyre::Result;
use ratatui::{
    crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style, Stylize},
    text::Line,
    widgets::{Block, Borders, Padding, Paragraph, Wrap},
    Frame,
};
use rayon::{ThreadPool, ThreadPoolBuilder};
use throbber_widgets_tui::{Throbber, ThrobberState, BRAILLE_EIGHT_DOUBLE};
use tui_textarea::TextArea;

use crate::llm;

type LlmResult<T> = Result<T, llm::Error>;

/// Period between UI refreshes.
const TICK: Duration = Duration::from_millis(100);
const PROMPT_SIGIL: &str = "❯ (Ctrl+Space to Execute Prompt)";
const PROMPT_WORKER_POOL_SIZE: usize = 2;
const BORDERS: u16 = 2;
const TITLE_HEIGHT: u16 = 1;

// TODO: generate system prompt based on handbook and any specified solana repo
const SYSTEM_PROMPT: &str = "You are a helpful assistant";

struct PromptWorkerPool {
    pool: ThreadPool,
}

impl PromptWorkerPool {
    fn init() -> Result<Self> {
        let pool = ThreadPoolBuilder::new()
            .num_threads(PROMPT_WORKER_POOL_SIZE)
            .build()?;

        Ok(Self { pool })
    }

    // TODO: Pass previous messages and responses to append to the context
    fn start(&self, prompt: String) -> PromptWorker {
        let (tx, rx) = mpsc::channel();
        self.pool.spawn(move || {
            tx.send(llm::execute(SYSTEM_PROMPT, &prompt)).ok();
        });
        PromptWorker { rx }
    }
}

#[derive(Debug)]
struct PromptWorker {
    rx: mpsc::Receiver<LlmResult<String>>,
}

impl PromptWorker {
    fn try_recv(&self) -> Option<LlmResult<String>> {
        self.rx.try_recv().ok()
    }
}

#[derive(Debug)]
enum ChatStatus {
    Idle,
    Waiting(PromptWorker),
}

impl ChatStatus {
    #[must_use]
    fn is_waiting(&self) -> bool {
        matches!(self, Self::Waiting(..))
    }
}

#[derive(Debug, Clone, Copy)]
enum ChatItemKind {
    User,
    Llm,
    Spinner,
}

impl ChatItemKind {
    #[must_use]
    fn is_spinner(&self) -> bool {
        matches!(self, Self::Spinner)
    }
}

#[derive(Debug, Clone)]
struct ChatItem {
    kind: ChatItemKind,
    message: String,
}

struct SessionCtx {
    prompt_worker_pool: PromptWorkerPool,
    chat_status: ChatStatus,
    chat_list_items: Vec<ChatItem>,
    chat_list_area: Rect,
    chat_list_scroll_y: u16,
    throbber_state: ThrobberState,
    prompt_text_area: TextArea<'static>,
    title_line: String,
}

impl SessionCtx {
    fn init() -> Result<Self> {
        let mut prompt_text_area = TextArea::default();
        prompt_text_area.set_block(
            Block::default()
                .borders(Borders::ALL)
                .title(PROMPT_SIGIL)
                .yellow(),
        );

        let model = llm::model()?;
        let title_line = format!("❯ StylusPort::Chat - {model}");

        Ok(Self {
            prompt_worker_pool: PromptWorkerPool::init()?,
            chat_status: ChatStatus::Idle,
            chat_list_items: vec![],
            chat_list_area: Rect::default(),
            chat_list_scroll_y: 0,
            throbber_state: ThrobberState::default(),
            prompt_text_area,
            title_line,
        })
    }

    fn draw_interface(&mut self, f: &mut Frame) {
        const MIN_HEIGHT: usize = 3;

        let prompt_area_max_height = f.area().height / 2; // 50%

        let prompt_area_height =
            cmp::max(self.prompt_text_area.lines().len(), MIN_HEIGHT) as u16 + BORDERS;
        let prompt_area_height = prompt_area_height.min(prompt_area_max_height);

        let chat_area_height = f.area().height - prompt_area_height - TITLE_HEIGHT;

        let [title_area, chat_area, prompt_area] = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Length(chat_area_height),
                Constraint::Length(prompt_area_height.min(prompt_area_max_height)),
            ])
            .areas(f.area());

        let chat_lines: Vec<Line> = self
            .chat_list_items
            .iter()
            .flat_map(|item| match item.kind {
                ChatItemKind::User => {
                    let mut lines = vec![Line::from("")];
                    lines.extend(
                        item.message
                            .lines()
                            .map(|line| Line::from(line).fg(Color::Yellow)),
                    );
                    lines.push(Line::from(
                        '-'.to_string().repeat((chat_area.width - BORDERS) as usize),
                    ));
                    lines
                }
                ChatItemKind::Llm => tui_markdown::from_str(&item.message).lines,
                ChatItemKind::Spinner => {
                    let throbber_line = Throbber::default()
                        .throbber_style(Style::default().fg(Color::Blue))
                        .throbber_set(BRAILLE_EIGHT_DOUBLE)
                        .to_line(&self.throbber_state);
                    vec![throbber_line]
                }
            })
            .collect();

        let line_count = chat_lines.len() as u16;

        let max_scroll_y = line_count.saturating_sub((chat_area.height / 3) * 2);

        self.chat_list_scroll_y = self.chat_list_scroll_y.min(max_scroll_y);

        f.render_widget(
            Paragraph::new(self.title_line.as_str())
                .fg(Color::Blue)
                .centered(),
            title_area,
        );

        f.render_widget(
            Paragraph::new(chat_lines)
                .block(Block::new().padding(Padding::horizontal(1)))
                .scroll((self.chat_list_scroll_y, 0))
                .wrap(Wrap { trim: true }),
            chat_area,
        );
        f.render_widget(&self.prompt_text_area, prompt_area);

        self.chat_list_area = chat_area;
    }

    fn cancel_spinner(&mut self) {
        assert!(
            self.chat_list_items
                .pop()
                .is_some_and(|i| i.kind.is_spinner()),
            "if the chat status is waiting the last item in the list is always a spinner"
        );
        self.chat_status = ChatStatus::Idle;
    }

    fn send_user_message(&mut self) {
        // cut the message from the prompt input
        self.prompt_text_area.select_all();
        self.prompt_text_area.cut();
        let prompt = self.prompt_text_area.yank_text();
        self.chat_list_items.push(ChatItem {
            kind: ChatItemKind::User,
            message: prompt.clone(),
        });
        self.chat_list_items.push(ChatItem {
            kind: ChatItemKind::Spinner,
            message: String::new(),
        });
        // scroll the chat list to the bottom when a new user message is sent.
        self.chat_list_scroll_y = u16::MAX;
        // execute prompt on a background thread
        let worker = self.prompt_worker_pool.start(prompt);
        self.chat_status = ChatStatus::Waiting(worker);
    }

    // returns true if the session should quit
    fn handle_key_event(&mut self, key_event: KeyEvent) -> Result<bool> {
        match key_event {
            KeyEvent {
                code: KeyCode::Esc, ..
            } if !self.chat_status.is_waiting() => return Ok(true),

            KeyEvent {
                code: KeyCode::Esc, ..
            } => self.cancel_spinner(),

            KeyEvent {
                code: KeyCode::Up, ..
            } => {
                self.chat_list_scroll_y = self.chat_list_scroll_y.saturating_sub(1);
            }

            KeyEvent {
                code: KeyCode::Down,
                ..
            } => {
                self.chat_list_scroll_y = self.chat_list_scroll_y.saturating_add(1);
            }

            KeyEvent {
                code: KeyCode::PageUp,
                ..
            } => {
                self.chat_list_scroll_y = self
                    .chat_list_scroll_y
                    .saturating_sub(self.chat_list_area.height / 2);
            }

            KeyEvent {
                code: KeyCode::PageDown,
                ..
            } => {
                self.chat_list_scroll_y = self
                    .chat_list_scroll_y
                    .saturating_add(self.chat_list_area.height / 2);
            }

            // TODO: currently Ctrl+Space is used to send a prompt to the LLM, ideally this is changed to 'Enter'.
            // However, Shift+Enter does not seem to be captured by crossterm on MacOs, which is required to add a new line
            // if 'Enter' is re-mapped.
            KeyEvent {
                code: KeyCode::Char(' '),
                modifiers: KeyModifiers::CONTROL,
                ..
            } if !self.prompt_text_area.is_empty() && !self.chat_status.is_waiting() => {
                self.send_user_message()
            }

            input => {
                self.prompt_text_area.input(input);
            }
        }

        Ok(false)
    }

    fn check_for_response(&mut self) -> Result<()> {
        let ChatStatus::Waiting(ref worker) = self.chat_status else {
            return Ok(());
        };

        let Some(llm_response) = worker.try_recv().transpose()? else {
            // drive the spinner
            self.throbber_state.calc_next();
            return Ok(());
        };

        self.cancel_spinner();
        self.chat_list_items.push(ChatItem {
            kind: ChatItemKind::Llm,
            message: llm_response,
        });

        Ok(())
    }
}

pub fn session() -> Result<()> {
    let mut terminal = ratatui::init();
    terminal.hide_cursor()?;

    let mut ctx = SessionCtx::init()?;

    loop {
        terminal.draw(|f| ctx.draw_interface(f))?;

        if event::poll(TICK)? {
            if let Event::Key(key_event) = event::read()? {
                if ctx.handle_key_event(key_event)? {
                    break;
                }
            }
        }

        ctx.check_for_response()?
    }

    ratatui::restore();

    Ok(())
}
