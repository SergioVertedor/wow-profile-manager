// Lógica de la lista de personajes (#45): cómo se llama cada cuenta, cuándo hay reinos, cómo se agrupa y cómo se
// busca. Funciones puras: los componentes solo pintan lo que devuelven.
import type { Personaje } from "../nucleo";

/** Con más personajes que estos en la cuenta aparece el buscador. Medido: Forever 6, Retail hasta 68. */
export const UMBRAL_BUSCADOR = 10;

/**
 * El número de licencia de cada cuenta (`107710745#1` → `1`), que es lo que las distingue dentro de una cuenta de
 * Battle.net. Si dos cuentas darían el mismo número (dos cuentas de Battle.net en el mismo `WTF`), ninguna de las dos
 * lo usa: `null` y se enseña la carpeta completa.
 */
export function numerosDeCuenta(carpetas: readonly string[]): Map<string, string | null> {
  const numeros = carpetas.map((carpeta) => carpeta.slice(carpeta.lastIndexOf("#") + 1));
  const veces = new Map<string, number>();
  for (const numero of numeros) {
    veces.set(numero, (veces.get(numero) ?? 0) + 1);
  }
  return new Map(
    carpetas.map((carpeta, i) => {
      const numero = numeros[i];
      const usable = carpeta.includes("#") && numero !== "" && veces.get(numero) === 1;
      return [carpeta, usable ? numero : null];
    }),
  );
}

/** WoW Forever no tiene reinos como tal (su única carpeta de reino es `93`): ahí no se enseñan. */
export function muestraReinos(producto: string): boolean {
  return producto !== "wowf";
}

export type GrupoReino = Readonly<{ reino: string; personajes: Personaje[] }>;

/** Agrupa por carpeta de reino respetando el orden en que llegan reinos y personajes. */
export function agruparPorReino(personajes: readonly Personaje[]): GrupoReino[] {
  const grupos = new Map<string, Personaje[]>();
  for (const personaje of personajes) {
    const grupo = grupos.get(personaje.carpetaReino);
    if (grupo === undefined) {
      grupos.set(personaje.carpetaReino, [personaje]);
    } else {
      grupo.push(personaje);
    }
  }
  return Array.from(grupos, ([reino, lista]) => ({ reino, personajes: lista }));
}

/**
 * El nombre resuelto o, si no se pudo, la carpeta tal cual. Nunca se inventa. En Retail el núcleo da la clave AceDB,
 * `"Nombre - Reino"` (`candidato` en `src-tauri/src/descubrimiento.rs`); el reino ya se enseña aparte, así que se
 * quita cuando el final es exactamente la carpeta de reino.
 */
export function nombreVisible(personaje: Personaje): string {
  const texto = personaje.nombreEnDatos?.texto ?? personaje.carpetaPersonaje;
  const sufijoReino = ` - ${personaje.carpetaReino}`;
  return texto.endsWith(sufijoReino) ? texto.slice(0, -sufijoReino.length) : texto;
}

/** Identifica a un personaje dentro de su cuenta. */
export function claveDePersonaje(personaje: Personaje): string {
  return `${personaje.carpetaReino}/${personaje.carpetaPersonaje}`;
}

/** La primera letra completa (sin partir pares sustitutos), en mayúscula. */
export function inicial(texto: string): string {
  return (Array.from(texto)[0] ?? "?").toLocaleUpperCase();
}

/** Busca por nombre o carpeta sin distinguir mayúsculas ni acentos. Una consulta vacía lo deja todo. */
export function filtrar(personajes: readonly Personaje[], consulta: string): Personaje[] {
  const buscada = normalizar(consulta.trim());
  if (buscada === "") {
    return [...personajes];
  }
  return personajes.filter(
    (personaje) =>
      normalizar(nombreVisible(personaje)).includes(buscada) ||
      normalizar(personaje.carpetaPersonaje).includes(buscada),
  );
}

function normalizar(texto: string): string {
  // `replace` con `/g` y no `replaceAll`: el `lib` de TypeScript del proyecto es anterior a ES2021.
  return texto.normalize("NFD").replace(/\p{M}/gu, "").toLocaleLowerCase();
}
