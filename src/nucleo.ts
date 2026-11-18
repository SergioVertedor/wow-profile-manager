// Única puerta de la interfaz al núcleo en Rust. Ningún componente llama a `invoke` directamente: así los nombres de
// los comandos y los tipos que devuelven viven en un solo sitio.
//
// Convención para los mensajes del núcleo: Rust no devuelve textos para el usuario, sino un código y sus datos; la
// interfaz los traduce con `src/textos/`.
import { invoke } from "@tauri-apps/api/core";
import type { Idioma } from "./textos";

/** Lo que devuelve el comando `info_app` (ver `src-tauri/src/info.rs`). */
export interface InfoApp {
  nombre: string;
  version: string;
  autor: string;
  repositorio: string;
  koFi: string;
  licencia: string;
}

export function pedirInfoApp(): Promise<InfoApp> {
  return invoke<InfoApp>("info_app");
}

/** Error del núcleo: código estable y sus datos (`src-tauri/src/error.rs`). */
export type ErrorNucleo =
  | { codigo: "lecturaFallida"; datos: { ruta: string; detalle: string } }
  | { codigo: "enlaceNoAbierto"; datos: { url: string; detalle: string } }
  | { codigo: "escrituraFallida"; datos: { ruta: string; detalle: string } }
  | { codigo: "preferenciasIlegibles"; datos: { ruta: string } }
  | { codigo: "nombreNoValido"; datos: { maximo: number } }
  | { codigo: "juegoAbierto"; datos: Record<string, never> }
  | { codigo: "personajeNoEncontrado"; datos: Record<string, never> }
  | { codigo: "enlaceNoAdmitido"; datos: { ruta: string } }
  | { codigo: "operacionFallida"; datos: { id: string; restaurada: boolean; detalle: string } }
  | { codigo: "operacionNoEncontrada"; datos: Record<string, never> }
  | { codigo: "noDeshacible"; datos: Record<string, never> }
  | { codigo: "operacionPosterior"; datos: { id: string } }
  | { codigo: "juegoDistinto"; datos: { raiz: string } }
  | { codigo: "planCaducado"; datos: Record<string, never> }
  | { codigo: "operacionEnCurso"; datos: Record<string, never> }
  | { codigo: "restauracionManualSinConfirmar"; datos: Record<string, never> }
  | { codigo: "carpetaNoAbierta"; datos: { ruta: string; detalle: string } }
  | { codigo: "exportacionSinVersion"; datos: Record<string, never> }
  | { codigo: "perfilNoValido"; datos: Record<string, never> }
  | { codigo: "seleccionVacia"; datos: Record<string, never> }
  | { codigo: "paqueteDemasiadoGrande"; datos: Record<string, never> }
  | { codigo: "paqueteInvalido"; datos: { motivo: MotivoPaquete; detalle: string } }
  | { codigo: "versionPaquete"; datos: { version: string } }
  | { codigo: "paqueteNoAbierto"; datos: Record<string, never> }
  | { codigo: "destinosNoValidos"; datos: Record<string, never> };

/** Qué paso de «Leer un paquete de forma segura» falló (`MotivoPaquete` en `src-tauri/src/error.rs`). */
export type MotivoPaquete =
  | "noEsZip"
  | "demasiadoGrande"
  | "demasiadasEntradas"
  | "manifiestoIlegible"
  | "manifiestoNoValido"
  | "entradasDelZip"
  | "piezaIlegible"
  | "piezaDemasiadoGrande"
  | "totalDemasiadoGrande"
  | "piezaDanada";

export function esErrorNucleo(valor: unknown): valor is ErrorNucleo {
  return typeof valor === "object" && valor !== null && "codigo" in valor && "datos" in valor;
}

/** Lo que devuelve `detectar_instalaciones` (`src-tauri/src/deteccion.rs`, `descubrimiento.rs` y `addons.rs`). */
export interface Deteccion {
  instalaciones: Instalacion[];
  /** Carpetas añadidas a mano que ya no son una instalación: se enseñan hasta que el usuario las quita. */
  noEncontradas: CarpetaPerdida[];
  /** Por qué no se pudieron leer las preferencias; `null` si se leyeron. Se detecta igual, sin ellas. */
  preferencias: ErrorNucleo | null;
}

export interface Instalacion {
  raiz: string;
  fuente: "registro" | "battlenet" | "manual";
  /** Nombre propio que le ha puesto el usuario; `null` si no tiene. */
  nombre: string | null;
  versiones: VersionJuego[];
}

export interface CarpetaPerdida {
  raiz: string;
  nombre: string | null;
}

export interface VersionJuego {
  carpeta: string;
  producto: string;
  soportada: boolean;
  build: string | null;
  /** Número de interfaz (`engineSurveyPatch`); sin él no se puede exportar. */
  interfaz: number | null;
  cuentas: { cuentas: Cuenta[]; ignoradas: number };
  addons: { addons: Addon[]; ignoradas: number };
}

