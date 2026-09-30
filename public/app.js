const $ = (id) => document.getElementById(id);
const esc = (value) =>
  String(value).replace(
    /[&<>"']/g,
    (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[c]),
  );

const KEY = "sareva.mandate";
const MANDATE_FIELDS = [
  "revenue_min",
  "revenue_max",
  "ebitda_min",
  "min_ebitda_margin",
  "min_growth",
  "max_net_debt_to_ebitda",
];
const DEFAULTS = { min_ebitda_margin: "10", min_growth: "5", max_net_debt_to_ebitda: "4" };

const mandateForm = $("mandate");
const dealForm = $("deal");
const result = $("result");
const error = $("error");

const money = (n) =>
  Math.abs(n) >= 1e9
    ? `$${(n / 1e9).toFixed(2)}B`
    : Math.abs(n) >= 1e6
      ? `$${(n / 1e6).toFixed(1)}M`
      : Math.abs(n) >= 1e3
        ? `$${(n / 1e3).toFixed(1)}K`
        : `$${n.toFixed(0)}`;

const numOrNull = (value) => {
  if (value === "" || value == null) return null;
  const n = Number(value);
  return Number.isNaN(n) ? null : n;
};

function loadMandate() {
  let stored = {};
  try {
    stored = JSON.parse(localStorage.getItem(KEY) || "{}");
  } catch {
    stored = {};
  }
  MANDATE_FIELDS.forEach((name) => {
    const value = name in stored ? stored[name] : DEFAULTS[name];
    if (value != null) mandateForm[name].value = value;
  });
}

function saveMandate() {
  const raw = {};
  MANDATE_FIELDS.forEach((name) => (raw[name] = mandateForm[name].value));
  try {
    localStorage.setItem(KEY, JSON.stringify(raw));
  } catch {
    /* storage unavailable — continue without saving */
  }
}

function mandatePayload() {
  const percent = (value) => {
    const n = numOrNull(value);
    return n === null ? null : n / 100;
  };
  return {
    revenue_min: numOrNull(mandateForm.revenue_min.value),
    revenue_max: numOrNull(mandateForm.revenue_max.value),
    ebitda_min: numOrNull(mandateForm.ebitda_min.value),
    min_ebitda_margin: percent(mandateForm.min_ebitda_margin.value),
    min_growth: percent(mandateForm.min_growth.value),
    max_net_debt_to_ebitda: numOrNull(mandateForm.max_net_debt_to_ebitda.value),
  };
}

function showStep(step) {
  mandateForm.hidden = step !== 1;
  dealForm.hidden = step !== 2;
  $("tab-1").classList.toggle("active", step === 1);
  $("tab-2").classList.toggle("active", step === 2);
  if (step === 1) result.hidden = true;
}

function render(data, company) {
  const format = (criterion) =>
    criterion.format === "percent"
      ? `${(criterion.deal_value * 100).toFixed(1)}%`
      : criterion.format === "multiple"
        ? `${criterion.deal_value.toFixed(2)}x`
        : money(criterion.deal_value);

  const rows = data.criteria
    .map(
      (c) => `
      <li class="criterion ${c.passed ? "pass" : "fail"}">
        <span class="criterion-icon">${c.passed ? "✓" : "✗"}</span>
        <span class="criterion-label">${esc(c.label)}</span>
        <span class="criterion-value">${format(c)}</span>
        <span class="criterion-req">${esc(c.requirement)}</span>
      </li>`,
    )
    .join("");

  result.innerHTML = `
    <h2 class="verdict ${data.qualified ? "qualified" : "rejected"}">${data.qualified ? "Qualified" : "Not qualified"}</h2>
    <p class="hint">${data.passed} of ${data.total} mandate criteria met${company ? ` · ${esc(company)}` : ""}</p>
    <ul class="criteria">${rows || '<li class="criterion">No criteria in the mandate.</li>'}</ul>`;
  result.hidden = false;
}

async function checkHealth() {
  const dot = $("status");
  try {
    const res = await fetch("/api/health");
    dot.classList.toggle("ok", res.ok);
    dot.title = res.ok ? "API healthy" : `API: ${res.status}`;
  } catch {
    dot.classList.remove("ok");
    dot.title = "API unreachable";
  }
}

mandateForm.addEventListener("submit", (event) => {
  event.preventDefault();
  saveMandate();
  showStep(2);
});

$("edit-mandate").addEventListener("click", () => showStep(1));

$("reset-mandate").addEventListener("click", () => {
  MANDATE_FIELDS.forEach((name) => (mandateForm[name].value = DEFAULTS[name] ?? ""));
  try {
    localStorage.removeItem(KEY);
  } catch {
    /* ignore */
  }
});

dealForm.addEventListener("submit", async (event) => {
  event.preventDefault();
  error.hidden = true;

  const payload = {
    mandate: mandatePayload(),
    deal: {
      revenue: Number(dealForm.revenue.value),
      ebitda: Number(dealForm.ebitda.value),
      growth: Number(dealForm.growth.value) / 100,
      debt: Number(dealForm.debt.value),
    },
  };

  let res;
  try {
    res = await fetch("/api/screen", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(payload),
    });
  } catch {
    error.textContent = "Network error — is the Worker running?";
    error.hidden = false;
    return;
  }

  const data = await res.json().catch(() => ({}));
  if (!res.ok) {
    error.textContent = data.error || `Request failed (${res.status})`;
    error.hidden = false;
    return;
  }

  render(data, dealForm.name.value);
});

loadMandate();
showStep(1);
checkHealth();
