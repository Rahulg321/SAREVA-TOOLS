(function () {
  const ACR = {
    ev: "EV",
    ebitda: "EBITDA",
    irr: "IRR",
    moic: "MOIC",
    fcf: "FCF",
    dso: "DSO",
    net: "Net",
  };

  const prettyKey = (k) =>
    k
      .split("_")
      .map((w) => ACR[w.toLowerCase()] || w.charAt(0).toUpperCase() + w.slice(1))
      .join(" ");

  const el = (tag, cls, text) => {
    const node = document.createElement(tag);
    if (cls) node.className = cls;
    if (text != null) node.textContent = text;
    return node;
  };

  const isObject = (v) => v && typeof v === "object" && !Array.isArray(v);
  const isScalar = (v) => v === null || ["number", "string", "boolean"].includes(typeof v);
  const isStringArray = (v) =>
    Array.isArray(v) && v.length > 0 && v.every((x) => typeof x === "string");
  const isTable = (v) => Array.isArray(v) && v.length > 0 && v.every(isObject);
  const isMatrix = (v) =>
    isObject(v) && Array.isArray(v.rows) && Array.isArray(v.cols) && Array.isArray(v.values);

  function paragraph(text) {
    return el("p", "prose", text);
  }

  function bulletList(items) {
    const ul = el("ul", "bullets");
    items.forEach((item) => ul.appendChild(el("li", null, item)));
    return ul;
  }

  function scalarRow(dl, key, value) {
    dl.appendChild(el("dt", null, prettyKey(key)));
    dl.appendChild(el("dd", null, FT.formatByKey(key, value)));
  }

  function renderTable(rows) {
    const keys = Object.keys(rows[0]);
    const table = el("table", "data-table");
    const thead = el("thead");
    const hr = el("tr");
    keys.forEach((k) => hr.appendChild(el("th", null, prettyKey(k))));
    thead.appendChild(hr);
    table.appendChild(thead);
    const tbody = el("tbody");
    rows.forEach((row) => {
      const tr = el("tr");
      keys.forEach((k) => {
        const v = row[k];
        tr.appendChild(el("td", typeof v === "number" ? "num" : null, FT.formatByKey(k, v)));
      });
      tbody.appendChild(tr);
    });
    table.appendChild(tbody);
    return table;
  }

  function renderMatrix(m) {
    const table = el("table", "data-table");
    const thead = el("thead");
    const hr = el("tr");
    hr.appendChild(el("th", null, m.row_label || ""));
    m.cols.forEach((c) => hr.appendChild(el("th", "num", FT.formatByKey("", c))));
    thead.appendChild(hr);
    table.appendChild(thead);
    const tbody = el("tbody");
    const fmt = (v) =>
      m.value_format === "percent"
        ? FT.pct(v)
        : m.value_format === "multiple"
          ? FT.mult(v)
          : FT.formatByKey("", v);
    m.rows.forEach((r, i) => {
      const tr = el("tr");
      tr.appendChild(el("th", "rowhead", String(r)));
      (m.values[i] || []).forEach((v) => tr.appendChild(el("td", "num", fmt(v))));
      tbody.appendChild(tr);
    });
    table.appendChild(tbody);
    return table;
  }

  function renderFlags(flags) {
    const wrap = el("div", "flags");
    flags.forEach((f) => {
      const warn = f.level === "warn";
      const row = el("div", "flag " + (warn ? "warn" : "good"));
      row.appendChild(el("span", "flag-icon", warn ? "!" : "✓"));
      const body = el("div");
      body.appendChild(el("div", "flag-label", f.label));
      if (f.detail) body.appendChild(el("div", "flag-detail", f.detail));
      row.appendChild(body);
      wrap.appendChild(row);
    });
    return wrap;
  }

  function section(title, node) {
    const sec = el("div", "result-section");
    sec.appendChild(el("h2", "eyebrow", prettyKey(title)));
    sec.appendChild(node);
    return sec;
  }

  function renderResult(container, data) {
    container.innerHTML = "";
    const dl = el("dl", "result-scalars");
    const blocks = [];
    for (const [k, v] of Object.entries(data)) {
      if (typeof v === "number" || typeof v === "boolean" || v === null) scalarRow(dl, k, v);
      else if (typeof v === "string") blocks.push(section(k, paragraph(v)));
      else if (k === "flags" && Array.isArray(v)) blocks.push(section("Flags", renderFlags(v)));
      else if (isStringArray(v)) blocks.push(section(k, bulletList(v)));
      else if (isTable(v)) blocks.push(section(k, renderTable(v)));
      else if (isMatrix(v)) blocks.push(section(k, renderMatrix(v)));
      else if (isObject(v)) {
        const nested = el("dl");
        for (const [nk, nv] of Object.entries(v)) if (isScalar(nv)) scalarRow(nested, nk, nv);
        blocks.push(section(k, nested));
      }
    }
    if (dl.childElementCount) container.appendChild(dl);
    blocks.forEach((b) => container.appendChild(b));
  }

  FT.renderResult = renderResult;

  function buildPayload(form) {
    const payload = {};
    form.querySelectorAll("input, textarea, select").forEach((field) => {
      if (!field.name) return;
      const value =
        field.tagName === "TEXTAREA" && field.dataset.csv !== undefined
          ? FT.parseCsv(field.value)
          : field.type === "number"
            ? field.value === ""
              ? 0
              : Number(field.value)
            : field.value;
      const parts = field.name.split(".");
      let cur = payload;
      for (let i = 0; i < parts.length - 1; i++) {
        cur[parts[i]] = cur[parts[i]] || {};
        cur = cur[parts[i]];
      }
      cur[parts[parts.length - 1]] = value;
    });
    return payload;
  }

  document.addEventListener("DOMContentLoaded", () => {
    const form = document.querySelector("form[data-endpoint]");
    if (!form) return;
    const result = document.getElementById("result");
    const error = document.getElementById("error");
    const button = form.querySelector('button[type="submit"]');

    form.addEventListener("submit", async (event) => {
      event.preventDefault();
      error.hidden = true;
      const label = button.textContent;
      button.disabled = true;
      button.textContent = "Calculating…";
      try {
        const data = await FT.api(form.dataset.endpoint, buildPayload(form));
        result.hidden = false;
        if (window.renderResult) window.renderResult(result, data);
        else renderResult(result, data);
      } catch (err) {
        result.hidden = true;
        error.textContent = err.message;
        error.hidden = false;
      } finally {
        button.disabled = false;
        button.textContent = label;
      }
    });
  });
})();
