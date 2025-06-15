use anyhow::Result;
use clap::{Parser, Subcommand};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;

mod tui;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Verse {
    pub line_number: usize,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Canto {
    pub number: u8,
    pub roman_numeral: String,
    pub verses: Vec<Verse>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cantica {
    pub name: String,
    pub cantos: HashMap<u8, Canto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DivinaCommedia {
    pub inferno: Cantica,
    pub purgatorio: Cantica,
    pub paradiso: Cantica,
}

#[derive(Parser)]
#[command(name = "duca")]
#[command(about = "Read Dante's Divine Comedy from your terminal")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    #[command(about = "Search for text across all canticas")]
    Search {
        #[arg(help = "Pattern to search for")]
        pattern: String,
        #[arg(short, long, help = "Limit search to specific cantica")]
        cantica: Option<String>,
    },
    #[command(about = "Show specific canto")]
    Canto {
        #[arg(help = "Cantica (inferno, purgatorio, paradiso)")]
        cantica: String,
        #[arg(help = "Canto number")]
        number: u8,
    },
    #[command(about = "Interactive TUI mode")]
    Tui,
    #[cfg(debug_assertions)]
    #[command(about = "Parse and prepare text data (development only)")]
    Parse,
}

impl Default for DivinaCommedia {
    fn default() -> Self {
        Self::new()
    }
}

impl DivinaCommedia {
    pub fn new() -> Self {
        Self {
            inferno: Cantica {
                name: "Inferno".to_string(),
                cantos: HashMap::new(),
            },
            purgatorio: Cantica {
                name: "Purgatorio".to_string(),
                cantos: HashMap::new(),
            },
            paradiso: Cantica {
                name: "Paradiso".to_string(),
                cantos: HashMap::new(),
            },
        }
    }

    pub fn search(
        &self,
        pattern: &str,
        cantica_filter: Option<&str>,
    ) -> Vec<(String, u8, usize, String)> {
        let regex = Regex::new(&format!("(?i){}", pattern))
            .unwrap_or_else(|_| Regex::new(&regex::escape(pattern)).unwrap());

        let mut results = Vec::new();

        let canticas = match cantica_filter {
            Some("inferno") => vec![&self.inferno],
            Some("purgatorio") => vec![&self.purgatorio],
            Some("paradiso") => vec![&self.paradiso],
            _ => vec![&self.inferno, &self.purgatorio, &self.paradiso],
        };

        for cantica in canticas {
            // Sort cantos by number to ensure consistent ordering
            let mut canto_numbers: Vec<_> = cantica.cantos.keys().collect();
            canto_numbers.sort();

            for &canto_number in canto_numbers {
                let canto = &cantica.cantos[&canto_number];
                for verse in &canto.verses {
                    if regex.is_match(&verse.text) {
                        results.push((
                            cantica.name.clone(),
                            canto.number,
                            verse.line_number,
                            verse.text.clone(),
                        ));
                    }
                }
            }
        }

        // Sort results by cantica order (Inferno, Purgatorio, Paradiso), then canto, then line
        results.sort_by(|a, b| {
            // First compare by cantica order
            let cantica_order = |name: &str| match name {
                "Inferno" => 0,
                "Purgatorio" => 1,
                "Paradiso" => 2,
                _ => 3,
            };

            let cantica_cmp = cantica_order(&a.0).cmp(&cantica_order(&b.0));
            if cantica_cmp != std::cmp::Ordering::Equal {
                return cantica_cmp;
            }

            // Then compare by canto number
            let canto_cmp = a.1.cmp(&b.1);
            if canto_cmp != std::cmp::Ordering::Equal {
                return canto_cmp;
            }

            // Finally compare by line number
            a.2.cmp(&b.2)
        });

        results
    }
}

fn parse_text_files() -> Result<DivinaCommedia> {
    let mut commedia = DivinaCommedia::new();

    // Parse each cantica from separate files
    let files = [
        ("inferno.txt", "inferno"),
        ("purgatorio.txt", "purgatorio"),
        ("paradiso.txt", "paradiso"),
    ];

    for (filename, cantica_name) in files {
        if let Ok(content) = fs::read_to_string(filename) {
            parse_cantica_content(&content, cantica_name, &mut commedia)?;
        }
    }

    Ok(commedia)
}

fn parse_cantica_content(
    content: &str,
    cantica_name: &str,
    commedia: &mut DivinaCommedia,
) -> Result<()> {
    let lines: Vec<&str> = content.lines().collect();
    let mut current_canto_number = 0u8;
    let mut current_verses = Vec::new();
    let mut line_number_in_canto = 0usize;
    let mut in_canto = false;

    let canto_regex = Regex::new(r"^Canto\s+([IVXLCDM]+)\.?$").unwrap();

    for line in lines {
        let trimmed = line.trim();

        if trimmed.is_empty() {
            continue;
        }

        // Stop parsing when we hit the Gutenberg end marker
        if trimmed.starts_with("Updated editions will replace") {
            break;
        }

        if let Some(caps) = canto_regex.captures(trimmed) {
            // Save previous canto if exists
            if in_canto && current_canto_number > 0 {
                let canto = Canto {
                    number: current_canto_number,
                    roman_numeral: roman_to_number(current_canto_number),
                    verses: current_verses.clone(),
                };

                match cantica_name {
                    "inferno" => {
                        commedia.inferno.cantos.insert(current_canto_number, canto);
                    }
                    "purgatorio" => {
                        commedia
                            .purgatorio
                            .cantos
                            .insert(current_canto_number, canto);
                    }
                    "paradiso" => {
                        commedia.paradiso.cantos.insert(current_canto_number, canto);
                    }
                    _ => {}
                }
            }

            let roman = caps.get(1).unwrap().as_str();
            current_canto_number = roman_to_arabic(roman);
            current_verses.clear();
            line_number_in_canto = 0;
            in_canto = true;
            continue;
        }

        if in_canto && !trimmed.starts_with("*** ") && !trimmed.contains("Project Gutenberg") {
            line_number_in_canto += 1;
            current_verses.push(Verse {
                line_number: line_number_in_canto,
                text: trimmed.to_string(),
            });
        }
    }

    // Save last canto
    if in_canto && current_canto_number > 0 {
        let canto = Canto {
            number: current_canto_number,
            roman_numeral: roman_to_number(current_canto_number),
            verses: current_verses,
        };

        match cantica_name {
            "inferno" => {
                commedia.inferno.cantos.insert(current_canto_number, canto);
            }
            "purgatorio" => {
                commedia
                    .purgatorio
                    .cantos
                    .insert(current_canto_number, canto);
            }
            "paradiso" => {
                commedia.paradiso.cantos.insert(current_canto_number, canto);
            }
            _ => {}
        }
    }

    Ok(())
}

fn roman_to_arabic(roman: &str) -> u8 {
    let mut result = 0;
    let mut prev_value = 0;

    for c in roman.chars().rev() {
        let value = match c {
            'I' => 1,
            'V' => 5,
            'X' => 10,
            'L' => 50,
            'C' => 100,
            'D' => 500,
            'M' => 1000,
            _ => 0,
        };

        if value < prev_value {
            result -= value;
        } else {
            result += value;
        }
        prev_value = value;
    }

    result as u8
}

fn roman_to_number(num: u8) -> String {
    let values = [
        (1000, "M"),
        (900, "CM"),
        (500, "D"),
        (400, "CD"),
        (100, "C"),
        (90, "XC"),
        (50, "L"),
        (40, "XL"),
        (10, "X"),
        (9, "IX"),
        (5, "V"),
        (4, "IV"),
        (1, "I"),
    ];

    let mut result = String::new();
    let mut n = num as usize;

    for &(value, numeral) in &values {
        while n >= value {
            result.push_str(numeral);
            n -= value;
        }
    }

    result
}

fn load_commedia() -> Result<DivinaCommedia> {
    // Try to load from embedded data first, then fall back to external files
    const EMBEDDED_DATA: &str = include_str!("../commedia.json");

    if !EMBEDDED_DATA.trim().is_empty() {
        serde_json::from_str(EMBEDDED_DATA).map_err(|e| e.into())
    } else if fs::metadata("commedia.json").is_ok() {
        let json = fs::read_to_string("commedia.json")?;
        serde_json::from_str(&json).map_err(|e| e.into())
    } else {
        parse_text_files()
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        #[cfg(debug_assertions)]
        Commands::Parse => {
            println!("Parsing Divine Comedy text from all three files...");
            let commedia = parse_text_files()?;

            let json = serde_json::to_string_pretty(&commedia)?;
            fs::write("commedia.json", json)?;

            println!("Parsed and saved to commedia.json");
            println!("Inferno cantos: {}", commedia.inferno.cantos.len());
            println!("Purgatorio cantos: {}", commedia.purgatorio.cantos.len());
            println!("Paradiso cantos: {}", commedia.paradiso.cantos.len());
        }

        Commands::Search { pattern, cantica } => {
            let commedia = load_commedia()?;

            let results = commedia.search(&pattern, cantica.as_deref());

            if results.is_empty() {
                println!("No matches found for '{}'", pattern);
            } else {
                println!("Found {} matches for '{}':\n", results.len(), pattern);
                for (cantica_name, canto_num, line_num, text) in results {
                    println!("{} {}.{}: {}", cantica_name, canto_num, line_num, text);
                }
            }
        }

        Commands::Canto { cantica, number } => {
            let commedia = load_commedia()?;

            let cantica_data = match cantica.to_lowercase().as_str() {
                "inferno" => &commedia.inferno,
                "purgatorio" => &commedia.purgatorio,
                "paradiso" => &commedia.paradiso,
                _ => {
                    eprintln!("Invalid cantica. Use: inferno, purgatorio, or paradiso");
                    return Ok(());
                }
            };

            if let Some(canto) = cantica_data.cantos.get(&number) {
                println!("{} Canto {}\n", cantica_data.name, canto.roman_numeral);
                for verse in &canto.verses {
                    println!("{:3}: {}", verse.line_number, verse.text);
                }
            } else {
                println!("Canto {} not found in {}", number, cantica_data.name);
            }
        }

        Commands::Tui => {
            let commedia = load_commedia()?;

            tui::run_tui(commedia)?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_roman_to_arabic() {
        assert_eq!(roman_to_arabic("I"), 1);
        assert_eq!(roman_to_arabic("II"), 2);
        assert_eq!(roman_to_arabic("III"), 3);
        assert_eq!(roman_to_arabic("IV"), 4);
        assert_eq!(roman_to_arabic("V"), 5);
        assert_eq!(roman_to_arabic("IX"), 9);
        assert_eq!(roman_to_arabic("X"), 10);
        assert_eq!(roman_to_arabic("XIV"), 14);
        assert_eq!(roman_to_arabic("XIX"), 19);
        assert_eq!(roman_to_arabic("XX"), 20);
        assert_eq!(roman_to_arabic("XXXIII"), 33);
        assert_eq!(roman_to_arabic("XXXIV"), 34);
    }

    #[test]
    fn test_roman_to_number() {
        assert_eq!(roman_to_number(1), "I");
        assert_eq!(roman_to_number(2), "II");
        assert_eq!(roman_to_number(3), "III");
        assert_eq!(roman_to_number(4), "IV");
        assert_eq!(roman_to_number(5), "V");
        assert_eq!(roman_to_number(9), "IX");
        assert_eq!(roman_to_number(10), "X");
        assert_eq!(roman_to_number(14), "XIV");
        assert_eq!(roman_to_number(19), "XIX");
        assert_eq!(roman_to_number(20), "XX");
        assert_eq!(roman_to_number(33), "XXXIII");
        assert_eq!(roman_to_number(34), "XXXIV");
    }

    #[test]
    fn test_divina_commedia_new() {
        let commedia = DivinaCommedia::new();
        assert_eq!(commedia.inferno.name, "Inferno");
        assert_eq!(commedia.purgatorio.name, "Purgatorio");
        assert_eq!(commedia.paradiso.name, "Paradiso");
        assert!(commedia.inferno.cantos.is_empty());
        assert!(commedia.purgatorio.cantos.is_empty());
        assert!(commedia.paradiso.cantos.is_empty());
    }

    #[test]
    fn test_search_functionality() {
        let mut commedia = DivinaCommedia::new();

        // Add test data
        let canto = Canto {
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
        commedia.inferno.cantos.insert(1, canto);

        // Test search
        let results = commedia.search("selva", None);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, "Inferno");
        assert_eq!(results[0].1, 1);
        assert_eq!(results[0].2, 2);
        assert!(results[0].3.contains("selva"));

        // Test case insensitive search
        let results = commedia.search("SELVA", None);
        assert_eq!(results.len(), 1);

        // Test no matches
        let results = commedia.search("nonexistent", None);
        assert_eq!(results.len(), 0);

        // Test cantica filter
        let results = commedia.search("selva", Some("purgatorio"));
        assert_eq!(results.len(), 0);

        let results = commedia.search("selva", Some("inferno"));
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_parse_cantica_content() {
        let sample_text = r#"
Some header text
*** START OF THE PROJECT GUTENBERG EBOOK ***

Canto I

Nel mezzo del cammin di nostra vita
mi ritrovai per una selva oscura
ché la diritta via era smarrita.

Canto II

Per me si va ne la città dolente,
per me si va ne l'etterno dolore,
per me si va tra la perduta gente.

Updated editions will replace the previous one
This should be ignored
"#;

        let mut commedia = DivinaCommedia::new();
        let result = parse_cantica_content(sample_text, "inferno", &mut commedia);

        assert!(result.is_ok());
        assert_eq!(commedia.inferno.cantos.len(), 2);

        let canto1 = commedia.inferno.cantos.get(&1).unwrap();
        assert_eq!(canto1.number, 1);
        assert_eq!(canto1.roman_numeral, "I");
        assert_eq!(canto1.verses.len(), 3);
        assert!(canto1.verses[0].text.contains("Nel mezzo"));

        let canto2 = commedia.inferno.cantos.get(&2).unwrap();
        assert_eq!(canto2.number, 2);
        assert_eq!(canto2.roman_numeral, "II");
        assert_eq!(canto2.verses.len(), 3);
        assert!(canto2.verses[0].text.contains("Per me si va"));
    }

    #[test]
    fn test_verse_and_canto_structures() {
        let verse = Verse {
            line_number: 42,
            text: "Test verse text".to_string(),
        };
        assert_eq!(verse.line_number, 42);
        assert_eq!(verse.text, "Test verse text");

        let canto = Canto {
            number: 5,
            roman_numeral: "V".to_string(),
            verses: vec![verse],
        };
        assert_eq!(canto.number, 5);
        assert_eq!(canto.roman_numeral, "V");
        assert_eq!(canto.verses.len(), 1);
    }

    #[test]
    fn test_regex_patterns() {
        let canto_regex = regex::Regex::new(r"^Canto\s+([IVXLCDM]+)\.?$").unwrap();

        assert!(canto_regex.is_match("Canto I"));
        assert!(canto_regex.is_match("Canto II"));
        assert!(canto_regex.is_match("Canto XXXIII"));
        assert!(canto_regex.is_match("Canto XIV."));

        assert!(!canto_regex.is_match("canto i"));
        assert!(!canto_regex.is_match("Canto 1"));
        assert!(!canto_regex.is_match("Cantoi"));
        assert!(!canto_regex.is_match("Some other text"));
    }

    #[test]
    fn test_gutenberg_marker_detection() {
        let test_lines = vec![
            "Normal verse text",
            "Updated editions will replace the previous one",
            "This should not be parsed",
        ];

        // Simulate the parsing loop logic
        let mut should_continue = true;
        for line in test_lines {
            if line.starts_with("Updated editions will replace") {
                should_continue = false;
                break;
            }
        }

        assert!(!should_continue);
    }

    #[test]
    fn test_load_commedia() {
        // Test that load_commedia works with embedded data
        let result = load_commedia();
        assert!(result.is_ok());

        let commedia = result.unwrap();
        assert_eq!(commedia.inferno.name, "Inferno");
        assert_eq!(commedia.purgatorio.name, "Purgatorio");
        assert_eq!(commedia.paradiso.name, "Paradiso");

        // Should have the expected number of cantos
        assert!(commedia.inferno.cantos.len() > 30); // Expecting 34
        assert!(commedia.purgatorio.cantos.len() > 30); // Expecting 33
        assert!(commedia.paradiso.cantos.len() > 30); // Expecting 33
    }

    #[test]
    fn test_search_results_ordering() {
        let mut commedia = DivinaCommedia::new();

        // Add test data with specific ordering to verify sorting
        // Canto 3 comes before Canto 1 in creation order to test sorting
        let canto3 = Canto {
            number: 3,
            roman_numeral: "III".to_string(),
            verses: vec![
                Verse {
                    line_number: 1,
                    text: "test third canto first verse".to_string(),
                },
                Verse {
                    line_number: 5,
                    text: "test third canto fifth verse".to_string(),
                },
            ],
        };
        commedia.inferno.cantos.insert(3, canto3);

        let canto1 = Canto {
            number: 1,
            roman_numeral: "I".to_string(),
            verses: vec![
                Verse {
                    line_number: 2,
                    text: "test first canto second verse".to_string(),
                },
                Verse {
                    line_number: 1,
                    text: "test first canto first verse".to_string(),
                },
            ],
        };
        commedia.inferno.cantos.insert(1, canto1);

        let canto2 = Canto {
            number: 2,
            roman_numeral: "II".to_string(),
            verses: vec![Verse {
                line_number: 1,
                text: "test second canto first verse".to_string(),
            }],
        };
        commedia.inferno.cantos.insert(2, canto2);

        // Search for "test" which should match all verses
        let results = commedia.search("test", None);

        // Results should be ordered by canto number, then by line number
        assert_eq!(results.len(), 5);

        // Check ordering: should be sorted by (cantica, canto, line)
        assert_eq!(
            results[0],
            (
                "Inferno".to_string(),
                1,
                1,
                "test first canto first verse".to_string()
            )
        );
        assert_eq!(
            results[1],
            (
                "Inferno".to_string(),
                1,
                2,
                "test first canto second verse".to_string()
            )
        );
        assert_eq!(
            results[2],
            (
                "Inferno".to_string(),
                2,
                1,
                "test second canto first verse".to_string()
            )
        );
        assert_eq!(
            results[3],
            (
                "Inferno".to_string(),
                3,
                1,
                "test third canto first verse".to_string()
            )
        );
        assert_eq!(
            results[4],
            (
                "Inferno".to_string(),
                3,
                5,
                "test third canto fifth verse".to_string()
            )
        );
    }

    #[test]
    fn test_search_results_cross_cantica_ordering() {
        let mut commedia = DivinaCommedia::new();

        // Add test data across multiple canticas to verify cross-cantica sorting
        let paradiso_canto1 = Canto {
            number: 1,
            roman_numeral: "I".to_string(),
            verses: vec![Verse {
                line_number: 1,
                text: "test paradiso canto one".to_string(),
            }],
        };
        commedia.paradiso.cantos.insert(1, paradiso_canto1);

        let inferno_canto2 = Canto {
            number: 2,
            roman_numeral: "II".to_string(),
            verses: vec![Verse {
                line_number: 1,
                text: "test inferno canto two".to_string(),
            }],
        };
        commedia.inferno.cantos.insert(2, inferno_canto2);

        let purgatorio_canto1 = Canto {
            number: 1,
            roman_numeral: "I".to_string(),
            verses: vec![
                Verse {
                    line_number: 3,
                    text: "test purgatorio canto one".to_string(),
                },
                Verse {
                    line_number: 1,
                    text: "test purgatorio canto one first".to_string(),
                },
            ],
        };
        commedia.purgatorio.cantos.insert(1, purgatorio_canto1);

        let inferno_canto1 = Canto {
            number: 1,
            roman_numeral: "I".to_string(),
            verses: vec![Verse {
                line_number: 2,
                text: "test inferno canto one".to_string(),
            }],
        };
        commedia.inferno.cantos.insert(1, inferno_canto1);

        // Search for "test" which should match all verses
        let results = commedia.search("test", None);

        assert_eq!(results.len(), 5);

        // Results should be ordered: Inferno (1.2, 2.1), Purgatorio (1.1, 1.3), Paradiso (1.1)
        assert_eq!(
            results[0],
            (
                "Inferno".to_string(),
                1,
                2,
                "test inferno canto one".to_string()
            )
        );
        assert_eq!(
            results[1],
            (
                "Inferno".to_string(),
                2,
                1,
                "test inferno canto two".to_string()
            )
        );
        assert_eq!(
            results[2],
            (
                "Purgatorio".to_string(),
                1,
                1,
                "test purgatorio canto one first".to_string()
            )
        );
        assert_eq!(
            results[3],
            (
                "Purgatorio".to_string(),
                1,
                3,
                "test purgatorio canto one".to_string()
            )
        );
        assert_eq!(
            results[4],
            (
                "Paradiso".to_string(),
                1,
                1,
                "test paradiso canto one".to_string()
            )
        );
    }
}
