use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Span, Spans, Text},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame, Terminal,
};
use std::{
    env,
    error::Error,
    io,
    time::{Duration, Instant},
};

use copypasta::{ClipboardContext, ClipboardProvider};
use unicode_width::UnicodeWidthStr;

struct StatefulList<T: Default> {
    state: ListState,
    items: Vec<T>,
}

enum InputMode {
    Normal,
    Editing,
}

impl<T: Default> StatefulList<T> {
    fn with_items(items: Vec<T>) -> StatefulList<T> {
        let mut stateful_list = StatefulList {
            state: ListState::default(),
            items,
        };

        // Select the first element in the list
        stateful_list.state.select(Some(0));

        stateful_list
    }

    fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if self.items.len() == 0 {
                    0
                } else if i >= self.items.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    fn previous(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if self.items.len() == 0 {
                    0
                } else if i == 0 {
                    self.items.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    fn selected_index(&mut self) -> usize {
        // @TODO/improvement instead of returning the
        // index, get the actual item at that index.
        let index = match self.state.selected() {
            Some(i) => i,
            None => 0,
        };

        index
    }
}

/// This struct holds the current state of the app. In particular, it has the `items` field which is a wrapper
/// around `ListState`. Keeping track of the items state let us render the associated widget with its state
/// and have access to features such as natural scrolling.
///
/// Check the event handling at the bottom to see how to change the state on incoming events.
/// Check the drawing logic for items on how to specify the highlighting style for selected items.
struct App {
    full_history: Vec<String>,
    items: StatefulList<String>,
    input: String,
    input_prev: String,
    input_mode: InputMode,
    clipboard: copypasta::ClipboardContext,
}

impl App {
    fn new(history: Vec<String>) -> App {
        App {
            full_history: history.to_vec(),
            items: StatefulList::with_items(history),
            input: String::new(),
            input_prev: String::new(),
            input_mode: InputMode::Normal,
            clipboard: ClipboardContext::new().unwrap(),
        }
    }

    fn on_tick(&mut self) {
        match self.input_mode {
            InputMode::Editing => {
                // Only change the item state if the input is being updated. If not,
                // then no need to keep updating.
                if self.input_prev != self.input {
                    // @TODO/improvement add fuzzy searching here so it doesn't have to
                    // be an exact match. Also make it case insensitive so that I can type
                    // "curl" or "CURL" and it will show up all results for both either
                    // way. This is particularly helpful when searching for environment
                    // variables (e.g. "HUI_TERM") and being able to type "hui" to find
                    // it.
                    let mut filtered_history = self.full_history.to_vec();
                    filtered_history.retain(|line| line.contains(self.input.as_str()));
                    self.items = StatefulList::with_items(filtered_history);
                }
                self.input_prev = self.input.to_string();
            }
            _ => {}
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    // Determine the history file to fetch based on
    // the HUI_TERM environment variable.
    let history_file = match env::var_os("HUI_TERM") {
        Some(term) => {
            if term == "zsh" {
                Ok(".zsh_history".to_string())
            } else if term == "bash" {
                Ok(".bash_history".to_string())
            } else {
                Err("Currently only 'bash' or 'zsh' are supported for $HUI_TERM.")
            }
        }
        None => Err("$HUI_TERM needs to be set."),
    }
    .unwrap();

    // Fetch the history based on the HUI_TERM environment
    // variable that is set.
    let history = history::fetch(history_file);

    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // create app and run it
    let tick_rate = Duration::from_millis(250);
    let app = App::new(history);
    let res = run_app(&mut terminal, app, tick_rate);

    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    match res {
        Err(err) => println!("{:?}", err),
        Ok(resp) => {
            if resp != "" {
                println!("{}", resp);
            }
        },
    }

    Ok(())
}

fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    mut app: App,
    tick_rate: Duration,
) -> io::Result<String> {
    let mut last_tick = Instant::now();
    loop {
        terminal.draw(|f| ui(f, &mut app))?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));
        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                match app.input_mode {
                    InputMode::Normal => match key.code {
                        KeyCode::Char('/') => {
                            app.input = "".to_string();
                            app.input_mode = InputMode::Editing;
                        }
                        KeyCode::Char('q') => {
                            return Ok("".to_string());
                        }
                        KeyCode::Down => app.items.next(),
                        KeyCode::Up => app.items.previous(),
                        KeyCode::Enter => {
                            let index = app.items.selected_index();
                            let val = match app.items.items.get(index) {
                                Some(val) => val.to_string(),
                                None => "".to_string(),
                            };
                            // Copy the text to the clipboard before quitting
                            app.clipboard.set_contents(val.clone()).unwrap();
                            return Ok(format!("Copied to clipboard: {}", val.to_string()));
                        }
                        _ => {}
                    },
                    InputMode::Editing if key.kind == KeyEventKind::Press => match key.code {
                        // @TODO/improvement it would be nice te be able to
                        // move the cursor while selecting text, but I haven't
                        // figured out the best way to place text in the string 
                        // where the cursor is. So it is getting the boot for now.

                        // @TODO/improvement It would be nice to be able to
                        // use metacharacters just like in a normal terminal.
                        // Examples: Opt + Arrows to jump by word
                        //           Opt + Backspace to delete by word
                        //           Cmd + Arrows to jump to beginning and end
                        //           Cmd + Backspace to delete everything

                        KeyCode::Enter | KeyCode::Up | KeyCode::Down => {
                            app.input_mode = InputMode::Normal;
                        }
                        KeyCode::Char(c) => {
                            app.input.push(c);
                        }
                        KeyCode::Backspace => {
                            app.input.pop();
                        }
                        KeyCode::Esc => {
                            // Empty the input if nothing is done.
                            app.input.drain(..);
                            app.items = StatefulList::with_items(app.full_history.to_vec());
                            app.input_mode = InputMode::Normal;
                        }
                        _ => {}
                    },
                    _ => {}
                }
            }
        }
        if last_tick.elapsed() >= tick_rate {
            app.on_tick();
            last_tick = Instant::now();
        }
    }
}

