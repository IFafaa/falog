// Runs before the first paint: applies the theme the visitor picked last time, else the system's.
(function () {
  var saved = null;
  try {
    saved = localStorage.getItem("falog-theme");
  } catch (e) {
    // Storage blocked (private window, strict settings): follow the system.
  }
  if (saved === "light" || saved === "dark") {
    document.documentElement.dataset.theme = saved;
  } else if (window.matchMedia && matchMedia("(prefers-color-scheme: light)").matches) {
    document.documentElement.dataset.theme = "light";
  } else {
    document.documentElement.dataset.theme = "dark";
  }
})();
