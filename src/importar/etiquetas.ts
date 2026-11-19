// Nombres que se enseñan al importar. Una categoría que esta versión no conoce (de una 1.x más nueva) se enseña tal cual
// viene: nunca se inventa un nombre.
import type { TFunction } from "i18next";
import type { CategoriaFichero, EntradaPaquete } from "../nucleo";

/** El nombre del perfil cuando es el del personaje (`docs/wowprofile-format.md`, «Markers»). */
export const MARCADOR_PERSONAJE = "@@WOWPROFILE:PERSONAJE@@";

const CATEGORIAS: readonly CategoriaFichero[] = [
  "macros",
  "teclas",
  "teclas-clic",
  "opciones",
  "chat",
  "ventanas",
  "modo-edicion",
  "marcos-banda",
  "addons-activos",
];

/** Sin nombre propio de fichero: solo se nombran. `addon` es la carpeta de un addon (formato 1.1). */
const OTRAS: readonly string[] = ["config-addon", "addon"];
type Otra = "config-addon" | "addon";

export function esCategoria(texto: string): texto is CategoriaFichero {
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

/** Qué es una pieza de la configuración de un addon: su perfil, sus ajustes o un fichero entero. */
export function etiquetaDePieza(t: TFunction, entrada: EntradaPaquete): string {
  if (entrada.tipo === "perfil-acedb") {
    return entrada.perfil === MARCADOR_PERSONAJE
      ? t("importar.addons.pieza.perfilDelPersonaje")
      : t("importar.addons.pieza.perfilAcedb", { perfil: entrada.perfil ?? "" });
  }
  if (entrada.tipo === "adaptador") {
    return t("importar.addons.pieza.adaptador", { variable: entrada.variable ?? "" });
  }
  return entrada.ambito === "cuenta" ? t("importar.addons.pieza.genericoCuenta") : t("importar.addons.pieza.genericoPersonaje");
}
