//! Main TUI application for the prompt generator.

use std::io::{self, Stdout};
use std::path::PathBuf;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame, Terminal,
};

use codeloops_agent::{Agent, AgentConfig};

use crate::prompt::{
    protocol::{AgentMessage, InputType, SelectOption, UserResponse},
    session::{InterviewSession, PromptDraft},
    system_prompt::{build_continuation_prompt, build_system_prompt},
};

use super::layout::{DraftLayout, InterviewLayout, LayoutMode, MainLayout, PanelFocus};
use super::widgets::{
    ConfirmInput, DraftWidget, MultiSelectInput, QuestionWidget, SelectInput, TextInput,
};

/// The current focus area in the TUI
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Question,
    Draft,
}

/// The current input state
#[derive(Debug, Clone)]
pub enum InputState {
    /// No active input
    Idle,
    /// Text input mode
    TextInput { value: String, cursor: usize },
    /// Single select mode
    SelectInput {
        options: Vec<SelectOption>,
        selected: usize,
    },
    /// Multi-select mode
    MultiSelectInput {
        options: Vec<SelectOption>,
        selected: Vec<bool>,
        cursor: usize,
    },
    /// Confirm (yes/no) mode
    ConfirmInput {
        selected: bool, // true = yes
    },
    /// Editor mode - displays prompt to launch editor
    EditorPending,
    /// Waiting for agent response
    WaitingForAgent { message: String },
}

/// Current question being displayed
#[derive(Debug, Clone)]
pub struct CurrentQuestion {
    pub text: String,
    pub context: Option<String>,
    pub input_type: InputType,
    pub options: Vec<SelectOption>,
    pub section: Option<String>,
}

/// Agent timeout duration (120 seconds)
const AGENT_TIMEOUT: Duration = Duration::from_secs(120);

/// Maximum number of retries for agent calls
const MAX_AGENT_RETRIES: usize = 3;

/// Status message display duration (5 seconds)
const STATUS_DURATION: Duration = Duration::from_secs(5);

/// The main TUI application
pub struct App {
    /// The interview session
    session: InterviewSession,
    /// The agent being used
    agent: Box<dyn Agent>,
    /// Agent configuration
    agent_config: AgentConfig,
    /// Current focus area
    focus: Focus,
    /// Current input state
    input_state: InputState,
    /// Current question (if any)
    current_question: Option<CurrentQuestion>,
    /// Draft scroll position
    draft_scroll: u16,
    /// Whether the app is still running
    running: bool,
    /// Error message to display
    error_message: Option<String>,
    /// Status message with timestamp for auto-expiration
    status_message: Option<(String, Instant)>,
    /// Terminal instance
    terminal: Terminal<CrosstermBackend<Stdout>>,
    /// Flag indicating agent needs to continue after DraftUpdate
    needs_continuation: bool,
    /// When agent execution started (for elapsed time display)
    agent_start_time: Option<Instant>,
    /// Flag to cancel ongoing agent request
    cancel_requested: bool,
    /// Which panel to show in single-panel mode
    panel_focus: PanelFocus,
}

impl App {
    /// Create a new TUI application
    pub fn new(
        session: InterviewSession,
        agent: Box<dyn Agent>,
        agent_config: AgentConfig,
    ) -> Result<Self> {
        // Setup terminal
        enable_raw_mode().context("Failed to enable raw mode")?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen).context("Failed to enter alternate screen")?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend).context("Failed to create terminal")?;

        Ok(Self {
            session,
            agent,
            agent_config,
            focus: Focus::Question,
            input_state: InputState::Idle,
            current_question: None,
            draft_scroll: 0,
            running: true,
            error_message: None,
            status_message: None,
            terminal,
            needs_continuation: false,
            agent_start_time: None,
            cancel_requested: false,
            panel_focus: PanelFocus::default(),
        })
    }

    /// Get the session file path
    pub fn session_path(&self) -> Option<PathBuf> {
        self.session.session_path().ok()
    }

    /// Run the TUI application
    pub async fn run(&mut self) -> Result<Option<PromptDraft>> {
        // Start the interview if this is a new session
        if self.session.history.is_empty() {
            self.start_interview().await?;
        } else {
            // Resume - send continuation to get next question
            self.continue_interview().await?;
        }

        // Main event loop
        while self.running {
            // Check if we need to continue after a DraftUpdate (Task 1.2 fix)
            if self.needs_continuation {
                self.needs_continuation = false;
                self.continue_interview().await?;
            }

            // Draw the UI
            self.draw()?;

            // Handle events with a timeout so we can process async tasks
            if event::poll(Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    self.handle_key_event(key).await?;
                }
            }
        }

        // Cleanup terminal
        self.cleanup_terminal()?;

        // Return the draft if complete
        if self.session.is_complete {
            Ok(Some(self.session.draft.clone()))
        } else {
            // Save session for later
            if let Err(e) = self.session.save() {
                eprintln!("Warning: Failed to save session: {}", e);
            }
            Ok(None)
        }
    }

    /// Cleanup the terminal
    fn cleanup_terminal(&mut self) -> Result<()> {
        disable_raw_mode().context("Failed to disable raw mode")?;
        execute!(self.terminal.backend_mut(), LeaveAlternateScreen)
            .context("Failed to leave alternate screen")?;
        self.terminal
            .show_cursor()
            .context("Failed to show cursor")?;
        Ok(())
    }

    /// Draw the UI
    fn draw(&mut self) -> Result<()> {
        // Clear expired status messages
        if let Some((_, timestamp)) = &self.status_message {
            if timestamp.elapsed() > STATUS_DURATION {
                self.status_message = None;
            }
        }

        // Extract render state to avoid borrow issues
        let render_state = RenderState {
            agent_name: self.agent.name().to_string(),
            session_id: self.session.id.clone(),
            draft: self.session.draft.clone(),
            focus: self.focus,
            input_state: self.input_state.clone(),
            current_question: self.current_question.clone(),
            draft_scroll: self.draft_scroll,
            error_message: self.error_message.clone(),
            status_message: self.status_message.as_ref().map(|(msg, _)| msg.clone()),
            agent_start_time: self.agent_start_time,
            panel_focus: self.panel_focus,
        };

        self.terminal.draw(|frame| {
            render_state.render(frame);
        })?;
        Ok(())
    }
}

