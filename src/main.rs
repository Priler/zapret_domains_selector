use std::fs::{self, File};
use std::io::{self, Write, Read};
use std::path::Path;
use std::thread;
use std::time::Duration;
use crossterm::{
    execute,
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    terminal::{self, ClearType, EnterAlternateScreen, LeaveAlternateScreen},
    style::{self, Stylize},
    cursor::{self, Hide, Show},
    queue,
};

#[derive(Debug)]
struct FileEntry {
    name: String,
    selected: bool,
}

fn main() -> io::Result<()> {
    // Setup terminal
    terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, Hide)?;

    let result = run_app(&mut stdout);

    // Cleanup terminal
    execute!(stdout, Show, LeaveAlternateScreen)?;
    terminal::disable_raw_mode()?;

    result
}

fn draw_screen(stdout: &mut io::Stdout, entries: &[FileEntry], current_index: usize, clear_screen: bool) -> io::Result<()> {
    if clear_screen {
        queue!(
            stdout,
            terminal::Clear(ClearType::All),
            cursor::MoveTo(0, 0)
        )?;
    } else {
        queue!(stdout, cursor::MoveTo(0, 0))?;
    }

    // Header
    writeln!(stdout, "Используйте ↑↓ для навигации, ПРОБЕЛ или ENTER для выбора, ENTER на СОХРАНИТЬ/ОТМЕНА для завершения\n")?;

    // File list
    for (index, entry) in entries.iter().enumerate() {
        let name = if entry.name == "SAVE LIST" {
            "СОХРАНИТЬ СПИСОК".to_string()
        } else if entry.name == "CANCEL" {
            "ОТМЕНА".to_string()
        } else {
            entry.name.clone()
        };

        let line = format!(
            "{} [{}] {}",
            if index == current_index { ">" } else { " " },
            if entry.selected { "*" } else { " " },
            name
        );

        if index == current_index {
            writeln!(stdout, "{}", line.reverse())?;
        } else {
            writeln!(stdout, "{}", line)?;
        }
    }

    stdout.flush()
}

fn run_app(stdout: &mut io::Stdout) -> io::Result<()> {
    // Ensure lists directory exists
    let lists_dir = Path::new("lists");
    if !lists_dir.exists() {
        fs::create_dir(lists_dir)?;
    }

    // Read previously selected files
    let config_path = lists_dir.join("selected.txt");
    let mut selected_files = Vec::new();
    if config_path.exists() {
        let mut content = String::new();
        File::open(&config_path)?.read_to_string(&mut content)?;
        selected_files = content.lines().map(String::from).collect();
    }

    // Get list of txt files
    let mut entries: Vec<FileEntry> = fs::read_dir(lists_dir)?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let name = entry.file_name().into_string().ok()?;
            if name.starts_with("list-") &&
                name.ends_with(".txt") &&
                name != "list-ultimate.txt" {
                let selected = selected_files.contains(&name);
                Some(FileEntry {
                    name: name.clone(),
                    selected,
                })
            } else {
                None
            }
        })
        .collect();

    // Sort entries alphabetically
    entries.sort_by(|a, b| a.name.cmp(&b.name));

    // Add special options
    entries.push(FileEntry {
        name: String::from("SAVE LIST"),
        selected: false,
    });
    entries.push(FileEntry {
        name: String::from("CANCEL"),
        selected: false,
    });

    let mut current_index = 0;

    // Initial draw with full clear
    draw_screen(stdout, &entries, current_index, true)?;

    // Main event loop
    'main: loop {
        if let Ok(true) = event::poll(Duration::from_millis(16)) {
            if let Ok(Event::Key(key)) = event::read() {
                let mut redraw = true;

                match key {
                    KeyEvent {
                        code: KeyCode::Up,
                        kind: event::KeyEventKind::Press,
                        ..
                    } => {
                        if current_index > 0 {
                            current_index -= 1;
                        }
                    }
                    KeyEvent {
                        code: KeyCode::Down,
                        kind: event::KeyEventKind::Press,
                        ..
                    } => {
                        if current_index < entries.len() - 1 {
                            current_index += 1;
                        }
                    }
                    KeyEvent {
                        code: KeyCode::Char(' ') | KeyCode::Enter,
                        kind: event::KeyEventKind::Press,
                        ..
                    } => {
                        // Special handling for SAVE and CANCEL
                        if current_index >= entries.len() - 2 {
                            match entries[current_index].name.as_str() {
                                "SAVE LIST" => {
                                    let mut file = File::create(&config_path)?;
                                    for entry in &entries {
                                        if entry.selected {
                                            writeln!(file, "{}", entry.name)?;
                                        }
                                    }
                                    execute!(
                                        stdout,
                                        cursor::MoveToNextLine(1),
                                        terminal::Clear(ClearType::FromCursorDown)
                                    )?;
                                    println!("{}", "Успешно! Список сохранен. Выход через 5 секунд...".green());
                                    stdout.flush()?;
                                    thread::sleep(Duration::from_secs(5));
                                    break 'main Ok(());
                                }
                                "CANCEL" => break 'main Ok(()),
                                _ => {}
                            }
                        } else {
                            // Toggle selection for regular items
                            entries[current_index].selected = !entries[current_index].selected;
                        }
                    }
                    KeyEvent {
                        code: KeyCode::Char('c'),
                        modifiers: KeyModifiers::CONTROL,
                        kind: event::KeyEventKind::Press,
                        ..
                    } => {
                        break 'main Ok(());
                    }
                    _ => {
                        redraw = false;
                    }
                }

                if redraw {
                    draw_screen(stdout, &entries, current_index, false)?;
                }
            }
        }
    }
}