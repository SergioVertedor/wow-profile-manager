import { describe, expect, it } from "vitest";
import type { Deteccion, Instalacion, VersionJuego } from "../nucleo";
import { crearTextos } from "../textos";
import {
  conNombreDeCuenta,
  conNombreDeInstalacion,
  etiquetaDeInstalacion,
  normalizarNombre,
  numeroDeInstalacion,
} from "./nombres";

const t = crearTextos("es").t;

function version(carpeta: string): VersionJuego {
  return {
    carpeta,
    producto: "wow",
    soportada: true,
    build: null,
    interfaz: 1,
    cuentas: {
      ignoradas: 0,
      cuentas: [
        { carpeta: "1#1", nombre: null, reinosIgnorados: 0, personajes: [] },
        { carpeta: "1#2", nombre: null, reinosIgnorados: 0, personajes: [] },
      ],
    },
    addons: { addons: [], ignoradas: 0 },
  };
}

function instalacion(raiz: string, nombre: string | null = null): Instalacion {
  return { raiz, fuente: "registro", nombre, versiones: [version("_retail_"), version("_forever_")] };
}

const DETECCION: Deteccion = {
  instalaciones: [instalacion("C:\\A", "Casa"), instalacion("C:\\B")],
  noEncontradas: [{ raiz: "E:\\Perdida", nombre: null }],
  preferencias: null,
};

describe("nombres", () => {
  it("normaliza como el núcleo: recorta, y vacío es sin nombre", () => {
    expect(normalizarNombre("  Casa ")).toBe("Casa");
    expect(normalizarNombre("   ")).toBeNull();
  });

  it("numera por el puesto en la lista: encontradas y luego no encontradas", () => {
    expect(numeroDeInstalacion(DETECCION, "C:\\A")).toBe(1);
    expect(numeroDeInstalacion(DETECCION, "C:\\B")).toBe(2);
    expect(numeroDeInstalacion(DETECCION, "E:\\Perdida")).toBe(3);
  });

  it("el nombre propio gana; sin él, «Instalación N»", () => {
    expect(etiquetaDeInstalacion(t, DETECCION, DETECCION.instalaciones[0])).toBe("Casa");
    expect(etiquetaDeInstalacion(t, DETECCION, DETECCION.instalaciones[1])).toBe("Instalación 2");
    expect(etiquetaDeInstalacion(t, DETECCION, DETECCION.noEncontradas[0])).toBe("Instalación 3");
  });

  it("renombra una instalación, encontrada o no, sin tocar las demás", () => {
    const despues = conNombreDeInstalacion(conNombreDeInstalacion(DETECCION, "C:\\B", "Portátil"), "E:\\Perdida", "Vieja");
    expect(despues.instalaciones.map((i) => i.nombre)).toEqual(["Casa", "Portátil"]);
    expect(despues.noEncontradas[0].nombre).toBe("Vieja");
    expect(conNombreDeInstalacion(DETECCION, "C:\\A", null).instalaciones[0].nombre).toBeNull();
  });

  it("renombra solo la cuenta de esa instalación y esa versión", () => {
    const despues = conNombreDeCuenta(DETECCION, { raiz: "C:\\B", version: "_retail_", cuenta: "1#2" }, "Alts");
    const nombres = despues.instalaciones.flatMap((i) =>
      i.versiones.flatMap((v) => v.cuentas.cuentas.map((c) => `${i.raiz}|${v.carpeta}|${c.carpeta}=${c.nombre}`)),
    );
    expect(nombres.filter((n) => !n.endsWith("=null"))).toEqual(["C:\\B|_retail_|1#2=Alts"]);
  });
});
