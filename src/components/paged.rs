//! The long tables: they show a page of rows, the rows per page are a setting of Config (100 by
//! default), and the page controls are shown only when there are more rows than that. Their
//! headers sort all the rows, not only the page, and the sort is remembered like the other tables
//! (see `public/table_sort.js`, which leaves these tables alone: `data-server-sort`).

use crate::common::storage;
use leptos::prelude::*;
use swarmy_common::DEFAULT_MAX_TABLE_ROWS;

thread_local! {
    /// The rows per page of the tables, from Config: one for the window, set when the settings
    /// are read, also from async tasks without a reactive owner.
    static TABLE_ROWS: RwSignal<usize, LocalStorage> = RwSignal::new_local(DEFAULT_MAX_TABLE_ROWS as usize);
}

/// Sets the rows per page, 0 (a setting not saved yet) is the default.
pub fn set_table_rows(rows: u32) {
    let rows = if rows == 0 {
        DEFAULT_MAX_TABLE_ROWS
    } else {
        rows
    };
    TABLE_ROWS.with(|signal| signal.set(rows as usize));
    // The other tables are paged by `public/table_sort.js`.
    crate::components::appearance::set_html_attribute("data-table-rows", Some(&rows.to_string()));
}

/// The rows of a page of the tables, also the most suggestions of a name field.
pub fn page_rows() -> usize {
    TABLE_ROWS.with(|signal| signal.get()).max(1)
}

fn page_size() -> usize {
    page_rows()
}

/// The value a column sorts by.
#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum SortKey {
    Number(f64),
    Text(String),
}

/// The sort of a paged table, `(column, descending)`, remembered under its key.
#[derive(Debug, Clone, Copy)]
pub struct PagedSort {
    pub sort: RwSignal<(usize, bool)>,
    pub page: RwSignal<usize>,
    key: &'static str,
}

impl PagedSort {
    /// The sort remembered for the table `key`, else `default`.
    pub fn new(key: &'static str, default: (usize, bool)) -> Self {
        let storage_key = format!("swarmy.paged.{key}");
        let sort = RwSignal::new(storage::load_json(&storage_key).unwrap_or(default));
        Self {
            sort,
            page: RwSignal::new(0),
            key,
        }
    }

    /// Sorts by the column, again reverses it; back to the first page.
    pub fn toggle(&self, column: usize, descending_first: bool) {
        self.sort.update(|(current, descending)| {
            if *current == column {
                *descending = !*descending;
            } else {
                *current = column;
                *descending = descending_first;
            }
        });
        self.page.set(0);
        storage::save_json(
            &format!("swarmy.paged.{}", self.key),
            &self.sort.get_untracked(),
        );
    }
}

/// The rows of the current page, sorted by the column of `sort` with `key`.
pub fn page_of<T: Clone>(
    rows: &[T],
    sort: PagedSort,
    key: impl Fn(&T, usize) -> SortKey,
) -> (Vec<T>, usize) {
    let (column, descending) = sort.sort.get();
    let mut sorted: Vec<&T> = rows.iter().collect();
    sorted.sort_by(|a, b| {
        let order = key(a, column)
            .partial_cmp(&key(b, column))
            .unwrap_or(std::cmp::Ordering::Equal);
        if descending {
            order.reverse()
        } else {
            order
        }
    });
    let size = page_size();
    let pages = rows.len().div_ceil(size).max(1);
    let page = sort.page.get().min(pages - 1);
    let page_rows = sorted
        .into_iter()
        .skip(page * size)
        .take(size)
        .cloned()
        .collect();
    (page_rows, pages)
}

/// A header that sorts a paged table: ↕ until it sorts, then ▲ or ▼ (the styles of
/// `thead th.sortable`).
#[component]
pub fn SortTh(
    label: &'static str,
    column: usize,
    sort: PagedSort,
    /// Numbers and dates sort the largest first.
    #[prop(optional)]
    descending_first: bool,
    #[prop(default = "pr-2")] class: &'static str,
) -> impl IntoView {
    let aria = move || {
        let (current, descending) = sort.sort.get();
        (current == column).then_some(if descending {
            "descending"
        } else {
            "ascending"
        })
    };
    view! {
        <th
            class=format!("sortable cursor-pointer select-none hover:text-base-content {class}")
            aria-sort=aria
            title="Sort all the rows by this column, click again to reverse"
            on:click=move |_| sort.toggle(column, descending_first)
        >
            {label}
        </th>
    }
}

/// The page controls, shown only when the rows do not fit in one page.
#[component]
pub fn Pager(#[prop(into)] rows: Signal<usize>, sort: PagedSort) -> impl IntoView {
    move || {
        let rows = rows.get();
        let size = page_size();
        let pages = rows.div_ceil(size);
        (pages > 1).then(|| {
            let page = sort.page.get().min(pages - 1);
            view! {
                <div class="flex items-center gap-2 py-1 text-xs text-muted">
                    <button
                        class="btn btn-ghost btn-xs"
                        disabled=page == 0
                        on:click=move |_| sort.page.update(|page| *page = page.saturating_sub(1))
                    >
                        "Previous"
                    </button>
                    <span class="tabular-nums">
                        {format!(
                            "Page {} of {pages}, rows {} to {} of {rows}",
                            page + 1,
                            page * size + 1,
                            ((page + 1) * size).min(rows),
                        )}
                    </span>
                    <button
                        class="btn btn-ghost btn-xs"
                        disabled=page + 1 >= pages
                        on:click=move |_| sort.page.update(|page| *page += 1)
                    >
                        "Next"
                    </button>
                </div>
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_sorts_all_the_rows_then_pages() {
        let owner = Owner::new();
        owner.with(|| {
            TABLE_ROWS.with(|rows| rows.set(2));
            let sort = PagedSort {
                sort: RwSignal::new((0, true)),
                page: RwSignal::new(1),
                key: "test",
            };
            let rows = vec![1, 5, 3, 4, 2];
            let (page, pages) = page_of(&rows, sort, |row, _| SortKey::Number(*row as f64));
            assert_eq!(pages, 3);
            // Descending 5 4 | 3 2 | 1: the second page.
            assert_eq!(page, vec![3, 2]);
        });
    }
}
