const $ = (id) => document.getElementById(id);
const form = $("screener");

const money = (n) => {
  const abs = Math.abs(n);
  if (abs >= 1e9) return `$${(n / 1e9).toFixed(2)}B`;
  if (abs >= 1e6) return `$${(n / 1e6).toFixed(1)}M`;
  if (abs >= 1e3) return `$${(n / 1e3).toFixed(1)}K`;
  return `$${n.toFixed(0)}`;
};

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

form.addEventListener("submit", async (event) => {
  event.preventDefault();
  const error = $("error");
  error.hidden = true;

  const revenue = Number($("revenue").value);
  const payload = {
    revenue,
    ebitda: Number($("ebitda").value),
    growth: Number($("growth").value) / 100,
    debt: Number($("debt").value),
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

  const data = await res.json();
  if (!res.ok) {
    error.textContent = data.error || `Request failed (${res.status})`;
    error.hidden = false;
    return;
  }

  $("r-revenue").textContent = money(revenue);
  $("r-margin").textContent = `${(data.ebitda_margin * 100).toFixed(1)}%`;
  $("r-leverage").textContent = `${data.debt_to_ebitda.toFixed(2)}x`;
  $("r-score").textContent = `${data.score} / 100`;

  const verdict = $("r-verdict");
  verdict.textContent = data.qualified ? "Qualified" : "Not qualified";
  verdict.className = data.qualified ? "qualified" : "rejected";

  $("result").hidden = false;
});

checkHealth();
