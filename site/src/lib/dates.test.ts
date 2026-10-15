import { describe, expect, it } from "vitest";
import { formatDate } from "./dates";

describe("formatDate", () => {
  it("writes the date in each language", () => {
    expect(formatDate("2026-10-10T12:00:00Z", "en")).toBe("Oct 10, 2026");
    expect(formatDate("2026-10-10T12:00:00Z", "pt")).toBe("10 de out. de 2026");
  });

  it("uses the UTC date whatever the build machine's time zone", () => {
    expect(formatDate("2026-10-10T23:30:00Z", "en")).toBe("Oct 10, 2026");
    expect(formatDate("2026-10-10T00:30:00Z", "en")).toBe("Oct 10, 2026");
  });

  it("gives nothing for a missing or broken date", () => {
    expect(formatDate("", "en")).toBe("");
    expect(formatDate("not a date", "pt")).toBe("");
  });
});
