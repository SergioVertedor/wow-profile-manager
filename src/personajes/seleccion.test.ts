import { describe, expect, it } from "vitest";
import type { Cuenta, Instalacion, VersionJuego } from "../nucleo";
import { contarPersonajes, entradasDeVersion, resolverEleccion, SIN_ELEGIR } from "./seleccion";

function cuenta(carpeta: string, ...personajes: string[]): Cuenta {
  return {
    carpeta,
    nombre: null,
    reinosIgnorados: 0,
    personajes: personajes.map((p) => ({ carpetaReino: "R", carpetaPersonaje: p, nombreEnDatos: null })),
  };
}

function version(carpeta: string, ...cuentas: Cuenta[]): VersionJuego {
  return {
    carpeta,
    producto: "wow",
    soportada: true,
    build: null,
    interfaz: 1,
    cuentas: { cuentas, ignoradas: 0 },
    addons: { addons: [], ignoradas: 0 },
  };
}

const INSTALACIONES: Instalacion[] = [
  {
    raiz: "C:\\WoW",
    fuente: "registro",
    nombre: null,
    versiones: [version("_classic_"), version("_retail_", cuenta("1#1", "A", "B"), cuenta("1#2", "C"))],
  },
];
const ENTRADAS = entradasDeVersion(INSTALACIONES);

describe("entradasDeVersion", () => {
  it("una entrada por versión, con clave de instalación y carpeta", () => {
    expect(ENTRADAS.map((e) => e.clave)).toEqual(["C:\\WoW|_classic_", "C:\\WoW|_retail_"]);
  });
});

describe("contarPersonajes", () => {
  it("suma los personajes de todas las cuentas", () => {
    expect(ENTRADAS.map((e) => contarPersonajes(e.version))).toEqual([0, 3]);
  });
});

describe("resolverEleccion", () => {
  it("sin elegir nada: la primera versión con personajes, su primera cuenta y su primer personaje", () => {
    const elegido = resolverEleccion(ENTRADAS, SIN_ELEGIR);
    expect(elegido?.entrada.clave).toBe("C:\\WoW|_retail_");
    expect(elegido?.cuenta?.carpeta).toBe("1#1");
    expect(elegido?.personaje?.carpetaPersonaje).toBe("A");
  });

  it("respeta lo elegido", () => {
    const elegido = resolverEleccion(ENTRADAS, { version: "C:\\WoW|_retail_", cuenta: "1#2", personaje: "R/C" });
    expect(elegido?.cuenta?.carpeta).toBe("1#2");
    expect(elegido?.personaje?.carpetaPersonaje).toBe("C");
  });

  it("un personaje que no está en la cuenta elegida cae en el primero de esa cuenta", () => {
    const elegido = resolverEleccion(ENTRADAS, { version: null, cuenta: "1#2", personaje: "R/A" });
    expect(elegido?.personaje?.carpetaPersonaje).toBe("C");
  });

  it("una versión sin cuentas se puede elegir: sin cuenta ni personaje", () => {
    const elegido = resolverEleccion(ENTRADAS, { ...SIN_ELEGIR, version: "C:\\WoW|_classic_" });
    expect(elegido?.entrada.clave).toBe("C:\\WoW|_classic_");
    expect(elegido?.cuenta).toBeNull();
    expect(elegido?.personaje).toBeNull();
  });

  it("si ninguna versión tiene personajes, la primera", () => {
    const sinNadie = entradasDeVersion([{ ...INSTALACIONES[0], versiones: [version("_a_"), version("_b_")] }]);
    expect(resolverEleccion(sinNadie, SIN_ELEGIR)?.entrada.clave).toBe("C:\\WoW|_a_");
  });

  it("sin versiones no hay nada que elegir", () => {
    expect(resolverEleccion([], SIN_ELEGIR)).toBeNull();
  });
});