/// State needed for rendering (to avoid borrow issues)
struct RenderState {
    agent_name: String,
    session_id: String,
    draft: PromptDraft,
    focus: Focus,
    input_state: InputState,
    current_question: Option<CurrentQuestion>,
    draft_scroll: u16,
    error_message: Option<String>,
    status_message: Option<String>,
    agent_start_time: Option<Instant>,
    panel_focus: PanelFocus,
}

impl RenderState {
    fn render(&self, frame: &mut Frame) {
        let area = frame.area();
        let layout = MainLayout::new(area);

        // Render header
        self.render_header(frame, layout.header);

        // Render panels based on layout mode
        match layout.mode {
            LayoutMode::DualPanel => {
                // Render both panels side by side
                self.render_interview_panel(frame, layout.left_panel);
                self.render_draft_panel(frame, layout.right_panel);
            }
            LayoutMode::SinglePanel => {
                // Render only the focused panel
                match self.panel_focus {
                    PanelFocus::Interview => {
                        self.render_interview_panel(frame, layout.left_panel);
                    }
                    PanelFocus::Draft => {
                        self.render_draft_panel(frame, layout.right_panel);
                    }
                }
            }
        }

        // Render footer
        self.render_footer(frame, layout.footer);
    }

    fn render_header(&self, frame: &mut Frame, area: Rect) {
        let title = Line::from(vec![
            Span::styled(
                " codeloops prompt ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("| "),
            Span::styled(
                format!("Agent: {} ", &self.agent_name),
                Style::default().fg(Color::Yellow),
            ),
            Span::raw("| "),
            Span::styled(
                format!(
                    "Session: {} ",
                    &self.session_id[..20.min(self.session_id.len())]
                ),
                Style::default().fg(Color::DarkGray),
            ),
        ]);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Blue));

