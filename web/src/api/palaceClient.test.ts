import { describe, expect, it } from "vitest";

import { createPalaceClient } from "./palaceClient";

describe("createPalaceClient", () => {
  it("rejects an insecure production API origin", () => {
    expect(() =>
      createPalaceClient("http://api.example.com", { production: true }),
    ).toThrow(/HTTPS/);
  });

  it.each(["http://127.0.0.1:3001", "http://localhost:3001", "http://[::1]:3001"])(
    "allows local HTTP development at %s",
    (baseUrl) => {
      expect(() => createPalaceClient(baseUrl, { production: false })).not.toThrow();
    },
  );

  it("rejects non-local HTTP development origins", () => {
    expect(() =>
      createPalaceClient("http://api.example.com", { production: false }),
    ).toThrow(/local development/);
  });
});
