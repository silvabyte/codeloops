//! Main TUI application for the prompt generator.

use std::io::{self, Stdout};
use std::path::PathBuf;
use std::time::Duration;

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

use super::layout::{DraftLayout, InterviewLayout, MainLayout};
use super::widgets::{
    ConfirmInput, DraftWidget, MultiSelectInput, ProgressWidget, QuestionWidget, SelectInput,
    TextInput,
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
    /// Status message
    status_message: Option<String>,
    /// Terminal instance
    terminal: Terminal<CrosstermBackend<Stdout>>,
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
            status_message: self.status_message.clone(),
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
}

impl RenderState {
    fn render(&self, frame: &mut Frame) {
        let area = frame.area();
        let layout = MainLayout::new(area);

        // Render header
        self.render_header(frame, layout.header);

        // Render left panel (interview)
        self.render_interview_panel(frame, layout.left_panel);

        // Render right panel (draft preview)
        self.render_draft_panel(frame, layout.right_panel);

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
                let thinking = Paragraph::new(vec![
                    Line::from(Span::styled(
                        "⏳ Thinking...",
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    )),
                    Line::from(""),
                    Line::from(Span::styled(
                        message.as_str(),
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

        // Title with completion
        let completion = self.draft.completion_percentage();
        let title = Line::from(vec![
            Span::styled("prompt.md ", Style::default().fg(Color::White)),
            Span::styled(
                format!("({}% complete)", completion),
                Style::default().fg(if completion >= 75 {
                    Color::Green
                } else if completion >= 50 {
                    Color::Yellow
                } else {
                    Color::Red
                }),
            ),
        ]);
        frame.render_widget(Paragraph::new(title), draft_layout.title);

        // Progress bar
        let progress = ProgressWidget::new(completion);
        frame.render_widget(progress, draft_layout.progress);

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
                self.status_message = Some("Session saved!".to_string());
                return Ok(());
            }
            (KeyModifiers::NONE, KeyCode::Tab) => {
                // Switch focus
                self.focus = match self.focus {
                    Focus::Question => Focus::Draft,
                    Focus::Draft => Focus::Question,
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
                        self.status_message = Some("Editor returned empty content".to_string());
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
                // Can't do much while waiting
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

    /// Start a new interview
    async fn start_interview(&mut self) -> Result<()> {
        self.input_state = InputState::WaitingForAgent {
            message: "Starting interview...".to_string(),
        };
        self.draw()?;

        // Build the system prompt with project context
        let system_prompt = build_system_prompt(&self.session.project_context);

        // Send to agent
        let output = self
            .agent
            .execute(&system_prompt, &self.agent_config)
            .await
            .context("Failed to start interview")?;

        // Parse the response
        self.handle_agent_output(&output.stdout)?;

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

        // Send to agent
        let output = self
            .agent
            .execute(&full_prompt, &self.agent_config)
            .await
            .context("Failed to continue interview")?;

        // Parse the response
        self.handle_agent_output(&output.stdout)?;

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

        // Send to agent
        let output = self
            .agent
            .execute(&full_prompt, &self.agent_config)
            .await
            .context("Failed to get agent response")?;

        // Parse the response
        self.handle_agent_output(&output.stdout)?;

        Ok(())
    }

    /// Handle agent output and parse JSON messages
    fn handle_agent_output(&mut self, output: &str) -> Result<()> {
        // Try to find and parse JSON in the output
        let message = self.parse_agent_message(output)?;

        // Add to history
        self.session.add_agent_message(message.clone());

        // Handle the message
        self.handle_agent_message(message)?;

        Ok(())
    }

    /// Parse an agent message from output
    fn parse_agent_message(&self, output: &str) -> Result<AgentMessage> {
        // Try to find JSON in the output
        // The agent might output some text before/after the JSON

        // First, try to parse the whole thing as JSON
        if let Ok(msg) = serde_json::from_str::<AgentMessage>(output.trim()) {
            return Ok(msg);
        }

        // Try to find JSON object in the output
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
                                return Ok(msg);
                            }
                        }
                        start = None;
                    }
                }
                _ => {}
            }
        }

        // If we couldn't parse JSON, create a fallback question
        // This handles the case where the agent outputs plain text
        Ok(AgentMessage::Question {
            text: output.trim().to_string(),
            context: Some("(Agent response was not in expected format)".to_string()),
            input_type: InputType::Text,
            options: vec![],
            section: None,
        })
    }

    /// Handle a parsed agent message
    fn handle_agent_message(&mut self, message: AgentMessage) -> Result<()> {
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

                // Set up appropriate input state
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
                        self.input_state = InputState::SelectInput {
                            options,
                            selected: 0,
                        };
                    }
                    InputType::MultiSelect => {
                        let len = options.len();
                        self.input_state = InputState::MultiSelectInput {
                            options,
                            selected: vec![false; len],
                            cursor: 0,
                        };
                    }
                    InputType::Confirm => {
                        self.input_state = InputState::ConfirmInput { selected: true };
                    }
                }
            }
            AgentMessage::DraftUpdate {
                section,
                content,
                append,
            } => {
                // Update the draft
                self.session.apply_draft_update(&section, &content, append);
                self.status_message = Some(format!("Updated: {}", section));

                // Continue the interview
                // The agent should send another message after a draft update
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
                        self.input_state = InputState::SelectInput {
                            options,
                            selected: 0,
                        };
                    }
                    InputType::MultiSelect => {
                        let len = options.len();
                        self.input_state = InputState::MultiSelectInput {
                            options,
                            selected: vec![false; len],
                            cursor: 0,
                        };
                    }
                    InputType::Confirm => {
                        self.input_state = InputState::ConfirmInput { selected: true };
                    }
                }
            }
            AgentMessage::DraftComplete { summary } => {
                self.session.mark_complete();
                self.status_message = Some(summary);
                self.running = false;
            }
            AgentMessage::Error { message } => {
                self.error_message = Some(message);
                self.input_state = InputState::Idle;
            }
        }

        Ok(())
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