        let header = Paragraph::new(title).block(block);
        frame.render_widget(header, area);
    }

    fn render_interview_panel(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Blue))
            .title(" Interview ");

        let inner_area = block.inner(area);
        frame.render_widget(block, area);

        let interview_layout = InterviewLayout::new(inner_area);

        // Render question or status
        match &self.input_state {
            InputState::WaitingForAgent { message } => {
                // Show elapsed time if we're tracking agent start
                let elapsed_str = if let Some(start) = self.agent_start_time {
                    let elapsed = start.elapsed().as_secs();
                    format!(" ({}s)", elapsed)
                } else {
                    String::new()
                };

                let thinking = Paragraph::new(vec![
                    Line::from(Span::styled(
                        format!("⏳ Thinking...{}", elapsed_str),
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    )),
                    Line::from(""),
                    Line::from(Span::styled(
                        message.as_str(),
                        Style::default().fg(Color::DarkGray),
                    )),
                    Line::from(""),
                    Line::from(Span::styled(
                        "Press Esc to cancel",
                        Style::default().fg(Color::DarkGray),
                    )),
                ])
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::Yellow)),
                );
                frame.render_widget(thinking, interview_layout.question);
            }
            _ => {
                if let Some(ref question) = self.current_question {
                    let q = QuestionWidget::new(&question.text)
                        .context(question.context.as_deref())
                        .section(question.section.as_deref());
                    frame.render_widget(q, interview_layout.question);
                } else {
                    let placeholder = Paragraph::new(Span::styled(
                        "Waiting for question...",
                        Style::default().fg(Color::DarkGray),
                    ))
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .border_style(Style::default().fg(Color::DarkGray)),
                    );
                    frame.render_widget(placeholder, interview_layout.question);
                }
            }
        }

        // Render input area based on state
        match &self.input_state {
            InputState::Idle => {
                let placeholder = Paragraph::new(Span::styled(
                    "Press Enter to start typing...",
                    Style::default().fg(Color::DarkGray),
                ))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::DarkGray)),
                );
                frame.render_widget(placeholder, interview_layout.input);
            }
            InputState::TextInput { value, cursor } => {
                let input = TextInput::new(value, *cursor).focused(self.focus == Focus::Question);
                frame.render_widget(input, interview_layout.input);
            }
            InputState::SelectInput { options, selected } => {
                let input =
                    SelectInput::new(options, *selected).focused(self.focus == Focus::Question);
                frame.render_widget(input, interview_layout.input);
            }
            InputState::MultiSelectInput {
                options,
                selected,
                cursor,
            } => {
                let input = MultiSelectInput::new(options, selected, *cursor)
                    .focused(self.focus == Focus::Question);
                frame.render_widget(input, interview_layout.input);
            }
            InputState::ConfirmInput { selected } => {
                let input = ConfirmInput::new(*selected).focused(self.focus == Focus::Question);
                frame.render_widget(input, interview_layout.input);
            }
            InputState::EditorPending => {
                // Show editor launch prompt
                let editor_prompt = Paragraph::new(vec![
                    Line::from(Span::styled(
                        "Press Enter to open editor",
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    )),
                    Line::from(Span::styled(
                        "(or Esc to cancel)",
                        Style::default().fg(Color::DarkGray),
                    )),
                ])
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::Cyan)),
                );
                frame.render_widget(editor_prompt, interview_layout.input);
            }
            InputState::WaitingForAgent { .. } => {
                // Show disabled input
                let placeholder = Paragraph::new(Span::styled(
                    "Waiting for response...",
                    Style::default().fg(Color::DarkGray),
                ))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::DarkGray)),
                );
                frame.render_widget(placeholder, interview_layout.input);
            }
        }

        // Render status line
        let status = if let Some(ref err) = self.error_message {
            Line::from(Span::styled(
                format!("Error: {}", err),
                Style::default().fg(Color::Red),
            ))
        } else if let Some(ref msg) = self.status_message {
            Line::from(Span::styled(
                msg.as_str(),
                Style::default().fg(Color::Green),
            ))
        } else {
            Line::from(Span::styled(
                "Answer the questions to build your prompt",
                Style::default().fg(Color::DarkGray),
            ))
        };
        frame.render_widget(Paragraph::new(status), interview_layout.status);
    }

    fn render_draft_panel(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Magenta))
            .title(" Draft Preview ");

        let inner_area = block.inner(area);
        frame.render_widget(block, area);

        let draft_layout = DraftLayout::new(inner_area);

        // Title (Task 3.4: removed progress percentage - it was misleading)
        let title = Line::from(Span::styled("prompt.md", Style::default().fg(Color::White)));
        frame.render_widget(Paragraph::new(title), draft_layout.title);

        // Draft content
        let draft = DraftWidget::new(&self.draft).scroll(self.draft_scroll);
        frame.render_widget(draft, draft_layout.content);
    }

    fn render_footer(&self, frame: &mut Frame, area: Rect) {
        let shortcuts = Line::from(vec![
            Span::styled(
                " Enter ",
                Style::default().bg(Color::DarkGray).fg(Color::White),
            ),
            Span::raw(" Submit "),
            Span::styled(
                " Tab ",
                Style::default().bg(Color::DarkGray).fg(Color::White),
            ),
            Span::raw(" Switch panel "),
            Span::styled(
                " Ctrl+S ",
                Style::default().bg(Color::DarkGray).fg(Color::White),
            ),
            Span::raw(" Save & Exit "),
            Span::styled(
                " Ctrl+C ",
                Style::default().bg(Color::DarkGray).fg(Color::White),
            ),
            Span::raw(" Quit "),
            Span::styled(
                " ↑↓ ",
                Style::default().bg(Color::DarkGray).fg(Color::White),
            ),
            Span::raw(" Scroll "),
        ]);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray));

        let footer = Paragraph::new(shortcuts).block(block);
        frame.render_widget(footer, area);
    }
}

