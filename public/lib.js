(function () {
  const FT = {};

  FT.$ = (id) => document.getElementById(id);

  FT.money = (n) => {
    if (n === null || n === undefined || Number.isNaN(n)) return "—";
    const abs = Math.abs(n);
    const sign = n < 0 ? "-" : "";
    if (abs >= 1e9) return `${sign}$${(abs / 1e9).toFixed(2)}B`;
    if (abs >= 1e8) return `${sign}$${(abs / 1e6).toFixed(0)}M`;
    if (abs >= 1e6) return `${sign}$${(abs / 1e6).toFixed(1)}M`;
    if (abs >= 1e3) return `${sign}$${(abs / 1e3).toFixed(1)}K`;
    return `${sign}$${abs.toFixed(0)}`;
  };

  FT.pct = (n) =>
    n === null || n === undefined || Number.isNaN(n) ? "—" : `${(n * 100).toFixed(1)}%`;

  FT.mult = (n) =>
    n === null || n === undefined || Number.isNaN(n) ? "—" : `${n.toFixed(2)}x`;

  FT.num = (n) => (n === null || n === undefined ? "—" : Number(n).toLocaleString());

  FT.formatByKey = function (key, val) {
    if (val === null || val === undefined) return "—";
    if (typeof val === "boolean") return val ? "Yes" : "No";
    if (typeof val === "number") {
      const k = key.toLowerCase();
      if (/(irr|rate|margin|growth|conversion|ownership|yield|percent|pct)/.test(k)) {
        return FT.pct(val);
      }
      if (/(moic|multiple|to_ebitda|coverage|ev_revenue|ev_ebitda|leverage|_x$)/.test(k)) {
        return FT.mult(val);
      }
      if (/(^|_)(ev|debt|equity|ebitda|revenue|interest|proceeds|preference|value|fcf|profit|amount|total)($|_)/.test(k)) {
        return FT.money(val);
      }
      return FT.num(val);
    }
    return String(val);
  };

  FT.parseCsv = function (text) {
    const lines = text
      .trim()
      .split(/\r?\n/)
      .filter((l) => l.trim() !== "");
    if (lines.length < 2) return [];
    const headers = lines[0].split(",").map((h) => h.trim());
    return lines.slice(1).map((line) => {
      const cells = line.split(",").map((c) => c.trim());
      const row = {};
      headers.forEach((h, i) => {
        const raw = cells[i] ?? "";
        const num = Number(raw);
        row[h] = raw !== "" && !Number.isNaN(num) ? num : raw;
      });
      return row;
    });
  };

  FT.api = async function (path, body) {
    const res = await fetch(path, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(body),
    });
    const data = await res.json().catch(() => ({}));
    if (!res.ok) throw new Error(data.error || `Request failed (${res.status})`);
    return data;
  };

  window.FT = FT;
})();
