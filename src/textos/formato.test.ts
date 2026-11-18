import { describe, expect, it } from "vitest";
import { fechaLegible, tamanoLegible } from "./formato";

describe("fechaLegible", () => {
  it("pasa la marca UTC del núcleo a fecha y hora del idioma", () => {
    const fecha = fechaLegible("2026-11-18T09:35:49Z", "es");
    expect(fecha).toMatch(/2026/);
    expect(fecha).toMatch(/nov/);
    expect(fechaLegible("2026-11-18T09:35:49Z", "en")).toMatch(/Nov/);
  });

  it("una marca que no se entiende se enseña tal cual", () => {
    expect(fechaLegible("ayer", "es")).toBe("ayer");
  });
});

describe("tamanoLegible", () => {
  it("usa kB o MB según el tamaño y nunca dice 0", () => {
    expect(tamanoLegible(10, "es")).toMatch(/^1\skB$/);
    expect(tamanoLegible(120_000, "en")).toMatch(/^120\skB$/);
    expect(tamanoLegible(2_450_000, "es")).toMatch(/^2,5\sMB$/);
  });
});
