// Escaparate (solo desarrollo, `escaparate.html`): la app real con el núcleo simulado y datos ficticios, para sacar
// las capturas del README sin publicar datos de nadie. `npm run build` no lo incluye: solo construye `index.html`.
// Idioma con `?idioma=es`; inglés por defecto, como el README.
import React from "react";
import ReactDOM from "react-dom/client";
import { I18nextProvider } from "react-i18next";
import { mockIPC } from "@tauri-apps/api/mocks";
import App from "../App";
import "../estilo/base.css";
import type { Apertura, Exportacion, Recuperacion } from "../nucleo";
import { crearTextos, type Idioma } from "../textos";
import { DETECCION, DIAGNOSTICO, HISTORIAL, INFO, PAQUETE, PROPUESTA, VISTA_PREVIA } from "./datos";

const idioma: Idioma = new URLSearchParams(location.search).get("idioma") === "es" ? "es" : "en";

const SIN_RECUPERAR: Recuperacion = {
  canceladas: 0,
  restauradas: 0,
  sinRestaurar: 0,
  pendientes: 0,
  aMedias: 0,
  ilegibles: 0,
};
const ABIERTA: Apertura = { resultado: "abierta", resumen: PAQUETE };
const CANCELADA: Exportacion = { resultado: "cancelada" };

/** Respuesta de cada comando que devuelve datos. */
const RESPUESTAS: Record<string, unknown> = {
  info_app: INFO,
  idioma_interfaz: idioma,
  detectar_instalaciones: DETECCION,
  recuperar_operaciones: SIN_RECUPERAR,
  propuesta_exportacion: PROPUESTA,
  exportar_personaje: CANCELADA,
  abrir_paquete: ABIERTA,
  plan_importacion: VISTA_PREVIA,
  listar_operaciones: HISTORIAL,
  estado_diagnostico: DIAGNOSTICO,
};

/** Comandos que no devuelven nada: en el escaparate no hacen nada, porque solo enseña. */
const SIN_RESPUESTA = new Set([
  "cerrar_paquete",
  "abrir_enlace",
  "abrir_carpeta_personaje",
  "abrir_carpeta_operacion",
  "abrir_carpeta_registros",
  "cambiar_registro",
  "cerrar_ventana",
]);

/** Un comando sin datos falla con su nombre: así se ve qué falta en vez de pintar vacío. */
function responder(comando: string): unknown {
  if (comando in RESPUESTAS) return RESPUESTAS[comando];
  if (SIN_RESPUESTA.has(comando)) return undefined;
  throw new Error(`escaparate: el comando «${comando}» no tiene datos simulados`);
}

mockIPC(responder, { shouldMockEvents: true });
document.documentElement.lang = idioma;

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <I18nextProvider i18n={crearTextos(idioma)}>
      <App />
    </I18nextProvider>
  </React.StrictMode>,
);
