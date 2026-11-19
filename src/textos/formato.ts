// Formatos de números para la interfaz, en el idioma de los textos.

const UN_MEGA = 1_000_000;

/** Tamaño en kB o MB, con las unidades decimales de `Intl` y en el formato del idioma. Nunca «0 kB». */
export function tamanoLegible(bytes: number, idioma: string): string {
  const enMegas = bytes >= UN_MEGA;
  return new Intl.NumberFormat(idioma, {
    style: "unit",
    unit: enMegas ? "megabyte" : "kilobyte",
    maximumFractionDigits: enMegas ? 1 : 0,
  }).format(enMegas ? bytes / UN_MEGA : Math.max(1, Math.round(bytes / 1000)));
}

/** Fecha y hora locales de una marca RFC 3339 (las del núcleo van en UTC), en el formato del idioma. Si la marca no se
 * entiende (un diario tocado a mano), se devuelve tal cual: una fecha rara no puede dejar la pantalla en blanco. */
export function fechaLegible(rfc3339: string, idioma: string): string {
  const fecha = new Date(rfc3339);
  if (Number.isNaN(fecha.getTime())) {
    return rfc3339;
  }
  return new Intl.DateTimeFormat(idioma, { dateStyle: "medium", timeStyle: "short" }).format(fecha);
}