fn ui<B: Backend>(f: &mut Frame<B>, app: &mut App) {
    // Create two chunks with equal horizontal screen space
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints(
            [
                Constraint::Min(1),
                Constraint::Length(3),
                Constraint::Length(1),
            ]
            .as_ref(),
        )
        .split(f.size());

    let (msg, style) = match app.input_mode {
        InputMode::Normal => (
            vec![
                Span::raw("Press "),
                Span::styled("/", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" to filter results, "),
                Span::styled("Enter", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" to copy selected command and exit, "),
                Span::styled("q", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" to exit without copying."),
            ],
            Style::default().add_modifier(Modifier::RAPID_BLINK),
        ),
        InputMode::Editing => (
            vec![
                Span::raw("Press "),
                Span::styled("Enter", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" to filter history, "),
                Span::styled("Esc", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" to stop filtering."),
            ],
            Style::default(),
        ),
    };
    let mut text = Text::from(Spans::from(msg));
    text.patch_style(style);
    let help_message = Paragraph::new(text);
    f.render_widget(help_message, chunks[2]);

    let select_color = Color::Red;

    let input = Paragraph::new(app.input.as_ref())
        .style(match app.input_mode {
            InputMode::Normal => Style::default(),
            InputMode::Editing => Style::default().fg(select_color),
        })
        .block(Block::default().borders(Borders::ALL).title("Search"));
    f.render_widget(input, chunks[1]);
    match app.input_mode {
        InputMode::Normal =>
            // Hide the cursor. `Frame` does this by default, so we don't need to do anything here
            {}

        InputMode::Editing => {
            // Make the cursor visible and ask ratatui to put it at the specified coordinates after rendering
            f.set_cursor(
                // Put cursor past the end of the input text
                chunks[1].x + app.input.width() as u16 + 1,
                // Move one line down, from the border to the input line
                chunks[1].y + 1,
            )
        }
    }

    // Iterate through all elements in the `items` app and append some debug text to it.
    let items: Vec<ListItem> = app
        .items
        .items
        .iter()
        .map(|i| ListItem::new(i.to_string()).style(Style::default()))
        .collect();

    // Create a List from all list items and highlight the currently selected one
    let items = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("History"))
        .highlight_style(match app.input_mode {
            InputMode::Normal => Style::default()
                .bg(select_color)
                .add_modifier(Modifier::BOLD),
            InputMode::Editing => Style::default(),
        })
        .highlight_symbol(match app.input_mode {
            InputMode::Normal => "> ",
            InputMode::Editing => "  ",
        });

    // We can now render the item list
    f.render_stateful_widget(items, chunks[0], &mut app.items.state);
}

// This uses a lot of what hstr-rs did to parse ZSH history:
// https://github.com/overclockworked64/hstr-rs/blob/master/src/hstr.rs
mod history {
    use itertools::Itertools;
    use regex::Regex;
    use std::env;
    use std::fs;
    use std::path::Path;
    use std::process;

    pub type History = Vec<String>;

