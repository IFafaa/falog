import { COPY, type Lang } from "../i18n";

/** "Oct 10, 2026" / "10 de out. de 2026", in UTC so the build machine's time zone does not matter. */
export function formatDate(iso: string, lang: Lang): string {
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) return "";
  return date.toLocaleDateString(COPY[lang].dateLocale, {
    year: "numeric",
    month: "short",
    day: "numeric",
    timeZone: "UTC",
  });
}
