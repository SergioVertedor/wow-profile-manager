// Nombres propios de instalaciones y cuentas (#34): qué etiqueta se enseña y cómo queda la detección tras
// renombrar sin volver a leer el disco. Funciones puras: los componentes solo pintan lo que devuelven.
import type { TFunction } from "i18next";
import type { Deteccion, Instalacion } from "../nucleo";

/** Lo mismo que hace el núcleo con lo que escribe el usuario (`nombre_valido`): recortado, y vacío es sin nombre. */
export function normalizarNombre(texto: string): string | null {
  const nombre = texto.trim();
  return nombre === "" ? null : nombre;
}

/**
 * El número de «Instalación N»: su puesto en la lista, primero las encontradas (registro y luego a mano, en orden de
 * alta) y después las no encontradas. Puede cambiar al quitar una: por eso la ruta va siempre al lado.
 */
export function numeroDeInstalacion(deteccion: Deteccion, raiz: string): number {
  const raices = [...deteccion.instalaciones, ...deteccion.noEncontradas].map((i) => i.raiz);
  return raices.indexOf(raiz) + 1;
}

/** «Instalación N»: lo que se ve cuando no tiene nombre propio. */
export function nombrePorDefecto(t: TFunction, deteccion: Deteccion, raiz: string): string {
  return t("instalaciones.porDefecto", { numero: numeroDeInstalacion(deteccion, raiz) });
}

/** El nombre propio o, si no tiene, «Instalación N». */
export function etiquetaDeInstalacion(
  t: TFunction,
  deteccion: Deteccion,
  instalacion: Readonly<{ raiz: string; nombre: string | null }>,
): string {
  return instalacion.nombre ?? nombrePorDefecto(t, deteccion, instalacion.raiz);
}

/** Nombre legible de los productos conocidos; los demás, tal cual vienen de `.flavor.info`. */
export function nombreDeProducto(t: TFunction, producto: string): string {
  return producto === "wow" || producto === "wowf" ? t(`productos.${producto}`) : producto;
}

export function conNombreDeInstalacion(deteccion: Deteccion, raiz: string, nombre: string | null): Deteccion {
  const renombrar = <T extends { raiz: string; nombre: string | null }>(item: T): T =>
    item.raiz === raiz ? { ...item, nombre } : item;
  return {
    ...deteccion,
    instalaciones: deteccion.instalaciones.map(renombrar),
    noEncontradas: deteccion.noEncontradas.map(renombrar),
  };
}

export function conNombreDeCuenta(
  deteccion: Deteccion,
  donde: Readonly<{ raiz: string; version: string; cuenta: string }>,
  nombre: string | null,
): Deteccion {
  const renombrarEn = (instalacion: Instalacion): Instalacion => ({
    ...instalacion,
    versiones: instalacion.versiones.map((version) =>
      version.carpeta === donde.version
        ? {
            ...version,
            cuentas: {
              ...version.cuentas,
              cuentas: version.cuentas.cuentas.map((cuenta) =>
                cuenta.carpeta === donde.cuenta ? { ...cuenta, nombre } : cuenta,
              ),
            },
          }
        : version,
    ),
  });
  return {
    ...deteccion,
    instalaciones: deteccion.instalaciones.map((i) => (i.raiz === donde.raiz ? renombrarEn(i) : i)),
  };
}
