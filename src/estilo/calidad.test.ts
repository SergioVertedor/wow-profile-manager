import { describe, expect, it } from "vitest";
import { calidadDeNombre } from "./calidad";

describe("calidadDeNombre", () => {
  it.each([
    [{ texto: "Goldberg Joestar", origen: "datos" } as const, "raro"],
    [{ texto: "Goldberg Sp", origen: "carpeta" } as const, "pocoComun"],
    [null, "pobre"],
  ])("%j → %s", (nombre, esperada) => {
    expect(calidadDeNombre(nombre)).toBe(esperada);
  });
});
