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
