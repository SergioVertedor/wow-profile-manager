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

/** Idioma de la interfaz de Windows. Si el núcleo no responde, inglés: la app tiene que poder arrancar igual. */
export function pedirIdioma(): Promise<Idioma> {
  return invoke<Idioma>("idioma_interfaz").catch((): Idioma => "en");
}
