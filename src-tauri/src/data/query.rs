//! The identity of a query: the module and the file it lives in, written by the compiler. Every
//! query file has `pub const META: QueryMeta = query_meta!();` (AGENTS.md, "A query is a file"):
//! the views attach it to their answers for the "Show Polars Debug Information" links, and the
//! contract test of [`crate::data::contract`] finds the query files by it.

/// Where a query lives: `id` is its module path (`swarmy_tauri_lib::map_details::recent_games`),
/// `file` and `line` the source file of the `META` constant, as the compiler saw them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub struct QueryMeta {
    pub id: &'static str,
    pub file: &'static str,
    pub line: u32,
}

impl QueryMeta {
    /// The path of the file in the repository, `src-tauri/src/map_details/recent_games.rs`:
    /// `file!()` is relative to the workspace when cargo builds the workspace, absolute in some
    /// builds; either way the path after `src-tauri/` is the one of the repository.
    pub fn repo_path(&self) -> String {
        let file = self.file.replace('\\', "/");
        match file.find("src-tauri/") {
            Some(start) => file[start..].to_string(),
            None => file,
        }
    }

    /// The module path without the crate, `map_details::recent_games`.
    pub fn short_id(&self) -> &'static str {
        self.id
            .split_once("::")
            .map(|(_, rest)| rest)
            .unwrap_or(self.id)
    }
}

/// The query as the answers name it, `ResponseMeta::sources`.
impl From<QueryMeta> for swarmy_common::QuerySource {
    fn from(meta: QueryMeta) -> Self {
        Self {
            id: meta.short_id().to_string(),
            file: meta.repo_path(),
            line: meta.line,
        }
    }
}

/// The [`QueryMeta`] of the module that expands it. Written once at the top of each query file.
#[macro_export]
macro_rules! query_meta {
    () => {
        $crate::data::QueryMeta {
            id: module_path!(),
            file: file!(),
            line: line!(),
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    const META: QueryMeta = query_meta!();
    const META_LINE: u32 = line!() - 1;

    #[test]
    fn the_meta_names_this_module_and_file() {
        assert_eq!(META.id, "swarmy_tauri_lib::data::query::tests");
        assert_eq!(META.short_id(), "data::query::tests");
        assert_eq!(META.repo_path(), "src-tauri/src/data/query.rs");
        // The line of the `query_meta!()` above, not of the module.
        assert_eq!(META.line, META_LINE);
    }

    #[test]
    fn a_windows_or_absolute_file_still_gives_the_repository_path() {
        let meta = QueryMeta {
            id: "swarmy_tauri_lib::x",
            file: "C:\\build\\swarmy\\src-tauri\\src\\x.rs",
            line: 1,
        };
        assert_eq!(meta.repo_path(), "src-tauri/src/x.rs");
    }
}
