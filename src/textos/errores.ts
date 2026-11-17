// Traduce lo que rechaza un comando del núcleo. Un `ErrorNucleo` tiene su texto en `errores.<codigo>`; cualquier
// otra cosa (un fallo del propio canal con Rust, o un código que esta interfaz aún no conoce) se enseña como error
// inesperado, sin ocultarla y nunca como una clave en crudo.
import type { TFunction } from "i18next";
import { esErrorNucleo } from "../nucleo";

export function mensajeDeError(t: TFunction, error: unknown): string {
  if (esErrorNucleo(error)) {
    const desconocido = t("errores.desconocido", { mensaje: error.codigo });
    return t(`errores.${error.codigo}`, { ...error.datos, defaultValue: desconocido });
  }
  return t("errores.desconocido", { mensaje: String(error) });
}
