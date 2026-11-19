/** Agrupa por addon, en el orden en que llegan. Lo que no lleva addon va en el grupo `""`. */
export function porAddon<T extends { addon?: string | null }>(elementos: readonly T[]): [string, T[]][] {
  const grupos = new Map<string, T[]>();
  for (const elemento of elementos) {
    const addon = elemento.addon ?? "";
    const grupo = grupos.get(addon);
    if (grupo) {
      grupo.push(elemento);
    } else {
      grupos.set(addon, [elemento]);
    }
  }
  return [...grupos];
}