/** Un addon instalado, con lo que dice su `.toc`. Lo que el `.toc` no trae llega vacío, nunca inventado. */
export interface Addon {
  carpeta: string;
  toc: string;
  titulo: string | null;
  version: string | null;
  enlace: string | null;
  dependencias: string[];
  svCuenta: string[];
  svPersonaje: string[];
}

export interface Cuenta {
  carpeta: string;
  /** Nombre propio que le ha puesto el usuario; `null` si no tiene. */
  nombre: string | null;
  personajes: Personaje[];
  reinosIgnorados: number;
}

/** La identidad es la carpeta. `nombreEnDatos` es `null` si no se pudo resolver: nunca se inventa. */
export interface Personaje {
  carpetaReino: string;
  carpetaPersonaje: string;
  nombreEnDatos: { texto: string; origen: "datos" | "carpeta" } | null;
}

export function pedirDeteccion(): Promise<Deteccion> {
  return invoke<Deteccion>("detectar_instalaciones");
}

/** Qué pasó al añadir una carpeta a mano (`Alta` en `src-tauri/src/deteccion.rs`). */
export type Alta = "cancelada" | "anadida" | "yaEstaba" | "noEsInstalacion";

/** Abre el selector de carpetas del sistema desde el núcleo: la interfaz no elige rutas. */
export function anadirCarpeta(): Promise<Alta> {
  return invoke<Alta>("anadir_carpeta");
}

/** Olvida una carpeta añadida a mano, con sus nombres. No toca la carpeta. */
export function quitarCarpeta(raiz: string): Promise<void> {
  return invoke("quitar_carpeta", { raiz });
}

/** Un nombre vacío borra el que hubiera. */
export function nombrarInstalacion(raiz: string, nombre: string): Promise<void> {
  return invoke("nombrar_instalacion", { raiz, nombre });
}

/** Un nombre vacío borra el que hubiera. */
export function nombrarCuenta(raiz: string, version: string, cuenta: string, nombre: string): Promise<void> {
  return invoke("nombrar_cuenta", { raiz, version, cuenta, nombre });
}

/** Un personaje tal como se le pide al núcleo: su instalación y sus carpetas. El núcleo lo busca; no es una ruta. */
export interface PersonajePedido {
  raiz: string;
  version: string;
  cuenta: string;
  reino: string;
  personaje: string;
}

/** Lo que tiene la carpeta de un personaje, para confirmar antes de eliminarla. */
export interface ResumenEliminacion {
  ruta: string;
  ficheros: number;
  bytes: number;
}

export function pedirResumenEliminacion(personaje: PersonajePedido): Promise<ResumenEliminacion> {
  return invoke<ResumenEliminacion>("resumen_eliminacion", { personaje });
}

/** Saca la carpeta del personaje del `WTF` y guarda una copia como operación (spec 021). Devuelve su id. */
export function eliminarPersonaje(personaje: PersonajePedido): Promise<string> {
  return invoke<string>("eliminar_personaje", { personaje });
}

/** Categorías de un `.wowprofile` que exporta la entrega 1 (`docs/formato-wowprofile.md`). */
export type Categoria =
  | "macros"
  | "teclas"
  | "teclas-clic"
  | "opciones"
  | "chat"
  | "ventanas"
  | "marcos-banda"
  | "addons-activos";

/** Una pieza que se puede exportar (`PiezaPropuesta` en `src-tauri/src/exportacion.rs`). */
export interface PiezaPropuesta {
  id: string;
  categoria: Categoria;
  /** Dónde estaba en el origen. */
  ambito: "personaje" | "cuenta";
  bytes: number;
  /** Líneas que se quitan al filtrar (estado, addons no instalados). */
  quitadas: number;
}

/** Lo que se puede exportar del personaje ahora mismo. */
export interface PropuestaExportacion {
  piezas: PiezaPropuesta[];
  /** Ids de las piezas que ya traen el texto reservado `@@WOWPROFILE:`: no viajan. */
  apartadas: string[];
  juegoAbierto: boolean;
}

export function pedirPropuestaExportacion(personaje: PersonajePedido): Promise<PropuestaExportacion> {
  return invoke<PropuestaExportacion>("propuesta_exportacion", { personaje });
}

/** Qué pasó al exportar: el usuario cerró el diálogo de guardar, o el nombre del fichero guardado. */
export type Exportacion = { resultado: "cancelada" } | { resultado: "exportada"; fichero: string };

/** Arma el `.wowprofile` y abre el diálogo de guardar desde el núcleo: la interfaz no elige rutas. */
export function exportarPersonaje(
  personaje: PersonajePedido,
  seleccion: string[],
  nombre: string,
  descripcion: string,
): Promise<Exportacion> {
  return invoke<Exportacion>("exportar_personaje", { personaje, seleccion, nombre, descripcion });
}

