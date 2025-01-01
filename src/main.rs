use crossterm::{
    cursor::{Hide, MoveTo, Show},
    event::{read, Event, KeyCode},
    execute,
    style::Print,
    terminal::{Clear, ClearType, DisableLineWrap, EnableLineWrap},
};
use shlex;
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::path::Path;
use std::process::Command;

#[derive(Debug)]
struct MenuItem {
    name: String,
    working_dir: String,
    command: Vec<String>,
}

struct Menu {
    items: Vec<MenuItem>,
    selected: usize,
    max_length: usize,
}

const MENU_FILE: &str = "menu.csv";

impl MenuItem {
    fn from_csv_line(line: &str) -> Option<Self> {
        let parts: Vec<&str> = line.split(',').collect();
        if parts.len() >= 3 {
            let name = parts[0].to_string();
            let working_dir = parts[1].to_string();
            let command = if parts[2].trim().is_empty() {
                Vec::new()
            } else {
                shlex::split(parts[2].trim())?
            };

            Some(MenuItem {
                name,
                working_dir,
                command,
            })
        } else {
            None
        }
    }

    fn is_submenu(&self) -> bool {
        self.command.is_empty()
    }
}

impl Menu {
    fn new() -> Menu {
        Menu {
            items: Vec::new(),
            selected: 0,
            max_length: 0,
        }
    }

    fn load_from_file<P: AsRef<Path>>(path: P) -> io::Result<Menu> {
        let file = File::open(&path).map_err(|e| {
            io::Error::new(
                e.kind(),
                format!("Failed to open {}: {}", path.as_ref().display(), e),
            )
        })?;
        let reader = BufReader::new(file);
        let mut menu = Menu::new();

        for line in reader.lines() {
            let line = line?;
            if let Some(item) = MenuItem::from_csv_line(&line) {
                menu.max_length = menu.max_length.max(item.name.len() + 2);
                menu.items.push(item);
            }
        }

        Ok(menu)
    }

    fn draw(&self) -> io::Result<()> {
        let mut stdout = io::stdout();
        execute!(stdout, Clear(ClearType::All), MoveTo(0, 0))?;

        let term_size = crossterm::terminal::size()?;
        let center_x = (term_size.0 as usize - self.max_length) / 2;

        // Draw top border (single line)
        let top_border = format!("┌{}┐", "─".repeat(self.max_length - 2));
        execute!(stdout, MoveTo(center_x as u16, 0), Print(top_border))?;

        // Draw menu items
        for (i, item) in self.items.iter().enumerate() {
            let (left_border, right_border) = if i == self.selected {
                ("║", "║") // Double line for selected item
            } else {
                ("│", "│") // Single line for unselected items
            };

            let padding = self.max_length - 2;
            let name_padding = (padding - item.name.len()) / 2;
            let line = format!(
                "{}{}{}{}{}",
                left_border,
                " ".repeat(name_padding),
                item.name,
                " ".repeat(padding - name_padding - item.name.len()),
                right_border
            );
            execute!(stdout, MoveTo(center_x as u16, (i + 1) as u16), Print(line))?;
        }

        // Draw bottom border (single line)
        let bottom_border = format!("└{}┘", "─".repeat(self.max_length - 2));
        execute!(
            stdout,
            MoveTo(center_x as u16, (self.items.len() + 1) as u16),
            Print(bottom_border)
        )?;

        Ok(())
    }

    fn run_selected(&self) -> io::Result<()> {
        if let Some(item) = self.items.get(self.selected) {
            if item.is_submenu() {
                let submenu_path = Path::new(&item.working_dir).join(MENU_FILE);
                if let Ok(mut submenu) = Menu::load_from_file(&submenu_path) {
                    submenu.run()?;
                } else {
                    self.show_error("Failed to load submenu")?;
                }
                return Ok(());
            }

            execute!(io::stdout(), Clear(ClearType::All), EnableLineWrap)?;

            if let Some(program) = item.command.first() {
                let args = item.command.iter().skip(1);
                let status = Command::new(program)
                    .args(args)
                    .current_dir(&item.working_dir)
                    .status()
                    .map_err(|e| {
                        io::Error::new(e.kind(), format!("Failed to execute '{}': {}", program, e))
                    })?;

                if !status.success() {
                    self.show_error(&format!(
                        "Command failed with status: {}",
                        status.code().unwrap_or(-1)
                    ))?;
                }
            }
            execute!(io::stdout(), Hide, DisableLineWrap)?;
        }
        Ok(())
    }

    fn show_error(&self, message: &str) -> io::Result<()> {
        execute!(
            io::stdout(),
            Clear(ClearType::All),
            EnableLineWrap,
            Print(format!("Error: {}\nPress any key to continue...", message))
        )?;
        read()?;
        execute!(io::stdout(), DisableLineWrap)?;
        Ok(())
    }

    fn run(&mut self) -> io::Result<()> {
        loop {
            self.draw()?;

            match read()? {
                Event::Key(event) => match event.code {
                    KeyCode::Up if self.selected > 0 => {
                        self.selected -= 1;
                    }
                    KeyCode::Down if self.selected < self.items.len() - 1 => {
                        self.selected += 1;
                    }
                    KeyCode::Enter => {
                        self.run_selected()?;
                    }
                    KeyCode::Esc => {
                        break;
                    }
                    _ => {}
                },
                _ => {}
            }
        }
        Ok(())
    }
}

fn main() -> io::Result<()> {
    // Set up terminal
    crossterm::terminal::enable_raw_mode()?;
    execute!(io::stdout(), Hide, DisableLineWrap)?;

    // Ensure cleanup happens even if we panic
    let result = std::panic::catch_unwind(|| {
        if let Ok(mut menu) = Menu::load_from_file(MENU_FILE) {
            menu.run()
        } else {
            Menu::new().show_error(format!("Failed to load {}", MENU_FILE).as_str())
        }
    });

    // Clean up terminal state
    execute!(io::stdout(), Show, EnableLineWrap)?;
    crossterm::terminal::disable_raw_mode()?;

    // Handle any errors or panics
    match result {
        Ok(Ok(())) => Ok(()),
        Ok(Err(e)) => Err(e),
        Err(_) => Err(io::Error::new(io::ErrorKind::Other, "Program panicked")),
    }
}