// Sorts the rows of any table by clicking on the headers of its <thead>, ascending then
// descending. The tables sorted by the backend, the replay list, have `data-server-sort`.
// Leptos may add or replace the rows of a sorted table, a MutationObserver sorts them again.
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

  document.addEventListener("click", (event) => {
    const header = event.target.closest?.("thead th");
    // Links and buttons in the headers keep working.
    if (!header || event.target.closest("a, button, input, select")) return;
    const table = header.closest("table");
    if (!table || table.hasAttribute("data-server-sort") || header.hasAttribute("data-no-sort")) return;
    const column = String(header.cellIndex);
    const direction =
      table.dataset.sortColumn === column && table.dataset.sortDirection === "asc" ? "desc" : "asc";
    table.dataset.sortColumn = column;
    table.dataset.sortDirection = direction;
    table.querySelectorAll("thead th").forEach((th) => th.removeAttribute("aria-sort"));
    header.setAttribute("aria-sort", direction === "asc" ? "ascending" : "descending");
    sortTable(table);
    observer.observe(table, { childList: true, subtree: true, characterData: true });
  });
})();
