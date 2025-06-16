# Duca

<img width="550" alt="Screenshot 2025-06-16 at 10 23 20" src="https://github.com/user-attachments/assets/a1f0e139-dfc8-4146-bc05-549b9fb2c0f8" />

A terminal application for reading and searching Dante's *Divina Commedia*, inspired by the [kjv Bible reader](https://github.com/layeh/kjv). Built with Rust and featuring both CLI and TUI interfaces.

## Features

- **Interactive fuzzy search** like fzf with live filtering as you type
- **Context viewer** to see search results highlighted in their full canto
- **Fast text search** across all three cantiche (Inferno, Purgatorio, Paradiso)
- **CLI interface** for quick lookups and searches
- **Interactive TUI** for browsing with vim-like navigation
- **Precise referencing** by cantica, canto, and line number
- **Fuzzy matching** with intelligent scoring and ranking
- **Real-time search results** with instant feedback

## Installation

To build for development:

```bash
git clone <repository-url>
cd duca
cargo build --release
```

You can also install `duca` directly to your $PATH using Cargo:

```bash
cargo install --path .
```

## Usage

### Parse the text (first time setup - only for development)

```bash
duca parse
```

### Search for text

```bash
# Search across all canticas
duca search "selva"

# Search within specific cantica
duca search "selva" -c inferno
```

### Display specific canto

```bash
duca canto inferno 1
duca canto purgatorio 5
duca canto paradiso 33
```

### Interactive TUI mode

```bash
duca tui
```

#### TUI Navigation

**Browse Mode:**

- `h/←` `l/→` - Switch between cantiche
- `j/↓` `k/↑` - Navigate cantos
- `J` `K` - Scroll verses up/down
- `/` - Enter interactive search mode
- `Enter` - Select canto
- `q` - Quit

**Interactive Search Mode:**

- Type to filter results in real-time
- `j/k` or `↑/↓` - Navigate search results
- `Enter` - View result in context
- `Esc` - Return to browse mode

**Context View Mode:**

- `J/K` - Scroll through the canto
- Highlighted line shows your search match
- `Esc` - Return to search results

## Text Sources

The application uses the complete Italian text of Dante's Divine Comedy from Project Gutenberg:

- **Inferno**: eBook #997 (`inferno.txt`)
- **Purgatorio**: eBook #998 (`purgatorio.txt`)
- **Paradiso**: eBook #999 (`paradiso.txt`)

**Total**: 100 cantos (34 Inferno + 33 Purgatorio + 33 Paradiso)

## Architecture

- **Data Structure**: Hierarchical organization with Cantiche containing Cantos containing Verses
- **Parser**: Regex-based text parser that handles Roman numeral canto numbers
- **Search**: Fuzzy matching with SkimMatcher for intelligent ranking
- **Interactive UI**: Live search filtering with instant results like fzf
- **Context Viewing**: Full canto display with highlighted search matches
- **TUI**: Built with ratatui for responsive terminal interface
- **CLI**: Built with clap for command-line argument parsing

## File Structure

- `src/main.rs` - Main application logic, CLI interface, and text parser
- `src/tui.rs` - Interactive terminal UI with fuzzy search and context viewing
- `inferno.txt` - Inferno text (Project Gutenberg eBook #997)
- `purgatorio.txt` - Purgatorio text (Project Gutenberg eBook #998)
- `paradiso.txt` - Paradiso text (Project Gutenberg eBook #999)
- `commedia.json` - Parsed and structured text data (generated from all three files)

## Examples

```bash
# Find all mentions of "amore" across all canticas (59 matches found)
duca search "amore"

# Find "stelle" only in Paradiso (9 matches found)
duca search "stelle" -c paradiso

# Read the famous opening of Inferno
duca canto inferno 1

# Read the beginning of Purgatorio
duca canto purgatorio 1

# Read the final prayer to the Virgin (Paradiso's ending)
duca canto paradiso 33

# Interactive browsing with fzf-like search
duca tui
# Press / to enter search mode
# Type "amore" and see results filter in real-time
# Press Enter on a result to see it highlighted in context
```

## Backlog

- [ ] Ability to switch to an English translation.
