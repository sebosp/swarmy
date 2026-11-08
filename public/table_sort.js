// Sorts the rows of any table by clicking on the headers of its <thead>, ascending then
// descending. The tables sorted by the backend, the replay list, have `data-server-sort`.
// Leptos may add or replace the rows of a sorted table, a MutationObserver sorts them again.
// The sort chosen is remembered in the storage of the webview, by the headers of the table (or
// its `data-sort-key`), and applied again when a table with these headers is shown.
// A table with more rows than the setting of Config (`data-table-rows` of <html>, 100 by
// default) shows them in pages, with Previous and Next in a footer; the headers sort all the rows.
// The tables that can have thousands of rows are paged by the view instead (`data-server-sort`).
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

  const pageSize = () => Math.max(1, Number(document.documentElement.dataset.tableRows) || 100);

  // Shows the rows of the current page of the table, and the footer to change the page, only when
  // the rows do not fit in one page. Changes the page only where it differs, it runs again on
  // every change of the rows.
  function paginate(table) {
    if (table.hasAttribute("data-server-sort")) return;
    const body = table.tBodies[0];
    if (!body) return;
    const rows = Array.from(body.rows);
    const size = pageSize();
    const pages = Math.ceil(rows.length / size);
    let footer = table.querySelector(":scope > tfoot.swarmy-pager");
    if (pages <= 1) {
      rows.forEach((row) => row.hidden && (row.hidden = false));
      footer?.remove();
      return;
    }
    const page = Math.min(Number(table.dataset.page) || 0, pages - 1);
    table.dataset.page = String(page);
    rows.forEach((row, idx) => {
      const hidden = idx < page * size || idx >= (page + 1) * size;
      if (row.hidden !== hidden) row.hidden = hidden;
    });
    if (!footer) {
      footer = document.createElement("tfoot");
      footer.className = "swarmy-pager";
      footer.innerHTML =
        '<tr><td><div class="flex items-center gap-2 py-1 text-xs text-muted">' +
        '<button type="button" class="btn btn-ghost btn-xs" data-page-step="-1">Previous</button>' +
        '<span class="tabular-nums"></span>' +
        '<button type="button" class="btn btn-ghost btn-xs" data-page-step="1">Next</button>' +
        "</div></td></tr>";
      table.appendChild(footer);
    }
    const cell = footer.rows[0].cells[0];
    const columns = String(table.querySelector("thead tr")?.cells.length || 1);
    if (cell.getAttribute("colspan") !== columns) cell.setAttribute("colspan", columns);
    const text = `Page ${page + 1} of ${pages}, rows ${page * size + 1} to ${Math.min((page + 1) * size, rows.length)} of ${rows.length}`;
    const label = cell.querySelector("span");
    if (label.textContent !== text) label.textContent = text;
    const [previous, next] = cell.querySelectorAll("button");
    if (previous.disabled !== (page === 0)) previous.disabled = page === 0;
    if (next.disabled !== (page + 1 >= pages)) next.disabled = page + 1 >= pages;
  }

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
    paginate(table);
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

  // The column the rows are already sorted by, as the view or the backend ordered them, and its
  // direction: -1 when none, the first such column otherwise.
  function currentOrder(table) {
    const headers = Array.from(table.querySelector("thead tr")?.cells ?? []);
    const rows = Array.from(table.tBodies[0]?.rows ?? []);
    // The number columns first: a column of names in alphabetical order by chance is a weaker
    // sign of the order than a column of counts.
    for (const numbers of [true, false]) {
      for (const header of headers) {
        if (header.hasAttribute("data-no-sort") || header.textContent.trim() === "") continue;
        const values = rows.map((row) => cellValue(row, header.cellIndex)).filter((value) => !value.empty);
        if (values.length < 2 || values.every((value) => value.number !== undefined) !== numbers) continue;
        const orders = values.slice(1).map((value, idx) => compare(values[idx], value));
        if (orders.every((order) => order === 0)) continue;
        if (orders.every((order) => order >= 0)) return { column: header.cellIndex, direction: "desc" };
        if (orders.every((order) => order <= 0)) return { column: header.cellIndex, direction: "asc" };
      }
    }
    return null;
  }

  // The default sort of a table without a sort remembered: the one the view set, else the order
  // the rows already have, else the games (most first), else the first column.
  function defaultSort(table) {
    if (table.dataset.sortColumn !== undefined) {
      return { column: table.dataset.sortColumn, direction: table.dataset.sortDirection || "asc", keep: true };
    }
    const preset = table.querySelector("thead th[aria-sort]");
    if (preset) {
      const direction = preset.getAttribute("aria-sort") === "descending" ? "desc" : "asc";
      return { column: preset.cellIndex, direction, keep: true };
    }
    const current = currentOrder(table);
    if (current) return { ...current, keep: true };
    const headers = Array.from(table.querySelector("thead tr")?.cells ?? []);
    const games = headers.find((th) => th.textContent.replace(/[↕▲▼]/g, "").trim() === "Games");
    if (games) return { column: games.cellIndex, direction: "desc" };
    const first = headers.find((th) => !th.hasAttribute("data-no-sort") && th.textContent.trim() !== "");
    return first ? { column: first.cellIndex, direction: "asc" } : null;
  }

  // Marks the sorted column without moving the rows, they are already in its order.
  function markSort(table, column, direction) {
    const header = table.querySelector("thead tr")?.cells[Number(column)];
    if (!header) return;
    table.dataset.sortColumn = String(column);
    table.dataset.sortDirection = direction;
    table.querySelectorAll("thead th").forEach((th) => th.removeAttribute("aria-sort"));
    header.setAttribute("aria-sort", direction === "asc" ? "ascending" : "descending");
    observer.observe(table, { childList: true, subtree: true, characterData: true });
    paginate(table);
  }

  // The tables shown, once they have rows: the sort remembered, or the default one, is shown on
  // its column; every table says how it is sorted.
  const seen = new WeakSet();
  function restoreSorts() {
    document.querySelectorAll("table:not([data-server-sort])").forEach((table) => {
      paginate(table);
      if (seen.has(table) || !table.tHead) return;
      if ((table.tBodies[0]?.rows.length ?? 0) < 2) return;
      seen.add(table);
      const saved = remembered(table);
      if (saved && saved.column !== undefined) {
        applySort(table, saved.column, saved.direction === "desc" ? "desc" : "asc");
        return;
      }
      const sort = defaultSort(table);
      if (!sort) return;
      if (sort.keep) {
        markSort(table, sort.column, sort.direction);
      } else {
        applySort(table, sort.column, sort.direction);
      }
    });
  }
  let restoreTimer;
  new MutationObserver(() => {
    clearTimeout(restoreTimer);
    restoreTimer = setTimeout(restoreSorts, 50);
  }).observe(document.documentElement, { childList: true, subtree: true });
  // The tables already in the page when the script runs.
  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", restoreSorts);
  } else {
    restoreSorts();
  }

  document.addEventListener("click", (event) => {
    const header = event.target.closest?.("thead th");
    // Links and buttons in the headers keep working.
    if (!header || event.target.closest("a, button, input, select")) return;
    const table = header.closest("table");
    if (!table || table.hasAttribute("data-server-sort") || header.hasAttribute("data-no-sort")) return;
    const column = String(header.cellIndex);
    // A new order starts on the first page.
    table.dataset.page = "0";
    const direction =
      table.dataset.sortColumn === column && table.dataset.sortDirection === "asc" ? "desc" : "asc";
    applySort(table, column, direction);
    remember(table);
  });

  // The pages of the long tables.
  document.addEventListener("click", (event) => {
    const button = event.target.closest?.("tfoot.swarmy-pager button[data-page-step]");
    if (!button) return;
    const table = button.closest("table");
    table.dataset.page = String(Math.max(0, (Number(table.dataset.page) || 0) + Number(button.dataset.pageStep)));
    paginate(table);
  });
  // The rows per page changed in Config.
  new MutationObserver(() => document.querySelectorAll("table").forEach(paginate)).observe(
    document.documentElement,
    { attributes: true, attributeFilter: ["data-table-rows"] },
  );
})();
