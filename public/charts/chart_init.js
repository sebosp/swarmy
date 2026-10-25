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
(() => {
  if (window.__chartInitDone) return;
  window.__chartInitDone = true;
  const chartInstances = new Map();
  const chartMetadata = new Map();

  // Swarmy is dark only, the colors match the tailwind gray palette.
  const THEME = {
    mode: "dark",
    foreground: "#d1d5db",
    muted: "#9ca3af",
    grid: "#374151",
    palette: ["#6366f1", "#22c55e", "#0ea5e9", "#eab308", "#a855f7"],
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
    const labels = parse(container.dataset.chartLabels, []);
    const colors = parse(container.dataset.chartColors, null) || THEME.palette;
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
      theme: { mode: THEME.mode },
      colors,
      dataLabels: { enabled: false },
      legend: {
        show: showLegend,
        position: "top",
        horizontalAlign: "left",
        fontSize: "11px",
        fontFamily: "inherit",
        labels: { colors: THEME.foreground },
        markers: { size: 4 },
        itemMargin: { horizontal: 8 },
      },
      xaxis: {
        categories: labels,
        labels: { style: { colors: THEME.muted, fontSize: "10px" }, hideOverlappingLabels: true, rotate: 0 },
        axisBorder: { color: THEME.grid },
        axisTicks: { show: false },
        tickAmount: labels.length > 24 ? 12 : undefined,
      },
      yaxis: {
        max: yMax,
        min: 0,
        forceNiceScale: true,
        labels: { style: { colors: THEME.muted, fontSize: "10px" }, formatter: (value) => `${Math.round(value)}${suffix}` },
      },
      grid: { borderColor: THEME.grid, strokeDashArray: 2, padding: { top: -10, bottom: 0 } },
      tooltip: {
        theme: THEME.mode,
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
      options.stroke = { curve: "smooth", width: 2 };
      options.markers = { size: 0, hover: { size: 4 } };
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

  let resizeTimeout;
  window.addEventListener("resize", () => {
    clearTimeout(resizeTimeout);
    resizeTimeout = setTimeout(() => {
      chartMetadata.clear();
      checkAndInitCharts();
    }, 250);
  });
})();
