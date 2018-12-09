extern crate ra_syntax;
#[macro_use]
extern crate test_utils;
extern crate walkdir;

use std::{
    fmt::Write,
    fs,
    path::{Path, PathBuf, Component},
};

use ra_syntax::{
    utils::{check_fuzz_invariants, dump_tree},
    SourceFileNode,
};

#[test]
fn lexer_tests() {
    dir_tests(&["lexer"], |text| {
        let tokens = ra_syntax::tokenize(text);
        dump_tokens(&tokens, text)
    })
}

#[test]
fn parser_tests() {
    dir_tests(&["parser/inline", "parser/ok", "parser/err"], |text| {
        let file = SourceFileNode::parse(text);
        dump_tree(file.syntax())
    })
}

#[test]
fn parser_fuzz_tests() {
    for (_, text) in collect_tests(&["parser/fuzz-failures"]) {
        check_fuzz_invariants(&text)
    }
}

/// Test that Rust-analyzer can parse and validate the rust-analyser
/// TODO: Use this as a benchmark
#[test]
fn self_hosting_parsing() {
    let empty_vec = vec![];
    let dir = project_dir();
    let mut count = 0u32;
    for entry in walkdir::WalkDir::new(dir)
        .into_iter()
        .filter_entry(|entry| {
            !entry
                .path()
                .components()
                // TODO: this more neatly
                .any(|component| {
                    // Get all files which are not in the crates/ra_syntax/tests/data folder
                    (component == Component::Normal(std::ffi::OsStr::new("data"))
                    // or the .git folder
                        || component == Component::Normal(std::ffi::OsStr::new(".git")))
                })
        })
        .map(|e| e.unwrap())
        .filter(|entry| {
            // Get all `.rs ` files
            !entry.path().is_dir() && (entry.path().extension() == Some(std::ffi::OsStr::new("rs")))
        })
    {
        count += 1;
        let text = read_text(entry.path());
        let node = SourceFileNode::parse(&text);
        let errors = node.errors();
        assert_eq!(
            errors, empty_vec,
            "There should be no errors in the file {:?}",
            entry
        );
    }
    panic!("{}", count)
}
/// Read file and normalize newlines.
///
/// `rustc` seems to always normalize `\r\n` newlines to `\n`:
///
/// ```
/// let s = "
/// ";
/// assert_eq!(s.as_bytes(), &[10]);
/// ```
///
/// so this should always be correct.
fn read_text(path: &Path) -> String {
    fs::read_to_string(path)
        .expect(&format!("File at {:?} should be valid", path))
        .replace("\r\n", "\n")
}

pub fn dir_tests<F>(paths: &[&str], f: F)
where
    F: Fn(&str) -> String,
{
    for (path, input_code) in collect_tests(paths) {
        let parse_tree = f(&input_code);
        let path = path.with_extension("txt");
        if !path.exists() {
            println!("\nfile: {}", path.display());
            println!("No .txt file with expected result, creating...\n");
            println!("{}\n{}", input_code, parse_tree);
            fs::write(&path, parse_tree).unwrap();
            panic!("No expected result")
        }
        let expected = read_text(&path);
        let expected = expected.as_str();
        let parse_tree = parse_tree.as_str();
        assert_equal_text(expected, parse_tree, &path);
    }
}

const REWRITE: bool = false;

fn assert_equal_text(expected: &str, actual: &str, path: &Path) {
    if expected == actual {
        return;
    }
    let dir = project_dir();
    let pretty_path = path.strip_prefix(&dir).unwrap_or_else(|_| path);
    if expected.trim() == actual.trim() {
        println!("whitespace difference, rewriting");
        println!("file: {}\n", pretty_path.display());
        fs::write(path, actual).unwrap();
        return;
    }
    if REWRITE {
        println!("rewriting {}", pretty_path.display());
        fs::write(path, actual).unwrap();
        return;
    }
    assert_eq_text!(expected, actual, "file: {}", pretty_path.display());
}

fn collect_tests(paths: &[&str]) -> Vec<(PathBuf, String)> {
    paths
        .iter()
        .flat_map(|path| {
            let path = test_data_dir().join(path);
            test_from_dir(&path).into_iter()
        })
        .map(|path| {
            let text = read_text(&path);
            (path, text)
        })
        .collect()
}

fn test_from_dir(dir: &Path) -> Vec<PathBuf> {
    let mut acc = Vec::new();
    for file in fs::read_dir(&dir).unwrap() {
        let file = file.unwrap();
        let path = file.path();
        if path.extension().unwrap_or_default() == "rs" {
            acc.push(path);
        }
    }
    acc.sort();
    acc
}

fn project_dir() -> PathBuf {
    let dir = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(dir)
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_owned()
}

fn test_data_dir() -> PathBuf {
    project_dir().join("crates/ra_syntax/tests/data")
}

fn dump_tokens(tokens: &[ra_syntax::Token], text: &str) -> String {
    let mut acc = String::new();
    let mut offset = 0;
    for token in tokens {
        let len: u32 = token.len.into();
        let len = len as usize;
        let token_text = &text[offset..offset + len];
        offset += len;
        write!(acc, "{:?} {} {:?}\n", token.kind, token.len, token_text).unwrap()
    }
    acc
}
