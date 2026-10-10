// Falog's public site: static pages in English (/) and Brazilian Portuguese (/pt/), published to
// GitHub Pages at https://ifafaa.github.io/falog/.
import { defineConfig } from "astro/config";

export default defineConfig({
  site: "https://ifafaa.github.io",
  base: "/falog",
  trailingSlash: "always",
  output: "static",
  i18n: {
    locales: ["en", "pt"],
    defaultLocale: "en",
    routing: { prefixDefaultLocale: false, redirectToDefaultLocale: false },
  },
  build: {
    // Everything in files, nothing inline: the pages' Content Security Policy allows only 'self'.
    inlineStylesheets: "never",
  },
  vite: {
    build: { assetsInlineLimit: 0 },
    // The fonts and icon come straight from the app's assets, one folder up.
    server: { fs: { allow: [".."] } },
  },
  devToolbar: { enabled: false },
});
