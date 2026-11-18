// Nombres que se enseñan al importar. Una categoría que esta versión no conoce (de una 1.x más nueva) se enseña tal cual
// viene: nunca se inventa un nombre.
import type { TFunction } from "i18next";
import type { Categoria } from "../nucleo";

const CATEGORIAS: readonly Categoria[] = [
  "macros",
  "teclas",
  "teclas-clic",
  "opciones",
  "chat",
  "ventanas",
  "marcos-banda",
  "addons-activos",
];

/** Del formato 1.0, pero esta versión aún no las importa (spec 024): solo se nombran. */
const OTRAS: readonly string[] = ["config-addon", "modo-edicion"];
type Otra = "config-addon" | "modo-edicion";

function esCategoria(texto: string): texto is Categoria {
  return (CATEGORIAS as readonly string[]).includes(texto);
}

function esOtra(texto: string): texto is Otra {
  return OTRAS.includes(texto);
}

/** Las entradas de cuenta llevan su propio nombre: «Macros de la cuenta», «Opciones de la cuenta». */
export function etiquetaDeCategoria(t: TFunction, categoria: string, deCuenta: boolean): string {
  if (esOtra(categoria)) {
    return t(`importar.otrasCategorias.${categoria}`);
  }
  if (!esCategoria(categoria)) {
    return categoria;
  }
  const nombre = t(`exportar.categorias.${categoria}`);
  return deCuenta ? t("importar.deLaCuenta", { categoria: nombre }) : nombre;
}