/** `manifest.json` de un paquete abierto (`ManifiestoLeido` en `src-tauri/src/paquetes/lectura.rs`). */
export interface ManifiestoPaquete {
  formato: { nombre: string; version: string };
  perfil: { nombre: string; descripcion?: string; creado: string };
  origen: { producto: string; build: string; interfaz: number; clase?: string };
  addons: { id: string; titulo: string; version?: string; enlace?: string; dependencias: string[] }[];
}

/** Por qué una entrada no se importa: aún no se sabe, o su destino no es el fichero de su categoría. */
export type MotivoIncompatible = "noSeImporta" | "destinoAjeno";

/** Una entrada del paquete abierto (`EntradaResumen` en `src-tauri/src/importacion.rs`). */
export interface EntradaPaquete {
  id: string;
  /** Texto: un paquete de una versión más nueva puede traer categorías que esta app no conoce. */
  categoria: string;
  ambito: "personaje" | "cuenta";
  /** Se escribe en la carpeta de la cuenta: afecta a todos sus personajes y hay que decidirla en cada cuenta. */
  deCuenta: boolean;
  bytes: number;
  incompatible?: MotivoIncompatible;
}

export interface ResumenPaquete {
  /** Identifica el paquete abierto ante el núcleo; nunca se usa una ruta. */
  sha256: string;
  manifiesto: ManifiestoPaquete;
  entradas: EntradaPaquete[];
}

export type Apertura = { resultado: "cancelada" } | { resultado: "abierta"; resumen: ResumenPaquete };

/** Abre el diálogo de abrir desde el núcleo y lee el paquete con todas las comprobaciones del formato. */
export function abrirPaquete(): Promise<Apertura> {
  return invoke<Apertura>("abrir_paquete");
}

export function cerrarPaquete(): Promise<void> {
  return invoke("cerrar_paquete");
}

/** Qué decide el jugador con una entrada de cuenta en una cuenta concreta. */
export interface DecisionCuenta {
  cuenta: string;
  entrada: string;
  aceptar: boolean;
}

export interface PedidoImportacion {
  seleccion: string[];
  cuentas: DecisionCuenta[];
}

export type Cambio = "anadido" | "cambiado" | "quitado";

/** Una macro, tecla, opción o addon que cambia. */
export interface Elemento {
  nombre: string;
  cambio: Cambio;
  antes?: string;
  despues?: string;
}

export type Grupo = "nuevo" | "modificado" | "sinCambios";

export interface FicheroPlan {
  entrada: string;
  categoria: Categoria;
  /** Ruta lógica (`personaje/…` o `cuenta/…`). */
  destino: string;
  deCuenta: boolean;
  grupo: Grupo;
  accion: "crear" | "sobrescribir" | "fusionar";
  elementos: Elemento[];
  /** Opciones que no se importan, o addons activados que no están instalados en el destino. */
  apartadas: string[];
  bytesAntes?: number;
  bytesDespues: number;
  /** Lo de cuenta que ya escribe otro personaje de la misma cuenta (su carpeta). */
  escritaEn?: string;
}

export interface PlanPersonaje {
  cuenta: string;
  reino: string;
  personaje: string;
  ficheros: FicheroPlan[];
  /** Entradas de cuenta sin decidir. */
  conflictos: string[];
}

export interface VistaPrevia {
  plan: {
    personajes: PlanPersonaje[];
    incompatibles: { entrada: string; categoria: string; motivo: MotivoIncompatible }[];
  };
  juegoAbierto: boolean;
  interfazDestino?: number;
  /** Se devuelve tal cual al aplicar: si el plan ya no es este, el núcleo no escribe nada. */
  huella: string;
}

/** El plan de importar el paquete abierto en esos personajes. Solo lee. */
export function pedirPlanImportacion(
  sha256: string,
  personajes: PersonajePedido[],
  pedido: PedidoImportacion,
): Promise<VistaPrevia> {
  return invoke<VistaPrevia>("plan_importacion", { sha256, personajes, pedido });
}

/** Qué pasó en un personaje al aplicar (`ResultadoPersonaje` en `src-tauri/src/importacion/aplicar.rs`). */
export type ResultadoImportacion = { cuenta: string; reino: string; personaje: string } & (
  | { resultado: "completa"; operacion: string; ficheros: number }
  | { resultado: "sinCambios" }
  | { resultado: "fallida"; error: ErrorNucleo }
  | { resultado: "noAplicada" }
);

/**
 * Aplica el paquete abierto en esos personajes, una operación por personaje, si el plan sigue siendo el de `huella`.
 * Escribe en el WTF.
 */
