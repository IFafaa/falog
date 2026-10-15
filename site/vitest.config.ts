// Unit tests for the code the site runs while it is built (src/lib, src/i18n): `npm test`.
// Astro's own Vite settings apply, so `?raw` imports work as in `npm run build`. Vitest serves from
// `/`, so the site's base path is given to the tests the way the build sees it.
import { getViteConfig } from "astro/config";
import astroConfig from "./astro.config.mjs";

export default getViteConfig({
  test: {
    include: ["src/**/*.test.ts"],
    env: { BASE_URL: astroConfig.base },
  },
});
