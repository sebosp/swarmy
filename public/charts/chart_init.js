// ApexCharts initializer, adapted from the rust-ui charts (https://leptos.rust-ui.com, MIT
// License, Copyright (c) 2026 Max Wells).
// Leptos replaces the views without reloading the page, so a MutationObserver renders the chart
// containers, the `div[data-chart-values]`, when they are added or their data changes.
//
// The container attributes:
// - data-name: BarChart or LineChart.
// - data-chart-values: a JSON array of numbers, or of arrays of numbers for several series.
// - data-chart-labels: a JSON array of the x axis labels.
// - data-chart-series-names: a JSON array with the name of each series.
// - data-chart-colors: a JSON array with the CSS color of each series.
// - data-chart-tooltips: a JSON array with the tooltip of each label, instead of the values.
// - data-chart-stack-type: "normal" or "100%" to stack the bars of the series.
// - data-chart-y-max: the maximum of the y axis, so that several charts share the scale.
// - data-chart-value-suffix: appended to the values, i.e. "%".
// - data-chart-curve: the curve of the lines, "smooth" by default, "straight" or "stepline".
(() => {
  if (window.__chartInitDone) return;
  window.__chartInitDone = true;
  const chartInstances = new Map();
  const chartMetadata = new Map();

  // The colors of the theme picked in Config, the --chart-* variables of styles.css: read when a
  // chart renders, the charts render again when the theme changes.
  const cssVar = (name, fallback) => {
    const value = getComputedStyle(document.documentElement).getPropertyValue(name).trim();
    return value || fallback;
  };
  const theme = () => ({
    mode: cssVar("color-scheme", "dark") === "light" ? "light" : "dark",
    foreground: cssVar("--chart-fg", "#d1d5db"),
    muted: cssVar("--chart-muted", "#9ca3af"),
    grid: cssVar("--chart-grid", "#374151"),
    palette: [1, 2, 3, 4, 5, 6, 7, 8].map((idx) => cssVar(`--chart-${idx}`, "#6366f1")),
  });
  // A `var(--chart-N)` given as a series color, ApexCharts needs the computed color.
  const resolveColor = (color) => {
    const match = /^var\((--[\w-]+)\)$/.exec(String(color).trim());
    return match ? cssVar(match[1], "#6366f1") : color;
  };

  const parse = (value, fallback) => {
    try {
      return value ? JSON.parse(value) : fallback;
    } catch (_err) {
      return fallback;
    }
  };

  function chartSeries(container) {
    const data = parse(container.dataset.chartValues, []);
    const names = parse(container.dataset.chartSeriesNames, null);
    const multi = Array.isArray(data[0]);
    const datasets = multi ? data : [data];
    return {
      series: datasets.map((values, idx) => ({ name: names?.[idx] || `Series ${idx + 1}`, data: values })),
      showLegend: multi && names !== null,
    };
  }

  function commonOptions(container, type) {
    const { series, showLegend } = chartSeries(container);
    const t = theme();
    const labels = parse(container.dataset.chartLabels, []);
    const colors = (parse(container.dataset.chartColors, null) || t.palette).map(resolveColor);
    const tooltips = parse(container.dataset.chartTooltips, null);
    const suffix = container.dataset.chartValueSuffix || "";
    const yMax = container.dataset.chartYMax ? Number(container.dataset.chartYMax) : undefined;
    const stackType = container.dataset.chartStackType || "";
    return {
      series,
      chart: {
        type,
        height: container.offsetHeight || 200,
        toolbar: { show: false },
        background: "transparent",
        stacked: stackType !== "",
        stackType: stackType === "100%" ? "100%" : undefined,
        animations: { enabled: true, speed: 400 },
        parentHeightOffset: 0,
      },
      theme: { mode: t.mode },
      colors,
      dataLabels: { enabled: false },
      legend: {
        show: showLegend,
        position: "top",
        horizontalAlign: "left",
        fontSize: "11px",
        fontFamily: "inherit",
        labels: { colors: t.foreground },
        markers: { size: 4 },
        itemMargin: { horizontal: 8 },
      },
      xaxis: {
        categories: labels,
        labels: { style: { colors: t.muted, fontSize: "10px" }, hideOverlappingLabels: true, rotate: 0 },
        axisBorder: { color: t.grid },
        axisTicks: { show: false },
        tickAmount: labels.length > 24 ? 12 : undefined,
      },
      yaxis: {
        max: yMax,
        min: 0,
        forceNiceScale: true,
        labels: { style: { colors: t.muted, fontSize: "10px" }, formatter: (value) => `${Math.round(value)}${suffix}` },
      },
      grid: { borderColor: t.grid, strokeDashArray: 2, padding: { top: -10, bottom: 0 } },
      tooltip: {
        theme: t.mode,
        shared: true,
        intersect: false,
        custom: tooltips
          ? ({ dataPointIndex }) => `<div class="px-2 py-1 text-xs">${tooltips[dataPointIndex] ?? ""}</div>`
          : undefined,
        y: { formatter: (value) => `${value?.toLocaleString?.() ?? value}${suffix}` },
      },
    };
  }

  const builders = {
    bar: (container) => {
      const options = commonOptions(container, "bar");
      options.plotOptions = { bar: { borderRadius: 2, borderRadiusApplication: "end", columnWidth: "70%" } };
      options.stroke = { show: true, width: 1, colors: ["transparent"] };
      return options;
    },
    line: (container) => {
      const options = commonOptions(container, "line");
      const curve = container.dataset.chartCurve || "smooth";
      options.stroke = { curve, width: 2 };
      // The steps mark each value, i.e. each patch.
      options.markers = { size: curve === "stepline" ? 3 : 0, hover: { size: 5 } };
      options.yaxis.min = undefined;
      return options;
    },
  };

  function initChart(container) {
    if (!container.id) {
      container.id = "apex-auto-" + Math.random().toString(36).slice(2, 9);
    }
    const id = container.id;
    if (chartInstances.has(id)) {
      chartInstances.get(id).destroy();
      chartInstances.delete(id);
    }
    const type = container.dataset.name?.replace(/Chart$/, "").toLowerCase();
    const options = builders[type]?.(container);
    if (!options || typeof ApexCharts === "undefined") return;
    const chart = new ApexCharts(container, options);
    chart.render();
    chartInstances.set(id, chart);
    chartMetadata.set(id, metadataOf(container));
    container.dataset.chartInitialized = "true";
  }

  const metadataOf = (container) =>
    [
      container.dataset.name,
      container.dataset.chartValues,
      container.dataset.chartLabels,
      container.dataset.chartSeriesNames,
      container.dataset.chartTooltips,
      container.dataset.chartYMax,
      container.dataset.chartColors,
      container.dataset.chartCurve,
    ].join("|");

  function checkAndInitCharts() {
    if (document.readyState === "loading") return;
    for (const [id, chart] of chartInstances) {
      if (!document.getElementById(id)) {
        chart.destroy();
        chartInstances.delete(id);
        chartMetadata.delete(id);
      }
    }
    document.querySelectorAll("div[data-chart-values]").forEach((container) => {
      const needsInit =
        !container.dataset.chartInitialized || chartMetadata.get(container.id) !== metadataOf(container);
      if (needsInit) initChart(container);
    });
  }

  // Leptos mutates the DOM many times in a row, the charts are rendered once it settles.
  let debounceTimer;
  const observer = new MutationObserver(() => {
    clearTimeout(debounceTimer);
    debounceTimer = setTimeout(checkAndInitCharts, 50);
  });

  function startObserver() {
    observer.observe(document.body, {
      childList: true,
      subtree: true,
      attributes: true,
      attributeFilter: ["data-chart-values", "data-chart-labels", "data-chart-y-max"],
    });
    checkAndInitCharts();
  }

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", startObserver);
  } else {
    startObserver();
  }

  // The charts are drawn again with the colors of a new theme (the app sets data-theme, also
  // when the preference of the desktop changes with Follow the system) and for a new size.
  const rerender = () => {
    chartMetadata.clear();
    checkAndInitCharts();
  };
  let renderedTheme = document.documentElement.dataset.theme;
  new MutationObserver(() => {
    const current = document.documentElement.dataset.theme;
    if (current === renderedTheme) return;
    renderedTheme = current;
    rerender();
  }).observe(document.documentElement, { attributes: true, attributeFilter: ["data-theme"] });

  let resizeTimeout;
  window.addEventListener("resize", () => {
    clearTimeout(resizeTimeout);
    resizeTimeout = setTimeout(rerender, 250);
  });
})();
