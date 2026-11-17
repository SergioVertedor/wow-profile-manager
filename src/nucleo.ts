// Única puerta de la interfaz al núcleo en Rust. Ningún componente llama a `invoke` directamente: así los nombres de
// los comandos y los tipos que devuelven viven en un solo sitio.
//
// Convención para los mensajes del núcleo: Rust no devuelve textos para el usuario, sino un código y sus datos; la
// interfaz los traduce con `src/textos/`.
import { invoke } from "@tauri-apps/api/core";
import type { Idioma } from "./textos";

/** Lo que devuelve el comando `info_app` (ver `src-tauri/src/info.rs`). */
export interface InfoApp {
  nombre: string;
  version: string;
}

export function pedirInfoApp(): Promise<InfoApp> {
  return invoke<InfoApp>("info_app");
}

/** Error del núcleo: código estable y sus datos (`src-tauri/src/error.rs`). */
export type ErrorNucleo = { codigo: "lecturaFallida"; datos: { ruta: string; detalle: string } };

export function esErrorNucleo(valor: unknown): valor is ErrorNucleo {
  return typeof valor === "object" && valor !== null && "codigo" in valor && "datos" in valor;
}

/** Lo que devuelve `detectar_instalaciones` (`src-tauri/src/deteccion.rs` y `descubrimiento.rs`). */
export interface Instalacion {
  raiz: string;
  fuente: "registro";
  versiones: VersionJuego[];
}

export interface VersionJuego {
  carpeta: string;
  producto: string;
  soportada: boolean;
  build: string | null;
  interfaz: string | null;
  cuentas: { cuentas: Cuenta[]; ignoradas: number };
}

export interface Cuenta {
  carpeta: string;
  personajes: Personaje[];
  reinosIgnorados: number;
}

/** La identidad es la carpeta; el nombre del personaje desde los datos llega con #37. */
export interface Personaje {
  carpetaReino: string;
  carpetaPersonaje: string;
}

export function pedirInstalaciones(): Promise<Instalacion[]> {
  return invoke<Instalacion[]>("detectar_instalaciones");
}

/** Idioma de la interfaz de Windows. Si el núcleo no responde, inglés: la app tiene que poder arrancar igual. */
export function pedirIdioma(): Promise<Idioma> {
  return invoke<Idioma>("idioma_interfaz").catch((): Idioma => "en");
}
