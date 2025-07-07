use std::{cmp, io::stdout, sync::mpsc, time::Duration};

use color_eyre::Result;
use ratatui::{
    crossterm::{
        event::{
            self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyModifiers,
            MouseEvent, MouseEventKind,
        },
        execute,
    },
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style, Stylize},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};
use rayon::{ThreadPool, ThreadPoolBuilder};
use throbber_widgets_tui::{Throbber, ThrobberState, BRAILLE_EIGHT_DOUBLE};
use tui_textarea::TextArea;
use tui_widget_list::{ListBuilder, ListState, ListView};

use crate::llm;

type LlmResult<T> = Result<T, llm::Error>;

/// Period between UI refreshes.
const TICK: Duration = Duration::from_millis(100);
const PROMPT_SIGIL: &str = "❯ (Ctrl+Space to Execute Prompt)";
const USER_MSG_ITEM_TITLE: &str = "You";
const PROMPT_WORKER_POOL_SIZE: usize = 2;
const BORDERS: u16 = 2;
const SPINNER_CHAT_ITEM_HEIGHT: u16 = BORDERS + 1; // Borders(2) + Spinner(1)

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
    model: String,
    prompt_worker_pool: PromptWorkerPool,
    chat_status: ChatStatus,
    chat_list_items: Vec<ChatItem>,
    chat_list_area: Rect,
    throbber_state: ThrobberState,
    prompt_text_area: TextArea<'static>,
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

        Ok(Self {
            model: llm::model()?,
            prompt_worker_pool: PromptWorkerPool::init()?,
            chat_status: ChatStatus::Idle,
            chat_list_items: vec![],
            chat_list_area: Rect::default(),
            throbber_state: ThrobberState::default(),
            prompt_text_area,
        })
    }

    fn draw_interface(&mut self, f: &mut Frame, chat_list_state: &mut ListState) {
        const MIN_HEIGHT: usize = 3;

        let max_height = f.area().height / 2; // 50%

        let prompt_area_height =
            cmp::max(self.prompt_text_area.lines().len(), MIN_HEIGHT) as u16 + BORDERS;
        let prompt_area_height = prompt_area_height.min(max_height);

        let chat_area_height = f.area().height - prompt_area_height;

        let [chat_area, prompt_area] = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(chat_area_height),
                Constraint::Length(prompt_area_height.min(max_height)),
            ])
            .areas(f.area());

        let model = self.model.clone();
        let builder = ListBuilder::new(|ctx| {
            let chat_item = self
                .chat_list_items
                .get(ctx.index)
                .expect("list builder provides valid index");

            let min_axis_size = match chat_item.kind {
                ChatItemKind::Spinner => SPINNER_CHAT_ITEM_HEIGHT,
                _ => chat_item.message.lines().count() as u16 + BORDERS,
            };

            let title = match chat_item.kind {
                ChatItemKind::User => USER_MSG_ITEM_TITLE.to_owned(),
                _ => model.clone(),
            };

            let chat_list_widget = match chat_item.kind {
                ChatItemKind::Spinner => {
                    let throbber_line = Throbber::default()
                        .throbber_style(Style::default().fg(Color::Blue))
                        .throbber_set(BRAILLE_EIGHT_DOUBLE)
                        .to_line(&self.throbber_state);
                    Paragraph::new(throbber_line)
                }
                _ => Paragraph::new(tui_markdown::from_str(&chat_item.message))
                    .wrap(Wrap { trim: true }),
            }
            .block(Block::bordered().title(title));

            let chat_list_widget = if ctx.is_selected {
                chat_list_widget.blue()
            } else {
                chat_list_widget
            };

            (chat_list_widget, min_axis_size)
        });

        let chat_list = ListView::new(builder, self.chat_list_items.len())
            .infinite_scrolling(false)
            .block(Block::bordered().title("StylusPort::Chat"));

        f.render_stateful_widget(chat_list, chat_area, chat_list_state);
        f.render_widget(&self.prompt_text_area, prompt_area);

        self.chat_list_area = chat_area;
    }

    fn handle_mouse_event(&mut self, mouse_event: MouseEvent, chat_list_state: &mut ListState) {
        if mouse_event.row < self.chat_list_area.bottom() {
            match mouse_event.kind {
                MouseEventKind::ScrollDown => chat_list_state.next(),
                MouseEventKind::ScrollUp => chat_list_state.previous(),
                _ => {}
            }
        }
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

    fn send_user_message(&mut self, chat_list_state: &mut ListState) {
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
        chat_list_state.select(Some(self.chat_list_items.len() - 1));
        // execute prompt on a background thread
        let worker = self.prompt_worker_pool.start(prompt);
        self.chat_status = ChatStatus::Waiting(worker);
    }

    // returns true if the session should quit
    fn handle_key_event(
        &mut self,
        key_event: KeyEvent,
        chat_list_state: &mut ListState,
    ) -> Result<bool> {
        match key_event {
            KeyEvent {
                code: KeyCode::Esc, ..
            } if !self.chat_status.is_waiting() => return Ok(true),

            KeyEvent {
                code: KeyCode::Esc, ..
            } => self.cancel_spinner(),

            // TODO: currently Ctrl+Space is used to send a prompt to the LLM, ideally this is changed to 'Enter'.
            // However, Shift+Enter does not seem to be captured by crossterm on MacOs, which is required to add a new line
            // if 'Enter' is re-mapped.
            KeyEvent {
                code: KeyCode::Char(' '),
                modifiers: KeyModifiers::CONTROL,
                ..
            } if !self.prompt_text_area.is_empty() && !self.chat_status.is_waiting() => {
                self.send_user_message(chat_list_state)
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
    execute!(stdout(), EnableMouseCapture)?;
    terminal.hide_cursor()?;

    let mut ctx = SessionCtx::init()?;

    let mut chat_list_state = ListState::default();

    loop {
        terminal.draw(|f| ctx.draw_interface(f, &mut chat_list_state))?;

        if event::poll(TICK)? {
            match event::read()? {
                Event::Mouse(mouse_event) => {
                    ctx.handle_mouse_event(mouse_event, &mut chat_list_state);
                }

                Event::Key(key_event) => {
                    if ctx.handle_key_event(key_event, &mut chat_list_state)? {
                        break;
                    }
                }

                _ => {}
            }
        }

        ctx.check_for_response()?
    }

    ratatui::restore();

    execute!(stdout(), DisableMouseCapture)?;

    Ok(())
}
