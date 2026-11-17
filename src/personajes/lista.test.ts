import { describe, expect, it } from "vitest";
import type { Personaje } from "../nucleo";
import {
  agruparPorReino,
  claveDePersonaje,
  filtrar,
  inicial,
  muestraReinos,
  nombreVisible,
  numerosDeCuenta,
} from "./lista";

function pj(carpetaReino: string, carpetaPersonaje: string, texto: string | null = null): Personaje {
  return { carpetaReino, carpetaPersonaje, nombreEnDatos: texto === null ? null : { texto, origen: "datos" } };
}

describe("numerosDeCuenta", () => {
  it("usa el número de licencia de cada cuenta", () => {
    expect(numerosDeCuenta(["107710745#1", "107710745#2"])).toEqual(
      new Map([
        ["107710745#1", "1"],
        ["107710745#2", "2"],
      ]),
    );
  });

  it("si dos cuentas darían el mismo número, ninguna lo usa", () => {
    expect(numerosDeCuenta(["111#1", "222#1", "222#2"])).toEqual(
      new Map([
        ["111#1", null],
        ["222#1", null],
        ["222#2", "2"],
      ]),
    );
  });

  it("sin «#» o sin número detrás, la carpeta completa", () => {
    expect(numerosDeCuenta(["CUENTA", "1#"])).toEqual(
      new Map([
        ["CUENTA", null],
        ["1#", null],
      ]),
    );
  });
});

describe("muestraReinos", () => {
  it("en Forever no hay reinos; en el resto, sí", () => {
    expect(muestraReinos("wowf")).toBe(false);
    expect(muestraReinos("wow")).toBe(true);
  });
});

describe("agruparPorReino", () => {
  it("agrupa respetando el orden de llegada", () => {
    const lista = [pj("Dun Modr", "Allagan"), pj("C'Thun", "Angharad"), pj("Dun Modr", "Goldberg")];
    expect(agruparPorReino(lista)).toEqual([
      { reino: "Dun Modr", personajes: [lista[0], lista[2]] },
      { reino: "C'Thun", personajes: [lista[1]] },
    ]);
  });

  it("sin personajes, sin grupos", () => {
    expect(agruparPorReino([])).toEqual([]);
  });
});

describe("nombres", () => {
  it("el nombre resuelto o la carpeta tal cual", () => {
    expect(nombreVisible(pj("93", "Goldberg-Joestar", "Goldberg Joestar"))).toBe("Goldberg Joestar");
    expect(nombreVisible(pj("93", "Goldber8D31D"))).toBe("Goldber8D31D");
  });

  it("quita el reino de la clave AceDB de Retail, solo si es el suyo", () => {
    expect(nombreVisible(pj("C'Thun", "Angharad", "Angharad - C'Thun"))).toBe("Angharad");
    expect(nombreVisible(pj("C'Thun", "Angharad", "Angharad - Uldum"))).toBe("Angharad - Uldum");
  });

  it("la clave es reino y carpeta", () => {
    expect(claveDePersonaje(pj("Dun Modr", "Goldberg"))).toBe("Dun Modr/Goldberg");
  });

  it("la inicial no parte pares sustitutos y va en mayúscula", () => {
    expect(inicial("øyvind")).toBe("Ø");
    expect(inicial("𝔊old")).toBe("𝔊");
    expect(inicial("")).toBe("?");
  });
});

describe("filtrar", () => {
  const lista = [pj("C'Thun", "Gøldberg"), pj("Dun Modr", "Apañerouno"), pj("93", "Goldberg-Sp", "Goldberg Sp")];

  it("sin distinguir mayúsculas ni acentos", () => {
    expect(filtrar(lista, "APANE").map(nombreVisible)).toEqual(["Apañerouno"]);
    expect(filtrar(lista, "goldberg").map(nombreVisible)).toEqual(["Goldberg Sp"]);
  });

  it("también por la carpeta", () => {
    expect(filtrar(lista, "berg-sp").map(nombreVisible)).toEqual(["Goldberg Sp"]);
  });

  it("una consulta vacía o en blanco lo deja todo", () => {
    expect(filtrar(lista, "  ")).toEqual(lista);
  });

  it("sin coincidencias, vacío", () => {
    expect(filtrar(lista, "zz")).toEqual([]);
  });
});
