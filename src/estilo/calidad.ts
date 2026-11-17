// Los colores de calidad de objeto del juego, usados como información: cuánto nos fiamos del nombre de un personaje.
import type { Personaje } from "../nucleo";

export type Calidad = "raro" | "pocoComun" | "pobre";

/** Confirmado en los datos = raro; deducido de la carpeta = poco común; sin resolver = pobre. */
export function calidadDeNombre(nombre: Personaje["nombreEnDatos"]): Calidad {
  if (nombre === null) {
    return "pobre";
  }
  return nombre.origen === "datos" ? "raro" : "pocoComun";
}
