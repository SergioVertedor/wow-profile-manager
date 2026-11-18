import { describe, expect, it } from "vitest";
import { tamanoLegible } from "./formato";

describe("tamanoLegible", () => {
  it("usa kB o MB según el tamaño y nunca dice 0", () => {
    expect(tamanoLegible(10, "es")).toMatch(/^1\skB$/);
    expect(tamanoLegible(120_000, "en")).toMatch(/^120\skB$/);
    expect(tamanoLegible(2_450_000, "es")).toMatch(/^2,5\sMB$/);
  });
});
