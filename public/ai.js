// Turnstile gate for the AI tools. Renders the widget when a site key is
// configured (window.SAREVA_TURNSTILE_SITE_KEY or a meta tag), injects the
// token into the form as `turnstile_token`, and resets the widget after each
// request (tokens are single-use). Inert when no site key is present.
(function () {
  const meta = document.querySelector('meta[name="turnstile-sitekey"]');
  const sitekey = window.SAREVA_TURNSTILE_SITE_KEY || (meta && meta.content);
  const form = document.querySelector("form[data-endpoint]");
  if (!sitekey || !form) return;

  const button = form.querySelector('button[type="submit"]');

  const hidden = document.createElement("input");
  hidden.type = "hidden";
  hidden.name = "turnstile_token";
  hidden.value = "";
  form.appendChild(hidden);

  const widget = document.createElement("div");
  widget.className = "turnstile";
  form.insertBefore(widget, button);

  let widgetId = null;

  const lock = (locked) => {
    if (!button) return;
    button.disabled = locked;
  };
  lock(true);

  form.addEventListener("tool:done", () => {
    hidden.value = "";
    lock(true);
    if (window.turnstile && widgetId !== null) window.turnstile.reset(widgetId);
  });

  const script = document.createElement("script");
  script.src = "https://challenges.cloudflare.com/turnstile/v0/api.js";
  script.async = true;
  script.defer = true;
  script.onload = () => {
    if (!window.turnstile) return;
    widgetId = window.turnstile.render(widget, {
      sitekey,
      action: "ai",
      callback: (token) => {
        hidden.value = token;
        lock(false);
      },
      "error-callback": () => {
        hidden.value = "";
        lock(true);
      },
      "expired-callback": () => {
        hidden.value = "";
        lock(true);
      },
    });
  };
  document.head.appendChild(script);
})();
