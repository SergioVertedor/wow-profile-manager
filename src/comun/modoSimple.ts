// Modo simple de Exportar e Importar (#105): una casilla por grupo de categorías en lugar de una por pieza. Cada
// pantalla dice qué piezas son elegibles en simple; el resto solo se elige en el modo avanzado.

export type Modo = "simple" | "avanzado";

export type GrupoSimple =
  | "macros"
  | "teclas"
  | "interfaz"
  | "modoEdicion"
  | "addonsActivos"
  | "configAddons"
  | "carpetasAddons";

/** Orden en que salen las casillas. */
export const GRUPOS_SIMPLES: readonly GrupoSimple[] = [
  "macros",
  "teclas",
  "interfaz",
  "modoEdicion",
  "addonsActivos",
  "configAddons",
  "carpetasAddons",
];

const GRUPO_DE_CATEGORIA: ReadonlyMap<string, GrupoSimple> = new Map<string, GrupoSimple>([
  ["macros", "macros"],
  ["teclas", "teclas"],
  ["teclas-clic", "teclas"],
  ["opciones", "interfaz"],
  ["chat", "interfaz"],
  ["ventanas", "interfaz"],
  ["marcos-banda", "interfaz"],
  ["modo-edicion", "modoEdicion"],
  ["addons-activos", "addonsActivos"],
  ["config-addon", "configAddons"],
  ["addon", "carpetasAddons"],
]);

/** El grupo de una categoría. Una que esta versión no conoce (paquete más nuevo) no tiene: solo en avanzado. */
export function grupoDe(categoria: string): GrupoSimple | null {
  return GRUPO_DE_CATEGORIA.get(categoria) ?? null;
}

interface Pieza {
  id: string;
  categoria: string;
}

export interface Grupo {
  grupo: GrupoSimple;
  /** Todas las piezas del grupo, elegibles o no: desmarcar el grupo las quita todas. */
  ids: readonly string[];
  /** Las que marca la casilla. Nunca vacío: un grupo sin elegibles no sale. */
  elegibles: readonly string[];
}

/** Los grupos con alguna pieza elegible, en el orden de `GRUPOS_SIMPLES`. */
export function agrupar<P extends Pieza>(piezas: readonly P[], elegible: (pieza: P) => boolean): Grupo[] {
  return GRUPOS_SIMPLES.flatMap((grupo) => {
    const delGrupo = piezas.filter((p) => grupoDe(p.categoria) === grupo);
    const elegibles = delGrupo.filter(elegible).map((p) => p.id);
    return elegibles.length === 0 ? [] : [{ grupo, ids: delGrupo.map((p) => p.id), elegibles }];
  });
}

export type EstadoGrupo = "marcado" | "indeterminado" | "desmarcado";

/** Marcado si van todas sus elegibles; indeterminado si va algo del grupo pero no todas (se tocó en avanzado). */
export function estadoDe(grupo: Grupo, marcadas: ReadonlySet<string>): EstadoGrupo {
  if (grupo.elegibles.every((id) => marcadas.has(id))) {
    return "marcado";
  }
  return grupo.ids.some((id) => marcadas.has(id)) ? "indeterminado" : "desmarcado";
}

/** Marcado, quita todo el grupo; si no, añade sus elegibles. */
export function alternarGrupo(grupo: Grupo, marcadas: ReadonlySet<string>): Set<string> {
  if (estadoDe(grupo, marcadas) === "marcado") {
    return new Set([...marcadas].filter((id) => !grupo.ids.includes(id)));
  }
  return new Set([...marcadas, ...grupo.elegibles]);
}
