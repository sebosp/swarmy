//! The contract of the query files, checked by `cargo test` (AGENTS.md, "A query is a file"):
//! a file that declares `query_meta!()` is a query, and a query has its README and its tests next
//! to it, in the directory of its name. No module is a `mod.rs` (clippy's `mod_module_files`
//! says it too), so that the path of a module is the address of its file.

use std::path::{Path, PathBuf};

/// The `src` directories of the three crates, from this one.
fn source_roots() -> [PathBuf; 3] {
    let tauri = Path::new(env!("CARGO_MANIFEST_DIR"));
    [
        tauri.join("src"),
        tauri.join("../src"),
        tauri.join("../swarmy-common/src"),
    ]
}

fn files_under(dir: &Path, found: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap_or_else(|err| panic!("{}: {err}", dir.display())) {
        let path = entry.unwrap().path();
        if path.is_dir() {
            files_under(&path, found);
        } else {
            found.push(path);
        }
    }
}

fn relative(path: &Path) -> String {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let root = root.canonicalize().unwrap();
    path.canonicalize()
        .unwrap()
        .strip_prefix(&root)
        .unwrap()
        .display()
        .to_string()
}

#[test]
fn no_module_is_a_mod_rs() {
    let mut files = vec![];
    for root in source_roots() {
        files_under(&root, &mut files);
    }
    let offenders: Vec<String> = files
        .iter()
        .filter(|path| path.file_name().is_some_and(|name| name == "mod.rs"))
        .map(|path| relative(path))
        .collect();
    assert!(
        offenders.is_empty(),
        "modules are `name.rs` beside `name/`, never `mod.rs` (git mv it to the parent directory): {offenders:?}"
    );
}

/// A query file (`query_meta!()`) has `<name>/README.md` (with a ```sql fence or the sentence
/// "SQL: not expressible") included as its documentation, and `<name>/tests.rs` declared as its
/// tests; a directory with a README belongs to such a file.
#[test]
fn every_query_file_has_its_readme_and_tests() {
    let mut files = vec![];
    files_under(&source_roots()[0], &mut files);
    let mut problems = vec![];
    let mut query_dirs = std::collections::HashSet::new();
    for path in files
        .iter()
        .filter(|path| path.extension().is_some_and(|ext| ext == "rs"))
    {
        let source = std::fs::read_to_string(path).unwrap();
        // A whole line, so that this file and the documentation of the macro do not count.
        let declares_meta = source
            .lines()
            .any(|line| line.trim() == "pub const META: QueryMeta = query_meta!();");
        if !declares_meta {
            continue;
        }
        let name = relative(path);
        let dir = path.with_extension("");
        query_dirs.insert(dir.clone());
        let readme = dir.join("README.md");
        let stem = path.file_stem().unwrap().to_str().unwrap();
        match std::fs::read_to_string(&readme) {
            Err(_) => problems.push(format!("{name}: no {stem}/README.md")),
            Ok(readme) => {
                if !readme.contains("```sql") && !readme.contains("SQL: not expressible") {
                    problems.push(format!(
                        "{name}: {stem}/README.md has no ```sql fence and does not say \"SQL: not expressible\""
                    ));
                }
                for heading in [
                    "## Purpose",
                    "## Inputs",
                    "## Output",
                    "## Query",
                    "## Example",
                ] {
                    if !readme.contains(heading) {
                        problems.push(format!("{name}: {stem}/README.md has no \"{heading}\""));
                    }
                }
            }
        }
        let include = format!("#![doc = include_str!(\"{stem}/README.md\")]");
        if !source.contains(&include) {
            problems.push(format!("{name}: does not start with {include}"));
        }
        if !dir.join("tests.rs").is_file() {
            problems.push(format!("{name}: no {stem}/tests.rs"));
        }
        if !source.contains("mod tests;") {
            problems.push(format!("{name}: no `#[cfg(test)]\\nmod tests;`"));
        }
        if !source.contains("pub fn plan(") {
            problems.push(format!(
                "{name}: no `pub fn plan(`, the LazyFrame of the query"
            ));
        }
    }
    // A README in a directory of src-tauri/src is the one of a query file, no orphans.
    for path in files
        .iter()
        .filter(|path| path.file_name().is_some_and(|f| f == "README.md"))
    {
        let dir = path.parent().unwrap().to_path_buf();
        if !query_dirs.contains(&dir) {
            problems.push(format!(
                "{}: no query file {}.rs with `query_meta!()` beside it",
                relative(path),
                dir.file_name().unwrap().to_str().unwrap()
            ));
        }
    }
    assert!(problems.is_empty(), "\n{}", problems.join("\n"));
}