export function aplicarImportacion(
  sha256: string,
  huella: string,
  personajes: PersonajePedido[],
  pedido: PedidoImportacion,
): Promise<ResultadoImportacion[]> {
  return invoke<ResultadoImportacion[]>("aplicar_importacion", { sha256, huella, personajes, pedido });
}

/** Qué hizo el núcleo al arrancar con las operaciones que se cortaron (`Recuperacion` en `operaciones.rs`). */
export interface Recuperacion {
  canceladas: number;
  restauradas: number;
  sinRestaurar: number;
  pendientes: number;
  /** Importaciones cortadas a mitad: no se restauran solas y bloquean las demás operaciones. */
  aMedias: number;
  ilegibles: number;
}

export function recuperarOperaciones(): Promise<Recuperacion> {
  return invoke<Recuperacion>("recuperar_operaciones");
}

/** Tipos y estados de `operacion.json` (`docs/operacion.schema.json`). */
export type TipoOperacion = "importacion" | "deshacer" | "eliminacion";
export type EstadoOperacion =
  | "preparando"
  | "copiada"
  | "escribiendo"
  | "completa"
  | "deshecha"
  | "cancelada"
  | "fallida-restaurada"
  | "fallida-sin-restaurar";

/** Dónde escribió una operación. Las rutas no salen del equipo. */
export interface DestinoOperacion {
  raizVersion: string;
  producto: string;
  cuenta: string;
  carpetaReino: string;
  carpetaPersonaje: string;
  nombreEnDatos?: string;
}

/** Una operación del historial (`Resumen` en `src-tauri/src/operaciones/historial.rs`). */
export interface Operacion {
  id: string;
  tipo: TipoOperacion;
  creada: string;
  terminada: string | null;
  estado: EstadoOperacion;
  error: string | null;
  destino: DestinoOperacion;
  /** En una importación, el perfil importado. */
  paquete: { nombre: string; sha256: string } | null;
  deshace: string | null;
  deshechaPor: string | null;
  /** Un `deshacer` cuya original ya se borró. */
  deshaceBorrada: boolean;
  ficheros: number;
  bytes: number;
  deshacible: boolean;
  /** Una importación cortada a mitad: se ofrece «Restaurar». */
  aMedias: boolean;
  /**
   * En una `fallida-sin-restaurar` o una importación a medias: lo que hoy no está como antes y dónde está su copia
   * (`null`: no existía).
   */
  sinDevolver: { ruta: string; copia: string | null }[];
}

export interface Historial {
  /** De la más nueva a la más antigua. */
  operaciones: Operacion[];
  bytes: number;
  /** Carpetas cuyo diario no se entiende: no se tocan. */
  ilegibles: { id: string; bytes: number }[];
}

export function pedirHistorial(): Promise<Historial> {
  return invoke<Historial>("listar_operaciones");
}

/** Qué haría deshacer una operación. `huella` se devuelve tal cual al deshacer. */
export interface PlanDeshacer {
  /** Ficheros que vuelven desde la copia. */
  vuelven: number;
  /** Ficheros que hay ahora y antes no estaban: se quitan. */
  sobran: number;
  /** Ficheros que existen ahora y se pisan o se quitan. */
  cambian: string[];
  /** Los de la cuenta: cambian para todos sus personajes. */
  deCuenta: string[];
  huella: string;
}

export function pedirPlanDeshacer(id: string): Promise<PlanDeshacer> {
  return invoke<PlanDeshacer>("plan_deshacer", { id });
}

/** Deshace si el disco sigue como en el plan. Devuelve el id de la operación de deshacer. */
export function deshacerOperacion(id: string, huella: string): Promise<string> {
  return invoke<string>("deshacer_operacion", { id, huella });
}

/** `restauradaAMano`: el usuario confirma que ya devolvió a mano lo que faltaba (solo cuenta en una sin restaurar). */
/** Restaura una importación que se cortó a mitad. Escribe en el WTF. */
export function restaurarOperacion(id: string): Promise<void> {
  return invoke("restaurar_operacion", { id });
}

export function borrarOperacion(id: string, restauradaAMano: boolean): Promise<void> {
  return invoke("borrar_operacion", { id, restauradaAMano });
}

export function abrirCarpetaOperacion(id: string): Promise<void> {
  return invoke("abrir_carpeta_operacion", { id });
}

/** Enlaces que sabe abrir el núcleo (`src-tauri/src/enlaces.rs`). La interfaz nunca le pasa una URL. */
export type Enlace = "repositorio" | "koFi" | "licencia";

export function abrirEnlace(enlace: Enlace): Promise<void> {
  return invoke("abrir_enlace", { enlace });
}

/** Idioma de la interfaz de Windows. Si el núcleo no responde, inglés: la app tiene que poder arrancar igual. */
export function pedirIdioma(): Promise<Idioma> {
  return invoke<Idioma>("idioma_interfaz").catch((): Idioma => "en");
}
