import { describe, expect, it } from "vitest";

import { safeExternalUrl } from "./urlSecurity";

describe("safeExternalUrl", () => {
  it.each(["javascript:alert(1)", "data:text/html,unsafe", "file:///etc/passwd"])(
    "rejects the unsafe %s protocol",
    (value) => {
      expect(safeExternalUrl(value)).toBeNull();
    },
  );

  it.each(["https://example.com/package", "http://example.com/package"])(
    "allows the web URL %s",
    (value) => {
      expect(safeExternalUrl(value)).toBe(value);
    },
  );
});
