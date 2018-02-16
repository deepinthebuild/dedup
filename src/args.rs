use clap::App;

use error::DedupError;

use std::path::PathBuf;

#[derive(Debug)]
pub struct Args {
    pub input: Option<PathBuf>,
    pub output: Option<PathBuf>,
    pub mmap: bool,
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

        Ok(Args {
            input,
            output,
            mmap,
        })
    }
}

pub struct Options {
    pub crlf: bool,
    pub delim: u8,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            crlf: true,
            delim: b'\n',
        }
    }
}

impl From<Args> for Options {
    fn from(src: Args) -> Self {
        Self::default()
    }
}

impl<'a> From<&'a Args> for Options {
    fn from(src: &'a Args) -> Self {
        Self::default()
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
}
