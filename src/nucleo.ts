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
  autor: string;
  repositorio: string;
  koFi: string;
  licencia: string;
}

export function pedirInfoApp(): Promise<InfoApp> {
  return invoke<InfoApp>("info_app");
}

/** Error del núcleo: código estable y sus datos (`src-tauri/src/error.rs`). */
export type ErrorNucleo =
  | { codigo: "lecturaFallida"; datos: { ruta: string; detalle: string } }
  | { codigo: "enlaceNoAbierto"; datos: { url: string; detalle: string } }
  | { codigo: "escrituraFallida"; datos: { ruta: string; detalle: string } }
  | { codigo: "preferenciasIlegibles"; datos: { ruta: string } }
  | { codigo: "nombreNoValido"; datos: { maximo: number } };

export function esErrorNucleo(valor: unknown): valor is ErrorNucleo {
  return typeof valor === "object" && valor !== null && "codigo" in valor && "datos" in valor;
}

/** Lo que devuelve `detectar_instalaciones` (`src-tauri/src/deteccion.rs`, `descubrimiento.rs` y `addons.rs`). */
export interface Deteccion {
  instalaciones: Instalacion[];
  /** Carpetas añadidas a mano que ya no son una instalación: se enseñan hasta que el usuario las quita. */
  noEncontradas: CarpetaPerdida[];
  /** Por qué no se pudieron leer las preferencias; `null` si se leyeron. Se detecta igual, sin ellas. */
  preferencias: ErrorNucleo | null;
}

export interface Instalacion {
  raiz: string;
  fuente: "registro" | "manual";
  /** Nombre propio que le ha puesto el usuario; `null` si no tiene. */
  nombre: string | null;
  versiones: VersionJuego[];
}

export interface CarpetaPerdida {
  raiz: string;
  nombre: string | null;
}

export interface VersionJuego {
  carpeta: string;
  producto: string;
  soportada: boolean;
  build: string | null;
  interfaz: string | null;
  cuentas: { cuentas: Cuenta[]; ignoradas: number };
  addons: { addons: Addon[]; ignoradas: number };
}

/** Un addon instalado, con lo que dice su `.toc`. Lo que el `.toc` no trae llega vacío, nunca inventado. */
export interface Addon {
  carpeta: string;
  toc: string;
  titulo: string | null;
  version: string | null;
  enlace: string | null;
  dependencias: string[];
  svCuenta: string[];
  svPersonaje: string[];
}

export interface Cuenta {
  carpeta: string;
  /** Nombre propio que le ha puesto el usuario; `null` si no tiene. */
  nombre: string | null;
  personajes: Personaje[];
  reinosIgnorados: number;
}

/** La identidad es la carpeta. `nombreEnDatos` es `null` si no se pudo resolver: nunca se inventa. */
export interface Personaje {
  carpetaReino: string;
  carpetaPersonaje: string;
  nombreEnDatos: { texto: string; origen: "datos" | "carpeta" } | null;
}

export function pedirDeteccion(): Promise<Deteccion> {
  return invoke<Deteccion>("detectar_instalaciones");
}

/** Qué pasó al añadir una carpeta a mano (`Alta` en `src-tauri/src/deteccion.rs`). */
export type Alta = "cancelada" | "anadida" | "yaEstaba" | "noEsInstalacion";

/** Abre el selector de carpetas del sistema desde el núcleo: la interfaz no elige rutas. */
export function anadirCarpeta(): Promise<Alta> {
  return invoke<Alta>("anadir_carpeta");
}

/** Olvida una carpeta añadida a mano, con sus nombres. No toca la carpeta. */
export function quitarCarpeta(raiz: string): Promise<void> {
  return invoke("quitar_carpeta", { raiz });
}

/** Un nombre vacío borra el que hubiera. */
export function nombrarInstalacion(raiz: string, nombre: string): Promise<void> {
  return invoke("nombrar_instalacion", { raiz, nombre });
}

/** Un nombre vacío borra el que hubiera. */
export function nombrarCuenta(raiz: string, version: string, cuenta: string, nombre: string): Promise<void> {
  return invoke("nombrar_cuenta", { raiz, version, cuenta, nombre });
}

/** Enlaces que sabe abrir el núcleo (`src-tauri/src/enlaces.rs`). La interfaz nunca le pasa una URL. */
export type Enlace = "repositorio" | "koFi" | "licencia";

export function abrirEnlace(enlace: Enlace): Promise<void> {
  return invoke("abrir_enlace", { enlace });
}

/** Idioma de la interfaz de Windows. Si el núcleo no responde, inglés: la app tiene que poder arrancar igual. */
export function pedirIdioma(): Promise<Idioma> {
  return invoke<Idioma>("idioma_interfaz").catch((): Idioma => "en");
}
