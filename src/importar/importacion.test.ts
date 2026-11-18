import { describe, expect, it } from "vitest";
import type { Deteccion, Personaje, ResumenPaquete, VersionJuego } from "../nucleo";
import { entradasDeVersion } from "../personajes/seleccion";
import {
  alternarDestino,
  decisionesPendientes,
  marcadosEnCuenta,
  pedidoDe,
  seleccionPorDefecto,
  versionesDelProducto,
  claveDeDecision,
} from "./importacion";

function version(carpeta: string, producto: string, soportada = true): VersionJuego {
  return {
    carpeta,
    producto,
    soportada,
    build: null,
    interfaz: null,
    cuentas: { cuentas: [], ignoradas: 0 },
    addons: { addons: [], ignoradas: 0 },
  };
}

const DETECCION: Deteccion = {
  instalaciones: [
    { raiz: "C:/A", fuente: "registro", nombre: null, versiones: [version("_forever_", "wowf"), version("_retail_", "wow")] },
    { raiz: "D:/B", fuente: "manual", nombre: null, versiones: [version("_forever_", "wowf", false)] },
  ],
  noEncontradas: [],
  preferencias: null,
};

const RESUMEN: ResumenPaquete = {
  sha256: "a".repeat(64),
  manifiesto: {
    formato: { nombre: "wowprofile", version: "1.0" },
    perfil: { nombre: "Druida", creado: "2026-11-18T10:00:00Z" },
    origen: { producto: "wowf", build: "1.60.1.70938", interfaz: 16001 },
    addons: [],
  },
  entradas: [
    { id: "macros", categoria: "macros", ambito: "personaje", deCuenta: false, bytes: 10 },
    { id: "macros-cuenta", categoria: "macros", ambito: "cuenta", deCuenta: true, bytes: 10 },
    { id: "dominos", categoria: "config-addon", ambito: "personaje", deCuenta: false, bytes: 1, incompatible: "noSeImporta" },
  ],
};

const PJ: Personaje = { carpetaReino: "93", carpetaPersonaje: "Goldberg-Joestar", nombreEnDatos: null };
const OTRO: Personaje = { carpetaReino: "93", carpetaPersonaje: "Otro", nombreEnDatos: null };

describe("importacion", () => {
  it("solo ofrece versiones soportadas del producto del paquete", () => {
    expect(versionesDelProducto(DETECCION, "wowf").map((e) => e.clave)).toEqual(["C:/A|_forever_"]);
    expect(versionesDelProducto(DETECCION, "wowc")).toEqual([]);
  });

  it("marca por defecto lo que se puede importar", () => {
    expect([...seleccionPorDefecto(RESUMEN)]).toEqual(["macros", "macros-cuenta"]);
  });

  it("alterna destinos por cuenta y los devuelve como los entiende la lista", () => {
    const entrada = entradasDeVersion(DETECCION.instalaciones)[0];
    let destinos = alternarDestino(new Map(), entrada, "1#1", PJ);
    destinos = alternarDestino(destinos, entrada, "2#1", OTRO);
    expect([...destinos.values()].map((d) => d.pedido)).toEqual([
      { raiz: "C:/A", version: "_forever_", cuenta: "1#1", reino: "93", personaje: "Goldberg-Joestar" },
      { raiz: "C:/A", version: "_forever_", cuenta: "2#1", reino: "93", personaje: "Otro" },
    ]);
    expect([...marcadosEnCuenta(destinos, "1#1")]).toEqual(["93/Goldberg-Joestar"]);
    expect([...marcadosEnCuenta(destinos, "1")]).toEqual([]);
    destinos = alternarDestino(destinos, entrada, "1#1", PJ);
    expect(destinos.size).toBe(1);
  });

  it("pide decidir cada entrada de cuenta una vez por cuenta y solo manda lo decidido", () => {
    const entrada = entradasDeVersion(DETECCION.instalaciones)[0];
    const destinos = [
      ...alternarDestino(alternarDestino(alternarDestino(new Map(), entrada, "1#1", PJ), entrada, "1#1", OTRO), entrada, "2#1", PJ).values(),
    ];
    const seleccion = seleccionPorDefecto(RESUMEN);
    expect(decisionesPendientes(RESUMEN, seleccion, destinos)).toEqual([
      { cuenta: "1#1", entrada: "macros-cuenta" },
      { cuenta: "2#1", entrada: "macros-cuenta" },
    ]);
    const decisiones = new Map([[claveDeDecision("1#1", "macros-cuenta"), true]]);
    expect(pedidoDe(RESUMEN, seleccion, destinos, decisiones)).toEqual({
      seleccion: ["macros", "macros-cuenta"],
      cuentas: [{ cuenta: "1#1", entrada: "macros-cuenta", aceptar: true }],
    });
    expect(decisionesPendientes(RESUMEN, new Set(["macros"]), destinos)).toEqual([]);
  });
});
