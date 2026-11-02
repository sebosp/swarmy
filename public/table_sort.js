// Sorts the rows of any table by clicking on the headers of its <thead>, ascending then
// descending. The tables sorted by the backend, the replay list, have `data-server-sort`.
// Leptos may add or replace the rows of a sorted table, a MutationObserver sorts them again.
// The sort chosen is remembered in the storage of the webview, by the headers of the table (or
// its `data-sort-key`), and applied again when a table with these headers is shown.
(() => {
  if (window.__tableSortDone) return;
  window.__tableSortDone = true;
  const collator = new Intl.Collator(undefined, { numeric: true, sensitivity: "base" });

  // The value of a cell: game times (12:34) in seconds, numbers (1,234 · 56% · 3x · 1.2k), else
  // text.
  function cellValue(row, column) {
    const cell = row.cells[column];
    const text = (cell?.dataset.sortValue ?? cell?.textContent ?? "").replace(/^[▲▼■●\s]+/, "").trim();
    if (text === "" || text === "-") return { empty: true };
    const time = text.match(/^(\d+):(\d{2})(?::(\d{2}))?$/);
    if (time) {
      const [h, m, s] = time[3] ? [time[1], time[2], time[3]] : [0, time[1], time[2]];
      return { number: Number(h) * 3600 + Number(m) * 60 + Number(s) };
    }
    if (/^\d{4}-\d{2}-\d{2}/.test(text)) return { text };
    // The resources are written as 12.3k.
    const number = text.match(/^([-+]?\d[\d,]*(\.\d+)?)(k\b)?/);
    if (number) return { number: Number(number[1].replace(/,/g, "")) * (number[3] ? 1000 : 1) };
    return { text };
  }

  function compare(a, b) {
    if (a.empty || b.empty) return (a.empty ? 1 : 0) - (b.empty ? 1 : 0);
    if (a.number !== undefined && b.number !== undefined) return a.number - b.number;
    return collator.compare(String(a.text ?? a.number), String(b.text ?? b.number));
  }

  const sorting = new WeakSet();

  function sortTable(table) {
    const column = Number(table.dataset.sortColumn);
    const direction = table.dataset.sortDirection === "desc" ? -1 : 1;
    sorting.add(table);
    for (const body of table.tBodies) {
      const rows = Array.from(body.rows);
      const sorted = rows
        .map((row, idx) => ({ row, idx, value: cellValue(row, column) }))
        .sort((a, b) => {
          const order = compare(a.value, b.value);
          // The empty cells stay last in both directions.
          if (a.value.empty || b.value.empty) return order || a.idx - b.idx;
          return order * direction || a.idx - b.idx;
        });
      if (sorted.some((entry, idx) => entry.row !== rows[idx])) {
        sorted.forEach((entry) => body.appendChild(entry.row));
      }
    }
    // The mutations of the sort are not sorted again.
    queueMicrotask(() => sorting.delete(table));
  }

  const observer = new MutationObserver((mutations) => {
    const tables = new Set();
    for (const mutation of mutations) {
      const table = mutation.target.closest?.("table[data-sort-column]");
      if (table && !sorting.has(table)) tables.add(table);
    }
    tables.forEach(sortTable);
  });

  // The key of a table: its data-sort-key, or the names of its columns.
  const tableKey = (table) =>
    "swarmy.sort." +
    (table.dataset.sortKey ||
      Array.from(table.querySelectorAll("thead th"))
        .map((th) => th.textContent.replace(/[↕▲▼]/g, "").trim())
        .join("|"));

  // The storage can be missing or refuse, i.e. in a private window: the sort is not remembered.
  const remember = (table) => {
    try {
      localStorage.setItem(
        tableKey(table),
        JSON.stringify({ column: table.dataset.sortColumn, direction: table.dataset.sortDirection }),
      );
    } catch (_err) {}
  };
  const remembered = (table) => {
    try {
      return JSON.parse(localStorage.getItem(tableKey(table)) || "null");
    } catch (_err) {
      return null;
    }
  };

  function applySort(table, column, direction) {
    const header = table.querySelector("thead tr")?.cells[Number(column)];
    if (!header || header.hasAttribute("data-no-sort")) return;
    table.dataset.sortColumn = String(column);
    table.dataset.sortDirection = direction;
    table.querySelectorAll("thead th").forEach((th) => th.removeAttribute("aria-sort"));
    header.setAttribute("aria-sort", direction === "asc" ? "ascending" : "descending");
    sortTable(table);
    observer.observe(table, { childList: true, subtree: true, characterData: true });
  }

  // The tables shown: the remembered sort is applied once to each, or the one the view set
  // (data-sort-column) is kept as the rows change.
  const seen = new WeakSet();
  function restoreSorts() {
    document.querySelectorAll("table:not([data-server-sort])").forEach((table) => {
      if (seen.has(table) || !table.tHead) return;
      seen.add(table);
      const saved = remembered(table);
      if (saved && saved.column !== undefined) {
        applySort(table, saved.column, saved.direction === "desc" ? "desc" : "asc");
      } else if (table.dataset.sortColumn !== undefined) {
        observer.observe(table, { childList: true, subtree: true, characterData: true });
      }
    });
  }
  let restoreTimer;
  new MutationObserver(() => {
    clearTimeout(restoreTimer);
    restoreTimer = setTimeout(restoreSorts, 50);
  }).observe(document.documentElement, { childList: true, subtree: true });

  document.addEventListener("click", (event) => {
    const header = event.target.closest?.("thead th");
    // Links and buttons in the headers keep working.
    if (!header || event.target.closest("a, button, input, select")) return;
    const table = header.closest("table");
    if (!table || table.hasAttribute("data-server-sort") || header.hasAttribute("data-no-sort")) return;
    const column = String(header.cellIndex);
    const direction =
      table.dataset.sortColumn === column && table.dataset.sortDirection === "asc" ? "desc" : "asc";
    applySort(table, column, direction);
    remember(table);
  });
})();