impl App {
    /// Handle a key event
    async fn handle_key_event(&mut self, key: KeyEvent) -> Result<()> {
        // Clear error on any key press
        self.error_message = None;

        // Global shortcuts
        match (key.modifiers, key.code) {
            (KeyModifiers::CONTROL, KeyCode::Char('c')) => {
                // Save and quit
                self.save_and_quit().await?;
                return Ok(());
            }
            (KeyModifiers::CONTROL, KeyCode::Char('s')) => {
                // Save session
                self.save_session()?;
                self.status_message = Some(("Session saved!".to_string(), Instant::now()));
                return Ok(());
            }
            (KeyModifiers::NONE, KeyCode::Tab) => {
                // Switch focus between interview and draft
                self.focus = match self.focus {
                    Focus::Question => Focus::Draft,
                    Focus::Draft => Focus::Question,
                };
                // In single panel mode, also toggle which panel is visible
                self.panel_focus = match self.panel_focus {
                    PanelFocus::Interview => PanelFocus::Draft,
                    PanelFocus::Draft => PanelFocus::Interview,
                };
                return Ok(());
            }
            _ => {}
        }

        // Handle based on focus
        match self.focus {
            Focus::Question => self.handle_question_input(key).await?,
            Focus::Draft => self.handle_draft_input(key)?,
        }

        Ok(())
    }

    /// Handle input in the question/interview area
    async fn handle_question_input(&mut self, key: KeyEvent) -> Result<()> {
        match &mut self.input_state {
            InputState::Idle => {
                if key.code == KeyCode::Enter {
                    // Initialize text input if we have a question
                    if self.current_question.is_some() {
                        let q = self.current_question.as_ref().unwrap();
                        match q.input_type {
                            InputType::Text => {
                                self.input_state = InputState::TextInput {
                                    value: String::new(),
                                    cursor: 0,
                                };
                            }
                            InputType::Editor => {
                                // Set pending state and launch editor
                                self.input_state = InputState::EditorPending;
                            }
                            InputType::Select => {
                                self.input_state = InputState::SelectInput {
                                    options: q.options.clone(),
                                    selected: 0,
                                };
                            }
                            InputType::MultiSelect => {
                                let len = q.options.len();
                                self.input_state = InputState::MultiSelectInput {
                                    options: q.options.clone(),
                                    selected: vec![false; len],
                                    cursor: 0,
                                };
                            }
                            InputType::Confirm => {
                                self.input_state = InputState::ConfirmInput { selected: true };
                            }
                        }
                    }
                }
            }
            InputState::EditorPending => {
                if key.code == KeyCode::Enter {
                    // Launch external editor
                    let content = self.launch_external_editor().await?;
                    if !content.is_empty() {
                        let response = UserResponse::text(content);
                        self.submit_response(response).await?;
                    } else {
                        // User cancelled or empty content
                        self.input_state = InputState::EditorPending;
                        self.status_message =
                            Some(("Editor returned empty content".to_string(), Instant::now()));
                    }
                } else if key.code == KeyCode::Esc {
                    self.input_state = InputState::Idle;
                }
            }
            InputState::TextInput { value, cursor } => {
                match key.code {
                    KeyCode::Enter => {
                        // Submit the response
                        let response = UserResponse::text(value.clone());
                        self.submit_response(response).await?;
                    }
                    KeyCode::Char(c) => {
                        value.insert(*cursor, c);
                        *cursor += 1;
                    }
                    KeyCode::Backspace => {
                        if *cursor > 0 {
                            *cursor -= 1;
                            value.remove(*cursor);
                        }
                    }
                    KeyCode::Delete => {
                        if *cursor < value.len() {
                            value.remove(*cursor);
                        }
                    }
                    KeyCode::Left => {
                        if *cursor > 0 {
                            *cursor -= 1;
                        }
                    }
                    KeyCode::Right => {
                        if *cursor < value.len() {
                            *cursor += 1;
                        }
                    }
                    KeyCode::Home => {
                        *cursor = 0;
                    }
                    KeyCode::End => {
                        *cursor = value.len();
                    }
                    KeyCode::Esc => {
                        self.input_state = InputState::Idle;
                    }
                    _ => {}
                }
            }
            InputState::SelectInput { options, selected } => match key.code {
                KeyCode::Up => {
                    if *selected > 0 {
                        *selected -= 1;
                    }
                }
                KeyCode::Down => {
                    if *selected < options.len() - 1 {
                        *selected += 1;
                    }
                }
                KeyCode::Enter => {
                    let value = options[*selected].value.clone();
                    let response = UserResponse::selection(value);
                    self.submit_response(response).await?;
                }
                KeyCode::Esc => {
                    self.input_state = InputState::Idle;
                }
                _ => {}
            },
            InputState::MultiSelectInput {
                options,
                selected,
                cursor,
            } => match key.code {
                KeyCode::Up => {
                    if *cursor > 0 {
                        *cursor -= 1;
                    }
                }
                KeyCode::Down => {
                    if *cursor < options.len() - 1 {
                        *cursor += 1;
                    }
                }
                KeyCode::Char(' ') => {
                    // Toggle selection
                    if *cursor < selected.len() {
                        selected[*cursor] = !selected[*cursor];
                    }
                }
                KeyCode::Enter => {
                    // Submit selected items
                    let selections: Vec<String> = options
                        .iter()
                        .zip(selected.iter())
                        .filter(|(_, &s)| s)
                        .map(|(o, _)| o.value.clone())
                        .collect();
                    let response = UserResponse::multi_selection(selections);
                    self.submit_response(response).await?;
                }
                KeyCode::Esc => {
                    self.input_state = InputState::Idle;
                }
                _ => {}
            },
            InputState::ConfirmInput { selected } => match key.code {
                KeyCode::Left | KeyCode::Right => {
                    *selected = !*selected;
                }
                KeyCode::Char('y') | KeyCode::Char('Y') => {
                    *selected = true;
                }
                KeyCode::Char('n') | KeyCode::Char('N') => {
                    *selected = false;
                }
                KeyCode::Enter => {
                    let response = UserResponse::confirm(*selected);
                    self.submit_response(response).await?;
                }
                KeyCode::Esc => {
                    self.input_state = InputState::Idle;
                }
                _ => {}
            },
            InputState::WaitingForAgent { .. } => {
                // Allow Esc to cancel the agent request (Task 2.6)
                if key.code == KeyCode::Esc {
                    self.cancel_requested = true;
                    self.status_message =
                        Some(("Cancelling request...".to_string(), Instant::now()));
                }
            }
        }

        Ok(())
    }

