(function () {
  const TOOLS = [
    {
      href: "/screen",
      tag: "Screening",
      name: "Deal Screener",
      blurb: "Score a company against growth, margin and leverage mandates.",
    },
    {
      href: "/returns",
      tag: "Valuation",
      name: "Returns Calculator",
      blurb: "MOIC, IRR and an exit-value × hold-period sensitivity grid.",
    },
    {
      href: "/reverse-returns",
      tag: "Valuation",
      name: "Reverse Returns",
      blurb: "What exit value do you need to hit a target IRR?",
    },
    {
      href: "/debt-capacity",
      tag: "Credit",
      name: "Debt Capacity",
      blurb: "Leverage, interest and coverage with an EBITDA stress test.",
    },
    {
      href: "/comparables",
      tag: "Valuation",
      name: "Comparable Companies",
      blurb: "Trading multiples, medians and an implied valuation range.",
    },
    {
      href: "/waterfall",
      tag: "Equity",
      name: "Cap Table Waterfall",
      blurb: "Liquidation preferences and holder proceeds at exit.",
    },
    {
      href: "/ebitda-normalization",
      tag: "Earnings",
      name: "EBITDA Normalization",
      blurb: "Build an adjustment ledger and normalized EBITDA.",
    },
    {
      href: "/statement-metrics",
      tag: "Analysis",
      name: "Statement Metrics",
      blurb: "Margins, growth and leverage across periods, with flags.",
    },
    {
      href: "/formula-explain",
      tag: "AI",
      name: "Formula Explainer",
      blurb: "Paste a spreadsheet formula, get plain English.",
    },
    {
      href: "/formula-generate",
      tag: "AI",
      name: "Formula Generator",
      blurb: "Describe the calc, get the formula.",
    },
    {
      href: "/call-actions",
      tag: "AI",
      name: "Call Action Items",
      blurb: "Transcript to takeaways, follow-ups and red flags.",
    },
    {
      href: "/cim-questions",
      tag: "AI",
      name: "CIM → Investment Questions",
      blurb: "Grouped due-diligence questions from a CIM.",
    },
    {
      href: "/cim-snapshot",
      tag: "AI",
      name: "CIM → Deal Snapshot",
      blurb: "Drop a CIM, get metrics, risks and an initial screen.",
    },
    {
      href: "/memo",
      tag: "AI",
      name: "Investment Memo Generator",
      blurb: "Description + financials to a one-page IC memo.",
    },
  ];

  document.addEventListener("DOMContentLoaded", () => {
    const grid = document.getElementById("grid");
    if (!grid) return;
    TOOLS.forEach((t) => {
      const a = document.createElement("a");
      a.className = "tool-card";
      a.href = t.href;
      const tag = document.createElement("span");
      tag.className = "tool-tag";
      tag.textContent = t.tag;
      const name = document.createElement("span");
      name.className = "tool-name";
      name.textContent = t.name;
      const blurb = document.createElement("span");
      blurb.className = "tool-blurb";
      blurb.textContent = t.blurb;
      a.append(tag, name, blurb);
      grid.appendChild(a);
    });
  });
})();
