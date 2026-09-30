// Optional Turnstile gate for AI tools. Activates only when the page provides a
// site key: <meta name="turnstile-sitekey" content="...">. Without it (and when
// TURNSTILE_SECRET is unset server-side) verification is skipped entirely.
(function () {
  const meta = document.querySelector('meta[name="turnstile-sitekey"]');
  const sitekey = meta && meta.content;
  const form = document.querySelector("form[data-endpoint]");
  if (!sitekey || !form) return;

  const hidden = document.createElement("input");
  hidden.type = "hidden";
  hidden.name = "turnstile_token";
  hidden.value = "";
  form.appendChild(hidden);

  const widget = document.createElement("div");
  widget.className = "turnstile";
  form.insertBefore(widget, form.querySelector('button[type="submit"]'));

  const script = document.createElement("script");
  script.src = "https://challenges.cloudflare.com/turnstile/v0/api.js";
  script.async = true;
  script.defer = true;
  script.onload = () => {
    if (window.turnstile) {
      window.turnstile.render(widget, {
        sitekey,
        callback: (token) => {
          hidden.value = token;
        },
      });
    }
  };
  document.head.appendChild(script);
})();
