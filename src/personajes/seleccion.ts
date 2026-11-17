// Qué versión, cuenta y personaje están elegidos (spec 017). Se guarda solo lo que el jugador eligió; lo que falta o
// ya no existe se completa con el primero disponible, como en la selección de personaje del juego: mientras haya
// personajes, siempre hay uno elegido.
import type { Cuenta, Instalacion, Personaje, VersionJuego } from "../nucleo";
import { claveDePersonaje } from "./lista";

/** Una versión del juego dentro de su instalación. La clave las distingue aunque dos instalaciones repitan carpeta. */
export type EntradaVersion = Readonly<{ clave: string; instalacion: Instalacion; version: VersionJuego }>;

/** Lo que el jugador ha elegido; `null` = aún nada, se toma el primero. */
export type Eleccion = Readonly<{ version: string | null; cuenta: string | null; personaje: string | null }>;

export const SIN_ELEGIR: Eleccion = { version: null, cuenta: null, personaje: null };

export type Elegido = Readonly<{ entrada: EntradaVersion; cuenta: Cuenta | null; personaje: Personaje | null }>;

export function entradasDeVersion(instalaciones: readonly Instalacion[]): EntradaVersion[] {
  return instalaciones.flatMap((instalacion) =>
    instalacion.versiones.map((version) => ({ clave: `${instalacion.raiz}|${version.carpeta}`, instalacion, version })),
  );
}

export function contarPersonajes(version: VersionJuego): number {
  return version.cuentas.cuentas.reduce((total, cuenta) => total + cuenta.personajes.length, 0);
}

/** Completa la elección. Sin versiones, no hay nada que elegir. La versión por defecto es la primera con personajes. */
export function resolverEleccion(entradas: readonly EntradaVersion[], eleccion: Eleccion): Elegido | null {
  const entrada =
    entradas.find((e) => e.clave === eleccion.version) ??
    entradas.find((e) => contarPersonajes(e.version) > 0) ??
    entradas[0];
  if (entrada === undefined) {
    return null;
  }
  const cuentas = entrada.version.cuentas.cuentas;
  const cuenta = cuentas.find((c) => c.carpeta === eleccion.cuenta) ?? cuentas[0] ?? null;
  const personajes = cuenta?.personajes ?? [];
  const personaje = personajes.find((p) => claveDePersonaje(p) === eleccion.personaje) ?? personajes[0] ?? null;
  return { entrada, cuenta, personaje };
}
