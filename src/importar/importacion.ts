// Lo que decide la vista «Importar» (#63) sin pintar nada: de qué versiones salen los destinos, qué se marca por
// defecto, qué hay que decidir en cada cuenta y cómo se arma lo que se pide al núcleo.
import type {
  DecisionPerfil,
  Deteccion,
  FicheroPlan,
  Grupo,
  PedidoImportacion,
  Personaje,
  PersonajePedido,
  PlanPersonaje,
  ResumenPaquete,
} from "../nucleo";
import { claveDePersonaje, nombreVisible } from "../personajes/lista";
import { entradasDeVersion, type EntradaVersion } from "../personajes/seleccion";

/** Un personaje marcado como destino, con el nombre que se enseña. */
export type Destino = Readonly<{ pedido: PersonajePedido; nombre: string }>;

/** Solo las versiones del juego del paquete: Retail y Forever no se mezclan (#21). */
export function versionesDelProducto(deteccion: Deteccion, producto: string): EntradaVersion[] {
  return entradasDeVersion(deteccion.instalaciones).filter(
    (entrada) => entrada.version.soportada && entrada.version.producto === producto,
  );
}

/** Identifica un destino dentro de una versión: la cuenta y la clave del personaje en ella. */
export function claveDeDestino(cuenta: string, clavePersonaje: string): string {
  return `${cuenta}|${clavePersonaje}`;
}

/** Marca o desmarca a un personaje de una cuenta de esa versión como destino. */
export function alternarDestino(
  destinos: ReadonlyMap<string, Destino>,
  entrada: EntradaVersion,
  cuenta: string,
  personaje: Personaje,
): Map<string, Destino> {
  const siguiente = new Map(destinos);
  const clave = claveDeDestino(cuenta, claveDePersonaje(personaje));
  if (!siguiente.delete(clave)) {
    siguiente.set(clave, {
      nombre: nombreVisible(personaje),
      pedido: {
        raiz: entrada.instalacion.raiz,
        version: entrada.version.carpeta,
        cuenta,
        reino: personaje.carpetaReino,
        personaje: personaje.carpetaPersonaje,
      },
    });
  }
  return siguiente;
}

/** Las claves de personaje marcadas en una cuenta, como las entiende la lista. */
export function marcadosEnCuenta(destinos: ReadonlyMap<string, Destino>, cuenta: string): Set<string> {
  const prefijo = claveDeDestino(cuenta, "");
  return new Set(
    [...destinos.keys()].filter((k) => k.startsWith(prefijo)).map((k) => k.slice(prefijo.length)),
  );
}

/**
 * Marcado por defecto: lo que se puede importar, salvo los ficheros genéricos de la configuración de addons (el paquete
 * no lleva firma: no se fía de lo que marcó quien lo hizo). Las de cuenta se deciden además en cada cuenta.
 */
export function seleccionPorDefecto(resumen: ResumenPaquete): Set<string> {
  return new Set(resumen.entradas.filter((e) => e.incompatible === undefined && e.marcada).map((e) => e.id));
}

/** Lo que hay que decidir: cada entrada de cuenta marcada en cada cuenta de los destinos, una sola vez. */
export function decisionesPendientes(
  resumen: ResumenPaquete,
  seleccion: ReadonlySet<string>,
  destinos: readonly Destino[],
): { cuenta: string; entrada: string }[] {
  const cuentas = [...new Set(destinos.map((d) => d.pedido.cuenta))];
  const deCuenta = resumen.entradas.filter(
    (e) => e.deCuenta && e.incompatible === undefined && seleccion.has(e.id),
  );
  return cuentas.flatMap((cuenta) => deCuenta.map((e) => ({ cuenta, entrada: e.id })));
}

export function claveDeDecision(cuenta: string, entrada: string): string {
  return `${cuenta}|${entrada}`;
}

/** El pedido para el núcleo. Lo que no está en `decisiones` queda sin decidir: el plan lo enseña como conflicto. */
export function pedidoDe(
  resumen: ResumenPaquete,
  seleccion: ReadonlySet<string>,
  destinos: readonly Destino[],
  decisiones: ReadonlyMap<string, boolean>,
  perfiles: readonly DecisionPerfil[] = [],
): PedidoImportacion {
  const cuentas = decisionesPendientes(resumen, seleccion, destinos).flatMap(({ cuenta, entrada }) => {
    const aceptar = decisiones.get(claveDeDecision(cuenta, entrada));
    return aceptar === undefined ? [] : [{ cuenta, entrada, aceptar }];
  });
  return { seleccion: [...seleccion], cuentas, perfiles: [...perfiles] };
}

/** Cambia (o pone) lo decidido con un perfil que ya existe en una cuenta. */
export function decidirPerfil(perfiles: readonly DecisionPerfil[], decision: DecisionPerfil): DecisionPerfil[] {
  const otras = perfiles.filter((d) => d.cuenta !== decision.cuenta || d.entrada !== decision.entrada);
  return [...otras, decision];
}

export const GRUPOS: readonly Grupo[] = ["nuevo", "modificado", "sinCambios"];

/** Lo que de verdad se escribe en un personaje al aplicar: lo que no sale «sin cambios». */
export function aEscribir(plan: PlanPersonaje): FicheroPlan[] {
  return plan.ficheros.filter((f) => f.grupo !== "sinCambios");
}

export function delGrupo(ficheros: readonly FicheroPlan[], grupo: Grupo): FicheroPlan[] {
  return ficheros.filter((f) => f.grupo === grupo);
}