    trait FromBytes {
        fn from_bytes(bytes: Vec<u8>, history_type: String) -> History;
    }

    impl FromBytes for History {
        fn from_bytes(bytes: Vec<u8>, history_type: String) -> History {
            // Split by ": " since that is what is a new line command for ZSH
            // As far as I can tell, Bash automatically makes multiline commands
            // into one line when writing to the .bash_history file? I'd need to
            // look more into that to be sure though.
            let s = std::str::from_utf8(&bytes).unwrap();

            let pattern: &str;
            if history_type == "zsh" {
                pattern = std::str::from_utf8(&[10, 58, 32]).unwrap();
            } else {
                pattern = std::str::from_utf8(&[10]).unwrap();
            }
            s
                .split(pattern) // split on newline for bash and on "\n: " for zsh
                .map(|line| String::from_utf8(line.as_bytes().to_vec()).unwrap())
                .collect()
        }
    }

    pub fn fetch(history_file: String) -> History {
        // @TODO/improvement should definitely figure out
        // how to use the Result to return a better error
        // instead of killing the process. This will make
        // it hard to test.
        let home_dir = match env::home_dir() {
            Some(path) => path.display().to_string(),
            None => "".to_owned(),
        };
        if home_dir == "" {
            println!("Couldn't get home_dir");
            process::exit(0x0100);
        }

        let path = Path::new(home_dir.as_str());
        let full_path = path.join(history_file.as_str());
        let contents = fs::read(full_path).expect("Should have been able to read the file");

        let history_type: String;
        if history_file.contains(".zsh_history") {
            history_type = "zsh".to_string()
        } else if history_file.contains(".bash_history") {
            history_type = "bash".to_string()
        } else {
            println!("Unsupported history type");
            process::exit(0x0100);
        }
        // println!("{:?}", History::from_bytes(contents.clone()).len());
        // println!("{:?}", path.as_os_str());
        // println!("{}", history_file);
        // println!("{}", history_type);
        process_history(contents, history_type)
    }

    pub fn process_history(history: Vec<u8>, history_type: String) -> History {

        // @TODO/improvement I don't like how much I'm passing around zsh/bash, this should become
        // its zsh/bash interfaces built on top of history as a base.
        if history_type == "zsh"{
            return reverse(remove_duplicates(remove_empty(remove_timestamps(
                History::from_bytes(unmetafy(history), history_type),
            ))))
        }
        reverse(remove_duplicates(remove_empty(
            History::from_bytes(history, history_type),
        )))

    }

    fn unmetafy(mut bytestring: Vec<u8>) -> Vec<u8> {
        /* Unmetafying zsh history requires looping over the bytestring, removing
         * each encountered Meta character, and XOR-ing the following byte with 32.
         *
         * For instance:
         *
         * Input: ('a', 'b', 'c', Meta, 'd', 'e', 'f')
         * Wanted: ('a', 'b', 'c', 'd' ^ 32, 'e', 'f')
         */
        const ZSH_META: u8 = 0x83;

        for index in (0..bytestring.len()).rev() {
            if bytestring[index] == ZSH_META {
                bytestring.remove(index);
                bytestring[index] ^= 32;
            }
        }
        bytestring
    }

    fn remove_timestamps(mut history: History) -> History {
        /* The metadata in the .zsh_history file looks like:
         *
         * : 1330648651:0;sudo reboot
         * 
         * I strip it in from_bytes() by "\n: " so it better
         * handles multiline commands. So this will only
         * strip by what is left after that parsing:
         * 
         * 1330648651:0;sudo reboot
         * 
         * So the command it get after parsing is:
         * 
         * sudo reboot
         */
        //   : 1330648651:0;sudo reboot

        // Special case: need to handle the first element in the history
        // since it doesn't have a new line, so it wasn't parsed at all
        // in from_bytes().
        // @TODO/improvement I don't like having to do this, come up with
        // a better way.
        let regex_first = Regex::new(r"^: \d{10}:\d;").unwrap();
        let first = history.get(0);
        let val = regex_first.replace(first.unwrap(), "").to_owned();
        history[0] = val.to_string();
        
        let regex_rest = Regex::new(r"^\d{10}:\d;").unwrap();
        history
            .iter()
            .map(|line| regex_rest.replace(line, "").into_owned())
            .collect()
    }

    fn remove_empty(mut history: History) -> History {
        history.retain(|line| line != "");
        history
    }

    fn reverse(mut history: History) -> History {
        history.reverse();
        history
    }

    fn remove_duplicates(mut history: History) -> History {
        history = history.into_iter().unique().collect();
        history
    }
}
