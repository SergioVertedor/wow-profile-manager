import { describe, expect, it } from "vitest";
import { agrupar, alternarGrupo, estadoDe, grupoDe } from "./modoSimple";

const PIEZAS = [
  { id: "addon-dominos", categoria: "addon", segura: true },
  { id: "macros", categoria: "macros", segura: true },
  { id: "teclas", categoria: "teclas", segura: true },
  { id: "clic", categoria: "teclas-clic", segura: true },
  { id: "dominos", categoria: "config-addon", segura: true },
  { id: "prat", categoria: "config-addon", segura: false },
  { id: "solo-genericas", categoria: "chat", segura: false },
  { id: "futura", categoria: "algo-nuevo", segura: true },
];

describe("modoSimple", () => {
  it("agrupa por categoría en el orden fijo, sin grupos sin elegibles ni categorías desconocidas", () => {
    expect(agrupar(PIEZAS, (p) => p.segura)).toEqual([
      { grupo: "macros", ids: ["macros"], elegibles: ["macros"] },
      { grupo: "teclas", ids: ["teclas", "clic"], elegibles: ["teclas", "clic"] },
      { grupo: "configAddons", ids: ["dominos", "prat"], elegibles: ["dominos"] },
      { grupo: "carpetasAddons", ids: ["addon-dominos"], elegibles: ["addon-dominos"] },
    ]);
    expect(grupoDe("algo-nuevo")).toBeNull();
    expect(grupoDe("toString")).toBeNull();
  });

  it("marcar añade solo las elegibles; desmarcar quita todo el grupo", () => {
    const [, , addons] = agrupar(PIEZAS, (p) => p.segura);
    const otras = new Set(["macros"]);

    const marcadas = alternarGrupo(addons, otras);
    expect([...marcadas]).toEqual(["macros", "dominos"]);
    expect(estadoDe(addons, marcadas)).toBe("marcado");

    // La genérica marcada en avanzado también se va al desmarcar el grupo.
    expect([...alternarGrupo(addons, new Set([...marcadas, "prat"]))]).toEqual(["macros"]);
  });

  it("indeterminado si va algo del grupo pero no todas sus elegibles", () => {
    const [, teclas, addons] = agrupar(PIEZAS, (p) => p.segura);
    expect(estadoDe(teclas, new Set(["clic"]))).toBe("indeterminado");
    expect(estadoDe(addons, new Set(["prat"]))).toBe("indeterminado");
    expect(estadoDe(teclas, new Set(["macros"]))).toBe("desmarcado");
    expect([...alternarGrupo(teclas, new Set(["clic"]))]).toEqual(["clic", "teclas"]);
  });
});