    /// Handle input in the draft preview area
    fn handle_draft_input(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Up => {
                if self.draft_scroll > 0 {
                    self.draft_scroll -= 1;
                }
            }
            KeyCode::Down => {
                self.draft_scroll += 1;
            }
            KeyCode::PageUp => {
                self.draft_scroll = self.draft_scroll.saturating_sub(10);
            }
            KeyCode::PageDown => {
                self.draft_scroll += 10;
            }
            KeyCode::Home => {
                self.draft_scroll = 0;
            }
            _ => {}
        }
        Ok(())
    }

    /// Execute an agent call with timeout and retry logic.
    ///
    /// This wraps agent.execute() with:
    /// - A timeout of AGENT_TIMEOUT seconds
    /// - Exponential backoff retry (up to MAX_AGENT_RETRIES attempts)
    /// - Elapsed time tracking for UI display
    async fn execute_agent_with_retry(&mut self, prompt: &str) -> Result<String> {
        self.agent_start_time = Some(Instant::now());

        for attempt in 0..MAX_AGENT_RETRIES {
            if attempt > 0 {
                // Exponential backoff: 1s, 2s, 4s
                let delay = Duration::from_millis(1000 * 2u64.pow(attempt as u32 - 1));
                self.status_message = Some((
                    format!("Retry {} of {}...", attempt, MAX_AGENT_RETRIES - 1),
                    Instant::now(),
                ));
                self.draw()?;
                tokio::time::sleep(delay).await;
            }

            // Check for cancellation before each attempt
            if self.cancel_requested {
                self.cancel_requested = false;
                self.agent_start_time = None;
                return Err(anyhow::anyhow!("Request cancelled by user"));
            }

            match tokio::time::timeout(
                AGENT_TIMEOUT,
                self.agent.execute(prompt, &self.agent_config),
            )
            .await
            {
                Ok(Ok(output)) => {
                    self.agent_start_time = None;
                    return Ok(output.stdout);
                }
                Ok(Err(e)) if attempt == MAX_AGENT_RETRIES - 1 => {
                    self.agent_start_time = None;
                    return Err(e.into());
                }
                Ok(Err(_)) => continue, // Retry on agent error
                Err(_) if attempt == MAX_AGENT_RETRIES - 1 => {
                    self.agent_start_time = None;
                    return Err(anyhow::anyhow!(
                        "Agent timed out after {} seconds",
                        AGENT_TIMEOUT.as_secs()
                    ));
                }
                Err(_) => continue, // Retry on timeout
            }
        }

        unreachable!()
    }

    /// Start a new interview
    async fn start_interview(&mut self) -> Result<()> {
        self.input_state = InputState::WaitingForAgent {
            message: "Starting interview...".to_string(),
        };
        self.draw()?;

        // Build the system prompt with project context
        let system_prompt = build_system_prompt(&self.session.project_context);

        // Send to agent with timeout and retry
        let output = self
            .execute_agent_with_retry(&system_prompt)
            .await
            .context("Failed to start interview")?;

        // Parse the response
        self.handle_agent_output(&output)?;

        Ok(())
    }

    /// Continue an existing interview
    async fn continue_interview(&mut self) -> Result<()> {
        self.input_state = InputState::WaitingForAgent {
            message: "Resuming interview...".to_string(),
        };
        self.draw()?;

        // Build continuation prompt
        let system_prompt = build_system_prompt(&self.session.project_context);
        let history = self.session.history_for_prompt();
        let draft_md = self.session.draft.to_markdown();
        let continuation = build_continuation_prompt(&history, &draft_md);

        let full_prompt = format!("{}\n\n{}", system_prompt, continuation);

        // Send to agent with timeout and retry
        let output = self
            .execute_agent_with_retry(&full_prompt)
            .await
            .context("Failed to continue interview")?;

        // Parse the response
        self.handle_agent_output(&output)?;

        Ok(())
    }

    /// Submit a response to the agent
    async fn submit_response(&mut self, response: UserResponse) -> Result<()> {
        // Add to history
        self.session.add_user_response(response.clone());

        // Show waiting state
        self.input_state = InputState::WaitingForAgent {
            message: "Processing your answer...".to_string(),
        };
        self.current_question = None;
        self.draw()?;

        // Build the prompt with conversation so far
        let system_prompt = build_system_prompt(&self.session.project_context);
        let history = self.session.history_for_prompt();
        let draft_md = self.session.draft.to_markdown();
        let continuation = build_continuation_prompt(&history, &draft_md);

        let full_prompt = format!("{}\n\n{}", system_prompt, continuation);

        // Send to agent with timeout and retry
        let output = self
            .execute_agent_with_retry(&full_prompt)
            .await
            .context("Failed to get agent response")?;

        // Parse the response
        self.handle_agent_output(&output)?;

        Ok(())
    }

    /// Handle agent output and parse JSON messages.
    ///
    /// This function parses ALL JSON messages in the output (agents may send
    /// multiple messages like DraftUpdate + Question in one response) and
    /// processes them in order.
    fn handle_agent_output(&mut self, output: &str) -> Result<()> {
        // Parse all JSON messages from the output
        let messages = self.parse_all_agent_messages(output)?;

        // Process each message
        let total = messages.len();
        for (idx, message) in messages.into_iter().enumerate() {
            let is_last = idx == total - 1;

            // Add to history
            self.session.add_agent_message(message.clone());

            // Handle the message
            self.handle_agent_message(message, is_last)?;
        }

        Ok(())
    }

    /// Parse ALL agent messages from output.
    ///
    /// Agents may output multiple JSON objects in a single response (e.g.,
    /// DraftUpdate followed by Question). This function finds and parses
    /// all of them using bracket-matching.
    fn parse_all_agent_messages(&self, output: &str) -> Result<Vec<AgentMessage>> {
        let mut messages = Vec::new();

        // First, try to parse the whole thing as a single JSON object
        if let Ok(msg) = serde_json::from_str::<AgentMessage>(output.trim()) {
            return Ok(vec![msg]);
        }

        // Find all JSON objects using bracket matching
        let mut depth = 0;
        let mut start = None;

        for (i, c) in output.char_indices() {
            match c {
                '{' => {
                    if depth == 0 {
                        start = Some(i);
                    }
                    depth += 1;
                }
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        if let Some(s) = start {
                            let json_str = &output[s..=i];
                            if let Ok(msg) = serde_json::from_str::<AgentMessage>(json_str) {
                                messages.push(msg);
                            }
                        }
                        start = None;
                    }
                }
                _ => {}
            }
        }

        // If we couldn't parse any JSON, create a fallback question
        // This handles the case where the agent outputs plain text
        if messages.is_empty() {
            messages.push(AgentMessage::Question {
                text: output.trim().to_string(),
                context: Some("(Agent response was not in expected format)".to_string()),
                input_type: InputType::Text,
                options: vec![],
                section: None,
            });
        }

        Ok(messages)
    }

    /// Handle a parsed agent message.
    ///
    /// `is_last` indicates whether this is the last message in the current batch.
    /// For DraftUpdate messages, if it's the last message, we need to request
    /// continuation from the agent (otherwise the interview freezes).
    fn handle_agent_message(&mut self, message: AgentMessage, is_last: bool) -> Result<()> {
        match message {
            AgentMessage::Question {
                text,
                context,
                input_type,
                options,
                section,
            } => {
                self.current_question = Some(CurrentQuestion {
                    text,
                    context,
                    input_type: input_type.clone(),
                    options: options.clone(),
                    section,
                });

                // Set up appropriate input state, with empty options fallback (Task 1.3)
                self.setup_input_state_for_type(&input_type, options);
            }
            AgentMessage::DraftUpdate {
                section,
                content,
                append,
            } => {
                // Update the draft
                self.session.apply_draft_update(&section, &content, append);
                self.status_message = Some((format!("Updated: {}", section), Instant::now()));

                // If this is the last message and it's a DraftUpdate, we need to
                // continue the interview - the agent should have sent a follow-up
                // question but didn't (Task 1.2 fix)
                if is_last {
                    self.needs_continuation = true;
                }
                self.input_state = InputState::Idle;
            }
            AgentMessage::Thinking { message } => {
                self.input_state = InputState::WaitingForAgent { message };
            }
            AgentMessage::Clarification {
                text,
                original_answer,
                input_type,
                options,
            } => {
                let context = Some(format!("Your previous answer: \"{}\"", original_answer));
                self.current_question = Some(CurrentQuestion {
                    text,
                    context,
                    input_type: input_type.clone(),
                    options: options.clone(),
                    section: None,
                });

                // Set up appropriate input state, with empty options fallback (Task 1.3)
                self.setup_input_state_for_type(&input_type, options);
            }
            AgentMessage::DraftComplete { summary } => {
                self.session.mark_complete();
                self.status_message = Some((summary, Instant::now()));
                self.running = false;
            }
            AgentMessage::Error { message } => {
                self.error_message = Some(message);
                self.input_state = InputState::Idle;
            }
        }

        Ok(())
    }

    /// Set up the appropriate input state for a given input type.
    ///
    /// Handles empty options gracefully by falling back to text input (Task 1.3).
    fn setup_input_state_for_type(&mut self, input_type: &InputType, options: Vec<SelectOption>) {
        match input_type {
            InputType::Text => {
                self.input_state = InputState::TextInput {
                    value: String::new(),
                    cursor: 0,
                };
            }
            InputType::Editor => {
                self.input_state = InputState::EditorPending;
            }
            InputType::Select => {
                // Task 1.3: Handle empty options gracefully
                if options.is_empty() {
                    self.status_message = Some((
                        "No options provided - using text input".to_string(),
                        Instant::now(),
                    ));
                    self.input_state = InputState::TextInput {
                        value: String::new(),
                        cursor: 0,
                    };
                } else {
                    self.input_state = InputState::SelectInput {
                        options,
                        selected: 0,
                    };
                }
            }
            InputType::MultiSelect => {
                // Task 1.3: Handle empty options gracefully
                if options.is_empty() {
                    self.status_message = Some((
                        "No options provided - using text input".to_string(),
                        Instant::now(),
                    ));
                    self.input_state = InputState::TextInput {
                        value: String::new(),
                        cursor: 0,
                    };
                } else {
                    let len = options.len();
                    self.input_state = InputState::MultiSelectInput {
                        options,
                        selected: vec![false; len],
                        cursor: 0,
                    };
                }
            }
            InputType::Confirm => {
                self.input_state = InputState::ConfirmInput { selected: true };
            }
        }
    }

    /// Save the session
    fn save_session(&mut self) -> Result<()> {
        self.session.save()?;
        Ok(())
    }

    /// Save session and quit
    async fn save_and_quit(&mut self) -> Result<()> {
        self.save_session()?;
        self.running = false;
        Ok(())
    }

    /// Launch an external editor for multi-line input.
    ///
    /// Temporarily exits the TUI, spawns the editor with a temp file,
    /// waits for the editor to close, and reads back the content.
    async fn launch_external_editor(&mut self) -> Result<String> {
        use std::io::Write;

        // Create a temporary file
        let temp_dir = std::env::temp_dir();
        let temp_path = temp_dir.join(format!("codeloops_editor_{}.md", std::process::id()));

        // Write initial content (empty or with context)
        let mut file = std::fs::File::create(&temp_path)?;
        if let Some(ref q) = self.current_question {
            writeln!(file, "# {}", q.text)?;
            if let Some(ref ctx) = q.context {
                writeln!(file, "# Context: {}", ctx)?;
            }
            writeln!(
                file,
                "# Delete these comment lines and write your answer below:"
            )?;
            writeln!(file)?;
        }
        drop(file);

        // Temporarily exit TUI mode
        self.cleanup_terminal()?;

        // Determine editor to use
        let editor = std::env::var("EDITOR")
            .or_else(|_| std::env::var("VISUAL"))
            .unwrap_or_else(|_| "vi".to_string());

        // Spawn editor
        let status = std::process::Command::new(&editor)
            .arg(&temp_path)
            .status()
            .context(format!("Failed to launch editor: {}", editor))?;

        // Restore TUI mode
        enable_raw_mode().context("Failed to re-enable raw mode")?;
        execute!(self.terminal.backend_mut(), EnterAlternateScreen)
            .context("Failed to re-enter alternate screen")?;
        self.terminal.clear()?;

        // Check if editor exited successfully
        if !status.success() {
            // Clean up temp file
            let _ = std::fs::remove_file(&temp_path);
            return Ok(String::new());
        }

        // Read the content back
        let content = std::fs::read_to_string(&temp_path).unwrap_or_default();

        // Clean up temp file
        let _ = std::fs::remove_file(&temp_path);

        // Filter out comment lines (starting with #)
        let filtered: Vec<&str> = content
            .lines()
            .filter(|line| !line.starts_with('#'))
            .collect();

        Ok(filtered.join("\n").trim().to_string())
    }
}

