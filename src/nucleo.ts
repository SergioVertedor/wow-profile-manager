// Única puerta de la interfaz al núcleo en Rust. Ningún componente llama a `invoke` directamente: así los nombres de
// los comandos y los tipos que devuelven viven en un solo sitio.
import { invoke } from "@tauri-apps/api/core";

/** Lo que devuelve el comando `info_app` (ver `src-tauri/src/info.rs`). */
export interface InfoApp {
  nombre: string;
  version: string;
}

export function pedirInfoApp(): Promise<InfoApp> {
  return invoke<InfoApp>("info_app");
}

/** Texto de la cabecera: nombre y versión. */
export function textoCabecera(info: InfoApp): string {
  return `${info.nombre} ${info.version}`;
}
