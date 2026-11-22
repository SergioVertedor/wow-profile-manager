// Capturas del README (#83). Abre el escaparate (`escaparate.html`: la app real con el núcleo simulado y
// datos ficticios) en Edge sin ventana, recorre las vistas pulsando en el DOM y guarda PNG en `docs/capturas/`.
//
// Uso, con el servidor de Vite en marcha (`npm run dev` o `npm run tauri dev`):
//   node scripts/capturas.mjs
// `NAVEGADOR` cambia el ejecutable: cualquier Chromium con el protocolo DevTools (Chrome, Edge, Brave).
import { spawn } from "node:child_process";
import { mkdtempSync, rmSync, writeFileSync, mkdirSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import process from "node:process";
import { setTimeout as esperar } from "node:timers/promises";

const NAVEGADOR = process.env.NAVEGADOR ?? String.raw`C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe`;
const URL_ESCAPARATE = "http://localhost:1420/escaparate.html";
const PUERTO = 9333;
const SALIDA = join(import.meta.dirname, "..", "docs", "capturas");
/** Algo más que la ventana por defecto (1024×720), para que quepa el pie de cada vista; a 2× para pantallas densas. */
const ANCHO = 1100;
const ALTO = 780;
const ESCALA = 2;
const ESPERA_MAX_MS = 10_000;
/** Cada cuánto se vuelve a mirar si algo está listo. */
const SONDEO_MS = 150;
/** Lo que tardan las transiciones de CSS de la app en asentarse antes de capturar. */
const TRANSICIONES_MS = 400;
/** Lo que tardan los procesos hijos del navegador en soltar el perfil tras cerrarse. */
const SOLTAR_PERFIL_MS = 1000;

/** Lo que se hace en la página antes de cada captura. Corre dentro del navegador. */
const AYUDANTES = `
  window.__pulsar = (texto, dentro = document) => {
    const boton = [...dentro.querySelectorAll("button")].find((b) => b.textContent.trim().startsWith(texto));
    if (!boton) throw new Error("sin botón: " + texto);
    boton.click();
  };
  window.__marcar = (texto) => {
    const casilla = [...document.querySelectorAll("input[type=checkbox]")]
      .find((c) => c.closest("li, label")?.textContent.includes(texto));
    if (!casilla) throw new Error("sin casilla: " + texto);
    casilla.click();
  };
`;

const NAV = `document.querySelector("nav")`;

const CAPTURAS = [
  { fichero: "personajes.png", pasos: [], hasta: "Aelindra" },
  { fichero: "exportar.png", pasos: [`__pulsar("Export")`], hasta: "Addon settings" },
  {
    fichero: "importar.png",
    pasos: [
      `__pulsar("Import", ${NAV})`,
      `__pulsar("Open profile")`,
      `__marcar("Brannoc")`,
      `__pulsar("Next")`,
      `__pulsar("Next")`,
    ],
    hasta: "Apply",
  },
  { fichero: "historial.png", pasos: [`__pulsar("History", ${NAV})`], hasta: "Tank essentials" },
];

/** Una conexión al protocolo DevTools de una pestaña: `enviar` espera la respuesta de cada orden. */
async function conectar(urlWs) {
  const socket = new WebSocket(urlWs);
  await new Promise((resolver, rechazar) => {
    socket.addEventListener("open", resolver, { once: true });
    socket.addEventListener("error", rechazar, { once: true });
  });
  let siguiente = 0;
  const pendientes = new Map();
  socket.addEventListener("message", (evento) => {
    const mensaje = JSON.parse(evento.data);
    const pendiente = pendientes.get(mensaje.id);
    if (!pendiente) return;
    pendientes.delete(mensaje.id);
    if (mensaje.error) pendiente.rechazar(new Error(mensaje.error.message));
    else pendiente.resolver(mensaje.result);
  });
  // Si la conexión muere, ninguna orden se queda esperando para siempre.
  const rechazarTodas = () => {
    for (const pendiente of pendientes.values()) pendiente.rechazar(new Error("se cerró la conexión con el navegador"));
    pendientes.clear();
  };
  socket.addEventListener("close", rechazarTodas);
  socket.addEventListener("error", rechazarTodas);
  const enviar = (method, params = {}) =>
    new Promise((resolver, rechazar) => {
      siguiente += 1;
      const id = siguiente;
      const plazo = setTimeout(() => {
        pendientes.delete(id);
        rechazar(new Error(`${method}: sin respuesta en ${ESPERA_MAX_MS} ms`));
      }, ESPERA_MAX_MS);
      const terminar = (accion) => (valor) => {
        clearTimeout(plazo);
        accion(valor);
      };
      pendientes.set(id, { resolver: terminar(resolver), rechazar: terminar(rechazar) });
      socket.send(JSON.stringify({ id, method, params }));
    });
  return { enviar, cerrar: () => socket.close() };
}

async function evaluar(cdp, expresion) {
  const { result, exceptionDetails } = await cdp.enviar("Runtime.evaluate", {
    expression: expresion,
    awaitPromise: true,
  });
  if (exceptionDetails) {
    throw new Error(`${expresion}: ${exceptionDetails.exception?.description ?? exceptionDetails.text}`);
  }
  return result.value;
}

/** Espera a que la página enseñe un texto: la app pinta tras responder el núcleo simulado. */
async function esperarTexto(cdp, texto) {
  const limite = Date.now() + ESPERA_MAX_MS;
  let ultimoError = null;
  while (Date.now() < limite) {
    try {
      if (await evaluar(cdp, `document.body.innerText.includes(${JSON.stringify(texto)})`)) return;
    } catch (error) {
      // Durante una navegación el contexto de la página se destruye y se crea otro: se reintenta.
      ultimoError = error;
    }
    await esperar(SONDEO_MS);
  }
  const causa = ultimoError ? ` (${ultimoError.message})` : "";
  throw new Error(`no apareció «${texto}»${causa}`);
}

async function pestanaDevTools() {
  const limite = Date.now() + ESPERA_MAX_MS;
  while (Date.now() < limite) {
    try {
      const pestanas = await (await fetch(`http://127.0.0.1:${PUERTO}/json/list`)).json();
      const pagina = pestanas.find((p) => p.type === "page");
      if (pagina) return pagina.webSocketDebuggerUrl;
    } catch {
      // El navegador todavía arranca: se reintenta hasta el límite.
    }
    await esperar(SONDEO_MS);
  }
  throw new Error("el navegador no abrió el puerto de DevTools");
}

async function capturar(cdp, { fichero, pasos, hasta }) {
  const { errorText } = await cdp.enviar("Page.navigate", { url: URL_ESCAPARATE });
  if (errorText) throw new Error(`no se pudo abrir ${URL_ESCAPARATE} (${errorText}): ¿está en marcha Vite?`);
  await esperarTexto(cdp, "Aelindra");
  await evaluar(cdp, AYUDANTES);
  for (const paso of pasos) {
    await evaluar(cdp, paso);
    await esperar(SONDEO_MS);
  }
  await esperarTexto(cdp, hasta);
  await esperar(TRANSICIONES_MS);
  const { data } = await cdp.enviar("Page.captureScreenshot", { format: "png" });
  writeFileSync(join(SALIDA, fichero), Buffer.from(data, "base64"));
  process.stdout.write(`docs/capturas/${fichero}\n`);
}

/**
 * Cierra el navegador por el protocolo, desde la conexión del navegador (la de una pestaña no admite `Browser.close`).
 * `kill` no sirve: en Windows el proceso lanzado delega en otro y termina enseguida.
 */
async function cerrarNavegador() {
  try {
    const { webSocketDebuggerUrl } = await (await fetch(`http://127.0.0.1:${PUERTO}/json/version`)).json();
    const navegador = await conectar(webSocketDebuggerUrl);
    await navegador.enviar("Browser.close").catch(() => undefined);
    navegador.cerrar();
  } catch {
    // Ya no hay navegador escuchando.
  }
  const limite = Date.now() + ESPERA_MAX_MS;
  while (await puertoOcupado()) {
    if (Date.now() >= limite) {
      process.stderr.write(`el navegador sigue escuchando en el puerto ${PUERTO}: ciérralo a mano
`);
      return;
    }
    await esperar(SONDEO_MS);
  }
  await esperar(SOLTAR_PERFIL_MS);
}

/** El perfil es temporal: si Windows aún lo tiene bloqueado, se avisa y se deja para la limpieza del sistema. */
function borrarPerfil(perfil) {
  try {
    rmSync(perfil, { recursive: true, force: true, maxRetries: 10, retryDelay: 300 });
  } catch (error) {
    process.stderr.write(`no se pudo borrar el perfil temporal ${perfil}: ${error.code ?? error}
`);
  }
}

function puertoOcupado() {
  return fetch(`http://127.0.0.1:${PUERTO}/json/version`).then(
    () => true,
    () => false,
  );
}

/** Otro navegador en el puerto haría las capturas con su propio perfil y se quedaría vivo: mejor parar. */
async function comprobarPuertoLibre() {
  if (await puertoOcupado()) throw new Error(`el puerto ${PUERTO} ya tiene un navegador con DevTools: ciérralo antes`);
}

/** Lanza el navegador; si el ejecutable no existe, el error llega como rechazo y no tumba el proceso. */
function lanzarNavegador(perfil) {
  const proceso = spawn(NAVEGADOR, [
    "--headless=new",
    `--remote-debugging-port=${PUERTO}`,
    `--user-data-dir=${perfil}`,
    "--hide-scrollbars",
    "--no-first-run",
    "about:blank",
  ]);
  return new Promise((_, rechazar) =>
    proceso.once("error", (error) => rechazar(new Error(`no se pudo lanzar ${NAVEGADOR}: ${error.message}`))),
  );
}

async function principal() {
  await comprobarPuertoLibre();
  mkdirSync(SALIDA, { recursive: true });
  const perfil = mkdtempSync(join(tmpdir(), "wpm-capturas-"));
  let cdp = null;
  try {
    const fallo = lanzarNavegador(perfil);
    cdp = await conectar(await Promise.race([pestanaDevTools(), fallo]));
    await cdp.enviar("Emulation.setDeviceMetricsOverride", {
      width: ANCHO,
      height: ALTO,
      deviceScaleFactor: ESCALA,
      mobile: false,
    });
    // Fechas y formatos iguales en cualquier equipo.
    await cdp.enviar("Emulation.setTimezoneOverride", { timezoneId: "UTC" });
    await cdp.enviar("Emulation.setLocaleOverride", { locale: "en-US" });
    await cdp.enviar("Page.enable");
    await cdp.enviar("Runtime.enable");
    for (const captura of CAPTURAS) await capturar(cdp, captura);
  } finally {
    cdp?.cerrar();
    await cerrarNavegador();
    borrarPerfil(perfil);
  }
}

await principal();