impl Drop for App {
    fn drop(&mut self) {
        // Try to restore terminal state
        let _ = disable_raw_mode();
        let _ = execute!(self.terminal.backend_mut(), LeaveAlternateScreen);
        let _ = self.terminal.show_cursor();
    }
}

/// Parse all JSON objects from output (standalone function for testing)
#[cfg(test)]
fn parse_all_json_objects(output: &str) -> Vec<AgentMessage> {
    let mut messages = Vec::new();

    // First, try to parse the whole thing as a single JSON object
    if let Ok(msg) = serde_json::from_str::<AgentMessage>(output.trim()) {
        return vec![msg];
    }

    // Find all JSON objects using bracket matching
    let mut depth = 0;
    let mut start = None;

    for (i, c) in output.char_indices() {
        match c {
            '{' => {
                if depth == 0 {
                    start = Some(i);
                }
                depth += 1;
            }
            '}' => {
                depth -= 1;
                if depth == 0 {
                    if let Some(s) = start {
                        let json_str = &output[s..=i];
                        if let Ok(msg) = serde_json::from_str::<AgentMessage>(json_str) {
                            messages.push(msg);
                        }
                    }
                    start = None;
                }
            }
            _ => {}
        }
    }

    // Fallback if no JSON found
    if messages.is_empty() {
        messages.push(AgentMessage::Question {
            text: output.trim().to_string(),
            context: Some("(Agent response was not in expected format)".to_string()),
            input_type: InputType::Text,
            options: vec![],
            section: None,
        });
    }

    messages
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_single_json_object() {
        let output = r#"{"type": "question", "text": "What is your goal?", "input_type": "text", "options": []}"#;
        let messages = parse_all_json_objects(output);

        assert_eq!(messages.len(), 1);
        match &messages[0] {
            AgentMessage::Question { text, .. } => {
                assert_eq!(text, "What is your goal?");
            }
            _ => panic!("Expected Question"),
        }
    }

    #[test]
    fn test_parse_multiple_json_objects() {
        let output = r#"{"type": "draft_update", "section": "title", "content": "My Feature", "append": false}
{"type": "question", "text": "What else?", "input_type": "text", "options": []}"#;

        let messages = parse_all_json_objects(output);

        assert_eq!(messages.len(), 2);
        match &messages[0] {
            AgentMessage::DraftUpdate { section, .. } => {
                assert_eq!(section, "title");
            }
            _ => panic!("Expected DraftUpdate"),
        }
        match &messages[1] {
            AgentMessage::Question { text, .. } => {
                assert_eq!(text, "What else?");
            }
            _ => panic!("Expected Question"),
        }
    }

    #[test]
    fn test_parse_json_with_surrounding_text() {
        let output = r#"Some thinking text here...
{"type": "question", "text": "What is your goal?", "input_type": "text", "options": []}
More text after"#;

        let messages = parse_all_json_objects(output);

        assert_eq!(messages.len(), 1);
        match &messages[0] {
            AgentMessage::Question { text, .. } => {
                assert_eq!(text, "What is your goal?");
            }
            _ => panic!("Expected Question"),
        }
    }

    #[test]
    fn test_fallback_on_invalid_json() {
        let output = "This is just plain text, not JSON at all";
        let messages = parse_all_json_objects(output);

        assert_eq!(messages.len(), 1);
        match &messages[0] {
            AgentMessage::Question { text, context, .. } => {
                assert_eq!(text, output);
                assert!(context.as_ref().unwrap().contains("not in expected format"));
            }
            _ => panic!("Expected fallback Question"),
        }
    }

    #[test]
    fn test_empty_select_options_handled() {
        // This tests that the system handles empty options
        // The actual handling is in setup_input_state_for_type
        let output =
            r#"{"type": "question", "text": "Choose:", "input_type": "select", "options": []}"#;
        let messages = parse_all_json_objects(output);

        assert_eq!(messages.len(), 1);
        match &messages[0] {
            AgentMessage::Question {
                input_type,
                options,
                ..
            } => {
                assert!(matches!(input_type, InputType::Select));
                assert!(options.is_empty());
            }
            _ => panic!("Expected Question"),
        }
    }
}
