use clap::App;

use error::DedupError;

use std::path::PathBuf;

#[derive(Debug)]
pub struct Args {
    pub input: Option<PathBuf>,
    pub output: Option<PathBuf>,
    pub mmap: bool,
    pub terminator: u8,
    pub line_count: bool,
}

impl Args {
    pub fn parse() -> Result<Self, DedupError> {
        let yml = load_yaml!("../cli.yml");
        let m = App::from_yaml(yml).get_matches();

        let input = m.value_of("INPUT")
            .and_then(replace_with_stdout)
            .map(PathBuf::from);
        let output = m.value_of("OUTPUT").map(PathBuf::from);
        let mmap = !m.is_present("NO_MMAP");
        let terminator = m.value_of("TERMINATOR")
            .map_or(Ok(b'\n'), parse_to_byte_literal)?;
        let line_count = m.is_present("COUNTLINES");
        
        Ok(Args {
            input,
            output,
            mmap,
            terminator,
            line_count
        })
    }
}

pub struct Options {
    pub terminator: u8,
    pub num_threads: usize,
    pub line_count: bool,
    pub repeated: bool,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            terminator: b'\n',
            num_threads: 0,
            line_count: false,
            repeated: false,
        }
    }
}

impl From<Args> for Options {
    fn from(src: Args) -> Self {
        Options {
            terminator: src.terminator,
            num_threads: 0,
            line_count: src.line_count,
            repeated: false,
        }
    }
}

impl<'a> From<&'a Args> for Options {
    fn from(src: &'a Args) -> Self {
        Options {
            terminator: src.terminator,
            num_threads: 0,
            line_count: src.line_count,
            repeated: false,
        }
    }
}

fn parse_to_byte_literal(input: &str) -> Result<u8, DedupError> {
    if input.len() == 1 {
        return Ok(input.as_bytes()[0]);
    }
    if input.len() > 2 {
        return Err(DedupError::ArgumentParseError(format!(
            "Invalid delimiter specified, only single byte characters are permitted. Found: {}",
            input
        )));
    }

    let bytes = input.as_bytes();
    match (bytes[0], bytes[1]) {
        (b'\\', b'n') => Ok(b'\n'),
        (b'\\', b't') => Ok(b'\t'),
        (b'\\', b'0') => Ok(b'\0'),
        (b'\\', b'\\') => Ok(b'\\'),
        (b'\\', b'\'') => Ok(b'\''),
        (b'\\', b'"') => Ok(b'\"'),
        (_, _) => Err(DedupError::ArgumentParseError(format!(
            "Invalid delimiter specified, only single byte characters are permitted. Found: {}",
            input
        ))),
    }
}

fn replace_with_stdout(input: &str) -> Option<&str> {
    if input == "-" {
        None
    } else {
        Some(input)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn single_input_test() {
        let yml = load_yaml!("../cli.yml");
        let m = App::from_yaml(yml).get_matches_from(vec!["dedup", "inputfile"]);

        assert_eq!(m.value_of("INPUT"), Some("inputfile"));
    }

    #[test]
    fn no_mmap_test() {
        let yml = load_yaml!("../cli.yml");
        let m = App::from_yaml(yml).get_matches_from(vec!["dedup", "inputfile", "--no-mmap"]);

        assert!(m.is_present("NO_MMAP"));
    }

    #[test]
    fn input_output_test() {
        let yml = load_yaml!("../cli.yml");
        let m =
            App::from_yaml(yml).get_matches_from(vec!["dedup", "inputfile", "-o", "outputfile"]);

        assert_eq!(m.value_of("OUTPUT"), Some("outputfile"));
    }

    #[test]
    fn output_input_test() {
        let yml = load_yaml!("../cli.yml");
        let m =
            App::from_yaml(yml).get_matches_from(vec!["dedup", "-o", "outputfile", "inputfile"]);

        assert_eq!(m.value_of("OUTPUT"), Some("outputfile"));
    }

    #[test]
    fn specify_terminator_test() {
        let yml = load_yaml!("../cli.yml");
        let m = App::from_yaml(yml).get_matches_from(vec!["dedup", "--terminator", "\\t", "inputfile"]);

        assert!(m.is_present("TERMINATOR"));
        assert_eq!(
            parse_to_byte_literal(m.value_of("TERMINATOR").unwrap()).unwrap(),
            b'\t'
        );
    }

    #[test]
    fn unspecified_terminator_test() {
        let yml = load_yaml!("../cli.yml");
        let m = App::from_yaml(yml).get_matches_from(vec!["dedup", "inputfile"]);
        assert_eq!(m.value_of("TERMINATOR"), Some("\\n"));
    }

    #[test]
    fn implied_terminator_test() {
        let yml = load_yaml!("../cli.yml");
        let m = App::from_yaml(yml).get_matches_from(vec!["dedup", "inputfile", "-z"]);       
        assert_eq!(m.value_of("TERMINATOR"), Some("\\0"));
    }
}
