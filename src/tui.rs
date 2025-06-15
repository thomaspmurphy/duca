use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Frame, Terminal,
};
use std::io;

use crate::{Cantica, Canto, DivinaCommedia};

pub struct App {
    pub commedia: DivinaCommedia,
    pub current_cantica: String,
    pub current_canto: Option<u8>,
    pub cantica_list_state: ListState,
    pub canto_list_state: ListState,
    pub verse_scroll: u16,
    pub search_input: String,
    pub search_results: Vec<SearchResult>,
    pub filtered_results: Vec<SearchResult>,
    pub search_list_state: ListState,
    pub mode: AppMode,
    pub fuzzy_matcher: SkimMatcherV2,
    pub context_canto: Option<(String, u8)>,
    pub context_highlight_line: Option<usize>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SearchResult {
    pub cantica: String,
    pub canto: u8,
    pub line: usize,
    pub text: String,
    pub score: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AppMode {
    Browse,
    InteractiveSearch,
    ContextView,
}

impl App {
    pub fn new(commedia: DivinaCommedia) -> Self {
        let mut cantica_list_state = ListState::default();
        cantica_list_state.select(Some(0));

        Self {
            commedia,
            current_cantica: "Inferno".to_string(),
            current_canto: None,
            cantica_list_state,
            canto_list_state: ListState::default(),
            verse_scroll: 0,
            search_input: String::new(),
            search_results: Vec::new(),
            filtered_results: Vec::new(),
            search_list_state: ListState::default(),
            mode: AppMode::Browse,
            fuzzy_matcher: SkimMatcherV2::default(),
            context_canto: None,
            context_highlight_line: None,
        }
    }

    pub fn next_cantica(&mut self) {
        let i = match self.cantica_list_state.selected() {
            Some(i) => {
                if i >= 2 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.cantica_list_state.select(Some(i));
        self.update_current_cantica();
        self.canto_list_state.select(None);
        self.current_canto = None;
    }

    pub fn previous_cantica(&mut self) {
        let i = match self.cantica_list_state.selected() {
            Some(i) => {
                if i == 0 {
                    2
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.cantica_list_state.select(Some(i));
        self.update_current_cantica();
        self.canto_list_state.select(None);
        self.current_canto = None;
    }

    pub fn next_canto(&mut self) {
        let cantica = self.get_current_cantica();
        let max_cantos = cantica.cantos.len();

        let i = match self.canto_list_state.selected() {
            Some(i) => {
                if i >= max_cantos.saturating_sub(1) {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.canto_list_state.select(Some(i));
        self.update_current_canto();
        self.verse_scroll = 0;
    }

    pub fn previous_canto(&mut self) {
        let cantica = self.get_current_cantica();
        let max_cantos = cantica.cantos.len();

        let i = match self.canto_list_state.selected() {
            Some(i) => {
                if i == 0 {
                    max_cantos.saturating_sub(1)
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.canto_list_state.select(Some(i));
        self.update_current_canto();
        self.verse_scroll = 0;
    }

    pub fn scroll_down(&mut self) {
        self.verse_scroll = self.verse_scroll.saturating_add(1);
    }

    pub fn scroll_up(&mut self) {
        self.verse_scroll = self.verse_scroll.saturating_sub(1);
    }

    pub fn update_current_cantica(&mut self) {
        self.current_cantica = match self.cantica_list_state.selected() {
            Some(0) => "Inferno".to_string(),
            Some(1) => "Purgatorio".to_string(),
            Some(2) => "Paradiso".to_string(),
            _ => "Inferno".to_string(),
        };
    }

    pub fn update_current_canto(&mut self) {
        if let Some(selected) = self.canto_list_state.selected() {
            let cantica = self.get_current_cantica();
            let mut canto_numbers: Vec<_> = cantica.cantos.keys().collect();
            canto_numbers.sort();

            if let Some(&&canto_num) = canto_numbers.get(selected) {
                self.current_canto = Some(canto_num);
            }
        }
    }

    pub fn get_current_cantica(&self) -> &Cantica {
        match self.current_cantica.as_str() {
            "Inferno" => &self.commedia.inferno,
            "Purgatorio" => &self.commedia.purgatorio,
            "Paradiso" => &self.commedia.paradiso,
            _ => &self.commedia.inferno,
        }
    }

    pub fn get_current_canto(&self) -> Option<&Canto> {
        if let Some(canto_num) = self.current_canto {
            self.get_current_cantica().cantos.get(&canto_num)
        } else {
            None
        }
    }

    pub fn interactive_search(&mut self) {
        if self.search_input.trim().is_empty() {
            self.filtered_results.clear();
            self.search_list_state.select(None);
            return;
        }

        // Get all results from the basic search
        let basic_results = self.commedia.search(&self.search_input, None);

        // Convert to SearchResult and apply fuzzy matching
        let mut scored_results: Vec<SearchResult> = basic_results
            .into_iter()
            .filter_map(|(cantica, canto, line, text)| {
                self.fuzzy_matcher
                    .fuzzy_match(&text, &self.search_input)
                    .map(|score| SearchResult {
                        cantica,
                        canto,
                        line,
                        text,
                        score,
                    })
            })
            .collect();

        // Sort by score (highest first)
        scored_results.sort_by(|a, b| b.score.cmp(&a.score));

        // Take top 50 results for performance
        scored_results.truncate(50);

        self.filtered_results = scored_results;
        self.search_list_state
            .select(if self.filtered_results.is_empty() {
                None
            } else {
                Some(0)
            });
    }

    pub fn enter_search_mode(&mut self) {
        self.mode = AppMode::InteractiveSearch;
        self.search_input.clear();
        self.filtered_results.clear();
        self.search_list_state.select(None);
    }

    pub fn enter_context_view(&mut self) {
        if let Some(selected) = self.search_list_state.selected() {
            if let Some(result) = self.filtered_results.get(selected) {
                self.context_canto = Some((result.cantica.clone(), result.canto));
                self.context_highlight_line = Some(result.line);
                self.mode = AppMode::ContextView;
                self.verse_scroll = result.line.saturating_sub(10) as u16;
            }
        }
    }

    pub fn exit_context_view(&mut self) {
        self.context_canto = None;
        self.context_highlight_line = None;
        self.mode = AppMode::InteractiveSearch;
    }

    pub fn clear_search(&mut self) {
        self.search_input.clear();
        self.search_results.clear();
        self.filtered_results.clear();
        self.search_list_state.select(None);
        self.mode = AppMode::Browse;
    }

    pub fn next_search_result(&mut self) {
        let len = self.filtered_results.len();
        if len == 0 {
            return;
        }

        let i = match self.search_list_state.selected() {
            Some(i) => {
                if i >= len - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.search_list_state.select(Some(i));
    }

    pub fn previous_search_result(&mut self) {
        let len = self.filtered_results.len();
        if len == 0 {
            return;
        }

        let i = match self.search_list_state.selected() {
            Some(i) => {
                if i == 0 {
                    len - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.search_list_state.select(Some(i));
    }

    pub fn get_context_canto(&self) -> Option<&Canto> {
        if let Some((cantica_name, canto_num)) = &self.context_canto {
            let cantica = match cantica_name.as_str() {
                "Inferno" => &self.commedia.inferno,
                "Purgatorio" => &self.commedia.purgatorio,
                "Paradiso" => &self.commedia.paradiso,
                _ => return None,
            };
            cantica.cantos.get(canto_num)
        } else {
            None
        }
    }
}

pub fn run_tui(commedia: DivinaCommedia) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let app = App::new(commedia);
    let res = run_app(&mut terminal, app);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{err:?}");
    }

    Ok(())
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut app: App) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui(f, &mut app))?;

        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                match app.mode {
                    AppMode::Browse => match key.code {
                        KeyCode::Char('q') => return Ok(()),
                        KeyCode::Char('h') | KeyCode::Left => app.previous_cantica(),
                        KeyCode::Char('l') | KeyCode::Right => app.next_cantica(),
                        KeyCode::Char('j') | KeyCode::Down => app.next_canto(),
                        KeyCode::Char('k') | KeyCode::Up => app.previous_canto(),
                        KeyCode::Char('J') => app.scroll_down(),
                        KeyCode::Char('K') => app.scroll_up(),
                        KeyCode::Char('/') => app.enter_search_mode(),
                        KeyCode::Enter => {
                            if app.current_canto.is_none()
                                && app.canto_list_state.selected().is_some()
                            {
                                app.update_current_canto();
                            }
                        }
                        _ => {}
                    },
                    AppMode::InteractiveSearch => match key.code {
                        KeyCode::Esc => app.clear_search(),
                        KeyCode::Backspace => {
                            app.search_input.pop();
                            app.interactive_search();
                        }
                        KeyCode::Down => app.next_search_result(),
                        KeyCode::Up => app.previous_search_result(),
                        KeyCode::Enter => app.enter_context_view(),
                        KeyCode::Char('j') => app.next_search_result(),
                        KeyCode::Char('k') => app.previous_search_result(),
                        KeyCode::Char(c) => {
                            app.search_input.push(c);
                            app.interactive_search();
                        }
                        _ => {}
                    },
                    AppMode::ContextView => match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => app.exit_context_view(),
                        KeyCode::Char('J') | KeyCode::Down => app.scroll_down(),
                        KeyCode::Char('K') | KeyCode::Up => app.scroll_up(),
                        _ => {}
                    },
                }
            }
        }
    }
}

fn ui(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(20), Constraint::Percentage(80)].as_ref())
        .split(f.size());

    let left_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(5), Constraint::Min(0)].as_ref())
        .split(chunks[0]);

    render_cantica_list(f, left_chunks[0], app);
    render_canto_list(f, left_chunks[1], app);

    match app.mode {
        AppMode::Browse => render_verse_display(f, chunks[1], app),
        AppMode::InteractiveSearch => render_interactive_search(f, chunks[1], app),
        AppMode::ContextView => render_context_view(f, chunks[1], app),
    }
}

fn render_cantica_list(f: &mut Frame, area: Rect, app: &mut App) {
    let canticas = ["Inferno", "Purgatorio", "Paradiso"];
    let items: Vec<ListItem> = canticas
        .iter()
        .map(|cantica| {
            let content = cantica.to_string();
            ListItem::new(content)
        })
        .collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Cantica"))
        .highlight_style(
            Style::default()
                .bg(Color::LightGreen)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");

    f.render_stateful_widget(list, area, &mut app.cantica_list_state);
}

fn render_canto_list(f: &mut Frame, area: Rect, app: &mut App) {
    let cantica = app.get_current_cantica();
    let mut canto_numbers: Vec<_> = cantica.cantos.keys().collect();
    canto_numbers.sort();

    let items: Vec<ListItem> = canto_numbers
        .iter()
        .map(|&&num| ListItem::new(format!("Canto {}", num)))
        .collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Cantos"))
        .highlight_style(
            Style::default()
                .bg(Color::LightGreen)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");

    f.render_stateful_widget(list, area, &mut app.canto_list_state);
}

fn render_verse_display(f: &mut Frame, area: Rect, app: &App) {
    let title = if let Some(canto) = app.get_current_canto() {
        format!("{} Canto {}", app.current_cantica, canto.roman_numeral)
    } else {
        format!("{} - Select a Canto", app.current_cantica)
    };

    if let Some(canto) = app.get_current_canto() {
        let verses: Vec<Line> = canto
            .verses
            .iter()
            .skip(app.verse_scroll as usize)
            .map(|verse| {
                Line::from(vec![
                    Span::styled(
                        format!("{:3}: ", verse.line_number),
                        Style::default().fg(Color::Yellow),
                    ),
                    Span::raw(&verse.text),
                ])
            })
            .collect();

        let paragraph = Paragraph::new(verses)
            .block(Block::default().borders(Borders::ALL).title(title.clone()))
            .wrap(Wrap { trim: true });

        f.render_widget(paragraph, area);
    } else {
        let help_text = vec![
            Line::from("Navigation:"),
            Line::from("h/← l/→  - Switch Cantica"),
            Line::from("j/↓ k/↑  - Select Canto"),
            Line::from("J K      - Scroll verses"),
            Line::from("/        - Interactive Search (fzf-like)"),
            Line::from("q        - Quit"),
            Line::from(""),
            Line::from("Search Features:"),
            Line::from("• Live filtering as you type"),
            Line::from("• Fuzzy matching with scoring"),
            Line::from("• Enter to view in context"),
            Line::from("• Esc to return"),
        ];

        let paragraph = Paragraph::new(help_text)
            .block(Block::default().borders(Borders::ALL).title(title.clone()))
            .alignment(Alignment::Left);

        f.render_widget(paragraph, area);
    }
}

fn render_interactive_search(f: &mut Frame, area: Rect, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)].as_ref())
        .split(area);

    // Search input box
    let input = Paragraph::new(app.search_input.as_str())
        .style(Style::default().fg(Color::Yellow))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Interactive Search (type to filter)"),
        );
    f.render_widget(input, chunks[0]);

    // Live results
    let items: Vec<ListItem> = app
        .filtered_results
        .iter()
        .map(|result| {
            let preview = if result.text.len() > 80 {
                format!("{}...", &result.text[..77])
            } else {
                result.text.clone()
            };
            ListItem::new(format!(
                "{} {}.{}: {}",
                result.cantica, result.canto, result.line, preview
            ))
        })
        .collect();

    let results_title = if app.filtered_results.is_empty() && !app.search_input.is_empty() {
        "No matches found".to_string()
    } else {
        format!(
            "Results ({}) - Enter to view context",
            app.filtered_results.len()
        )
    };

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(results_title))
        .highlight_style(
            Style::default()
                .bg(Color::LightGreen)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("► ");

    f.render_stateful_widget(list, chunks[1], &mut app.search_list_state);
}

fn render_context_view(f: &mut Frame, area: Rect, app: &App) {
    if let Some(canto) = app.get_context_canto() {
        let title = if let Some((cantica, _canto_num)) = &app.context_canto {
            format!(
                "{} Canto {} - Context View (Esc to return)",
                cantica, canto.roman_numeral
            )
        } else {
            "Context View".to_string()
        };

        let verses: Vec<Line> = canto
            .verses
            .iter()
            .skip(app.verse_scroll as usize)
            .map(|verse| {
                let style = if Some(verse.line_number) == app.context_highlight_line {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };

                Line::from(vec![
                    Span::styled(
                        format!("{:3}: ", verse.line_number),
                        Style::default().fg(
                            if Some(verse.line_number) == app.context_highlight_line {
                                Color::Red
                            } else {
                                Color::Cyan
                            },
                        ),
                    ),
                    Span::styled(&verse.text, style),
                ])
            })
            .collect();

        let paragraph = Paragraph::new(verses)
            .block(Block::default().borders(Borders::ALL).title(title))
            .wrap(Wrap { trim: true });

        f.render_widget(paragraph, area);
    } else {
        let paragraph = Paragraph::new("No context available")
            .block(Block::default().borders(Borders::ALL).title("Context View"));
        f.render_widget(paragraph, area);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Canto, DivinaCommedia, Verse};

    fn create_test_commedia() -> DivinaCommedia {
        let mut commedia = DivinaCommedia::new();

        // Add test canto to Inferno
        let canto1 = Canto {
            number: 1,
            roman_numeral: "I".to_string(),
            verses: vec![
                Verse {
                    line_number: 1,
                    text: "Nel mezzo del cammin di nostra vita".to_string(),
                },
                Verse {
                    line_number: 2,
                    text: "mi ritrovai per una selva oscura".to_string(),
                },
                Verse {
                    line_number: 3,
                    text: "ché la diritta via era smarrita".to_string(),
                },
            ],
        };
        commedia.inferno.cantos.insert(1, canto1);

        // Add test canto to Purgatorio
        let canto1_purg = Canto {
            number: 1,
            roman_numeral: "I".to_string(),
            verses: vec![
                Verse {
                    line_number: 1,
                    text: "Per correr miglior acque alza le vele".to_string(),
                },
                Verse {
                    line_number: 2,
                    text: "omai la navicella del mio ingegno".to_string(),
                },
            ],
        };
        commedia.purgatorio.cantos.insert(1, canto1_purg);

        commedia
    }

    #[test]
    fn test_app_new() {
        let commedia = create_test_commedia();
        let app = App::new(commedia);

        assert_eq!(app.current_cantica, "Inferno");
        assert_eq!(app.mode, AppMode::Browse);
        assert!(app.search_input.is_empty());
        assert!(app.search_results.is_empty());
        assert_eq!(app.verse_scroll, 0);
        assert_eq!(app.current_canto, None);
    }

    #[test]
    fn test_cantica_navigation() {
        let commedia = create_test_commedia();
        let mut app = App::new(commedia);

        // Test next cantica
        assert_eq!(app.current_cantica, "Inferno");
        app.next_cantica();
        assert_eq!(app.current_cantica, "Purgatorio");
        app.next_cantica();
        assert_eq!(app.current_cantica, "Paradiso");
        app.next_cantica();
        assert_eq!(app.current_cantica, "Inferno"); // Should wrap around

        // Test previous cantica
        app.previous_cantica();
        assert_eq!(app.current_cantica, "Paradiso");
        app.previous_cantica();
        assert_eq!(app.current_cantica, "Purgatorio");
        app.previous_cantica();
        assert_eq!(app.current_cantica, "Inferno");
    }

    #[test]
    fn test_canto_navigation() {
        let commedia = create_test_commedia();
        let mut app = App::new(commedia);

        // Initially no canto selected
        assert_eq!(app.current_canto, None);

        // Select first canto
        app.next_canto();
        assert_eq!(app.current_canto, Some(1));

        // Navigate to Purgatorio
        app.next_cantica();
        assert_eq!(app.current_cantica, "Purgatorio");
        app.next_canto();
        assert_eq!(app.current_canto, Some(1));
    }

    #[test]
    fn test_search_result_structure() {
        let result = SearchResult {
            cantica: "Inferno".to_string(),
            canto: 1,
            line: 2,
            text: "test verse".to_string(),
            score: 100,
        };

        assert_eq!(result.cantica, "Inferno");
        assert_eq!(result.canto, 1);
        assert_eq!(result.line, 2);
        assert_eq!(result.text, "test verse");
        assert_eq!(result.score, 100);
    }

    #[test]
    fn test_app_mode_changes() {
        let commedia = create_test_commedia();
        let mut app = App::new(commedia);

        assert_eq!(app.mode, AppMode::Browse);

        // Test mode transitions
        app.mode = AppMode::InteractiveSearch;
        assert_eq!(app.mode, AppMode::InteractiveSearch);

        app.mode = AppMode::ContextView;
        assert_eq!(app.mode, AppMode::ContextView);
    }

    #[test]
    fn test_search_input_handling() {
        let commedia = create_test_commedia();
        let mut app = App::new(commedia);

        assert!(app.search_input.is_empty());

        app.search_input = "test search".to_string();
        assert_eq!(app.search_input, "test search");

        app.search_input.clear();
        assert!(app.search_input.is_empty());
    }

    #[test]
    fn test_verse_scrolling() {
        let commedia = create_test_commedia();
        let mut app = App::new(commedia);

        assert_eq!(app.verse_scroll, 0);

        app.verse_scroll = 10;
        assert_eq!(app.verse_scroll, 10);

        app.verse_scroll = 0;
        assert_eq!(app.verse_scroll, 0);
    }

    #[test]
    fn test_get_current_cantica() {
        let commedia = create_test_commedia();
        let app = App::new(commedia);

        let current = app.get_current_cantica();
        assert_eq!(current.name, "Inferno");
        assert!(current.cantos.contains_key(&1));
    }

    #[test]
    fn test_fuzzy_matcher_integration() {
        let commedia = create_test_commedia();
        let app = App::new(commedia);

        // Test that fuzzy matcher is initialized
        let score = app.fuzzy_matcher.fuzzy_match("test", "test");
        assert!(score.is_some());
        assert!(score.unwrap() > 0);

        let no_score = app.fuzzy_matcher.fuzzy_match("abc", "xyz");
        assert!(no_score.is_none() || no_score.unwrap() == 0);
    }

    #[test]
    fn test_context_canto_tracking() {
        let commedia = create_test_commedia();
        let mut app = App::new(commedia);

        assert_eq!(app.context_canto, None);
        assert_eq!(app.context_highlight_line, None);

        app.context_canto = Some(("Inferno".to_string(), 1));
        app.context_highlight_line = Some(2);

        assert_eq!(app.context_canto, Some(("Inferno".to_string(), 1)));
        assert_eq!(app.context_highlight_line, Some(2));
    }
}

