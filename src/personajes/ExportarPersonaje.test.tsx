import { afterEach, beforeAll, describe, expect, it } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { I18nextProvider } from "react-i18next";
import type { PersonajePedido, PiezaPropuesta, PropuestaExportacion } from "../nucleo";
import { crearTextos } from "../textos";
import ExportarPersonaje from "./ExportarPersonaje";

// jsdom no implementa el diálogo modal: lo justo para abrirlo y cerrarlo (con su evento `close`, como el navegador).
beforeAll(() => {
  HTMLDialogElement.prototype.showModal = function (this: HTMLDialogElement) {
    this.open = true;
  };
  HTMLDialogElement.prototype.close = function (this: HTMLDialogElement) {
    this.open = false;
    this.dispatchEvent(new Event("close"));
  };
});

afterEach(() => {
  cleanup();
  clearMocks();
});

const PEDIDO: PersonajePedido = {
  raiz: "C:\\Juegos\\World of Warcraft",
  version: "_forever_",
  cuenta: "107710745#1",
  reino: "93",
  personaje: "Goldberg-Joestar",
};

/** Una pieza del personaje o de la cuenta: marcada y sin datos de addon. */
function pieza(campos: Pick<PiezaPropuesta, "id" | "categoria"> & Partial<PiezaPropuesta>): PiezaPropuesta {
  return {
    tipo: "fichero",
    ambito: "personaje",
    bytes: 100,
    quitadas: [],
    marcada: true,
    addon: null,
    variable: null,
    perfil: null,
    regla: null,
    versionNoMedida: false,
    pistas: [],
    ...campos,
  };
}

/** Una pieza de configuración de addon. */
function deAddon(campos: Pick<PiezaPropuesta, "id" | "addon" | "regla"> & Partial<PiezaPropuesta>): PiezaPropuesta {
  return pieza({ categoria: "config-addon", marcada: false, ambito: "cuenta", ...campos });
}

const PROPUESTA: PropuestaExportacion = {
  piezas: [
    pieza({ id: "macros", categoria: "macros", bytes: 2_000 }),
    pieza({ id: "teclas", categoria: "teclas", ambito: "cuenta", bytes: 1_600 }),
    pieza({
      id: "addons-activos",
      categoria: "addons-activos",
      tipo: "fichero-filtrado",
      bytes: 400,
      quitadas: [{ motivo: "addonNoInstalado", claves: Array.from({ length: 12 }, (_, i) => `Addon${i}`) }],
    }),
  ],
  apartadas: [],
  noExportables: [],
  carpetasNoExportables: [],
  juegoAbierto: false,
};

const CON_ADDONS: PropuestaExportacion = {
  ...PROPUESTA,
  piezas: [
    pieza({ id: "macros", categoria: "macros" }),
    deAddon({ id: "dominos-perfil", addon: "Dominos", regla: "perfilAcedb", perfil: "Default - Druid", marcada: true }),
    deAddon({ id: "dominos-cuenta", addon: "Dominos", regla: "genericoCuenta" }),
    deAddon({ id: "wim-data", addon: "WIM", regla: "adaptador", variable: "WIM3_Data", versionNoMedida: true }),
    deAddon({ id: "prat-pj", addon: "Prat-3.0", regla: "genericoPersonaje", ambito: "personaje", pistas: ["guid"] }),
    deAddon({ id: "nodes-pj", addon: "NodeSight", regla: "genericoPersonaje", ambito: "personaje" }),
  ],
  noExportables: [{ addon: "Spy", ambito: "cuenta", variable: "SpyDB", motivo: "sinPerfil" }],
};

function pintar(respuestas: Record<string, () => unknown>) {
  const llamadas: { comando: string; argumentos: unknown }[] = [];
  mockIPC((comando, argumentos) => {
    llamadas.push({ comando, argumentos });
    return respuestas[comando]?.();
  });
  render(
    <I18nextProvider i18n={crearTextos("es")}>
      <ExportarPersonaje pedido={PEDIDO} nombre="Goldberg Joestar" donde={["Cuenta 1", "Instalación 1"]} />
    </I18nextProvider>,
  );
  return llamadas;
}

async function abrir() {
  fireEvent.click(screen.getByRole("button", { name: "Exportar" }));
  return screen.findByRole("dialog");
}

describe("ExportarPersonaje", () => {
  it("enseña las piezas marcadas, de dónde vienen, lo que se quita y que las barras no viajan", async () => {
    pintar({ propuesta_exportacion: () => PROPUESTA });

    const dialogo = await abrir();

    const casillas = within(dialogo).getAllByRole("checkbox");
    expect(casillas.map((c) => (c as HTMLInputElement).checked)).toEqual([true, true, true]);
    expect(within(dialogo).getByText("de la cuenta", { exact: false })).toBeTruthy();
    expect(within(dialogo).getByText(/sin 12 líneas que no viajan/)).toBeTruthy();
    expect(within(dialogo).getByText(/no viaja: lo guarda el servidor/)).toBeTruthy();
    expect(within(dialogo).queryByText(/WoW está abierto/)).toBeNull();
  });

  it("el Modo Edición dice qué diseño viaja, no que viene de la cuenta", async () => {
    pintar({
      propuesta_exportacion: () => ({
        ...PROPUESTA,
        piezas: [
          pieza({ id: "modo-edicion", categoria: "modo-edicion", tipo: "adaptador", ambito: "cuenta", perfil: "Sanador" }),
        ],
      }),
    });

    const dialogo = await abrir();

    expect(within(dialogo).getByText("diseño «Sanador»", { exact: false })).toBeTruthy();
    expect(within(dialogo).queryByText("de la cuenta", { exact: false })).toBeNull();
  });

  it("lo que no viaja se despliega agrupado por motivo, con su frase y sus claves", async () => {
    const quitadas = [
      { motivo: "seguridad" as const, claves: ["AllowDangerousScripts"] },
      { motivo: "estado" as const, claves: ["trackedQuests", "EJLootClass"] },
    ];
    pintar({
      propuesta_exportacion: () => ({
        ...PROPUESTA,
        piezas: [pieza({ id: "opciones", categoria: "opciones", tipo: "fichero-filtrado", bytes: 900, quitadas })],
      }),
    });

    const dialogo = await abrir();

    fireEvent.click(within(dialogo).getByText("sin 3 líneas que no viajan"));
    expect(within(dialogo).getByText(/^Por seguridad:/)).toBeTruthy();
    expect(within(dialogo).getByText(/^Son estado de la partida/)).toBeTruthy();
    expect(within(dialogo).getByText("AllowDangerousScripts")).toBeTruthy();
    expect(within(dialogo).getByText("EJLootClass")).toBeTruthy();
    // El desplegable no está dentro de la casilla: abrirlo no la desmarca.
    expect((within(dialogo).getByRole("checkbox") as HTMLInputElement).checked).toBe(true);
  });

  it("sin nombre no se puede guardar; con él manda lo marcado en el orden de la propuesta", async () => {
    const llamadas = pintar({
      propuesta_exportacion: () => PROPUESTA,
      exportar_personaje: () => ({ resultado: "exportada", fichero: "Druida.wowprofile" }),
    });
    const dialogo = await abrir();
    const guardar = within(dialogo).getByRole("button", { name: "Guardar…" });
    expect((guardar as HTMLButtonElement).disabled).toBe(true);

    fireEvent.click(within(dialogo).getByRole("checkbox", { name: /Macros/ }));
    fireEvent.change(within(dialogo).getByRole("textbox", { name: "Nombre del perfil" }), {
      target: { value: "Druida" },
    });
    fireEvent.click(guardar);

    expect(await within(dialogo).findByText("Guardado como Druida.wowprofile.")).toBeTruthy();
    expect(llamadas[llamadas.length - 1]).toEqual({
      comando: "exportar_personaje",
      argumentos: { personaje: PEDIDO, seleccion: ["teclas", "addons-activos"], nombre: "Druida", descripcion: "" },
    });
  });

  it("si se cancela el diálogo de guardar, sigue eligiendo; un error se enseña en el diálogo", async () => {
    let respuesta: () => unknown = () => ({ resultado: "cancelada" });
    pintar({ propuesta_exportacion: () => PROPUESTA, exportar_personaje: () => respuesta() });
    const dialogo = await abrir();
    fireEvent.change(within(dialogo).getByRole("textbox", { name: "Nombre del perfil" }), {
      target: { value: "Druida" },
    });
    const guardar = within(dialogo).getByRole("button", { name: "Guardar…" });

    fireEvent.click(guardar);
    await waitFor(() => expect((guardar as HTMLButtonElement).disabled).toBe(false));
    expect(within(dialogo).queryByRole("alert")).toBeNull();
    expect(within(dialogo).getAllByRole("checkbox")).toHaveLength(3);

    respuesta = () => Promise.reject({ codigo: "exportacionSinVersion", datos: {} });
    fireEvent.click(guardar);
    expect(await within(dialogo).findByRole("alert")).toBeTruthy();
    expect(within(dialogo).getByRole("alert").textContent).toMatch(/versión del juego/);
  });

  it("avisa si el juego está abierto y de las piezas apartadas por el marcador", async () => {
    pintar({ propuesta_exportacion: () => ({ ...PROPUESTA, juegoAbierto: true, apartadas: ["macros"] }) });

    const dialogo = await abrir();

    expect(within(dialogo).getByText(/WoW está abierto/)).toBeTruthy();
    expect(within(dialogo).getByText(/«@@WOWPROFILE:»: macros\./)).toBeTruthy();
  });

  it("la configuración de addons sale por addon, marcada solo si se reconoce, y con sus avisos", async () => {
    pintar({ propuesta_exportacion: () => CON_ADDONS });

    const dialogo = await abrir();

    const marcada = (nombre: RegExp) => (within(dialogo).getByRole("checkbox", { name: nombre }) as HTMLInputElement).checked;
    expect(marcada(/Perfil «Default - Druid»/)).toBe(true);
    expect(marcada(/Fichero de la cuenta/)).toBe(false);
    expect(marcada(/Ajustes: WIM3_Data/)).toBe(false);
    const dominos = within(dialogo).getByRole("group", { name: "Dominos" });
    expect(within(dominos).getAllByRole("checkbox")).toHaveLength(2);
    expect(within(dominos).getByText(/Fichero entero: puede llevar progreso/)).toBeTruthy();
    const prat = within(dialogo).getByRole("group", { name: "Prat-3.0" });
    expect(within(prat).getByText(/identificadores de jugador/)).toBeTruthy();
    // El aviso va ligado a su casilla, para quien usa lector de pantalla.
    const casilla = within(prat).getByRole("checkbox");
    expect(document.getElementById(casilla.getAttribute("aria-describedby") ?? "")?.textContent).toMatch(/jugador/);
    expect(within(dialogo).getByText(/no se ha comprobado/)).toBeTruthy();
  });

  it("«marcar» marca las genéricas de su regla salvo las que tienen pistas, y se mandan en orden", async () => {
    const llamadas = pintar({
      propuesta_exportacion: () => CON_ADDONS,
      exportar_personaje: () => ({ resultado: "exportada", fichero: "Druida.wowprofile" }),
    });
    const dialogo = await abrir();

    fireEvent.click(within(dialogo).getByRole("button", { name: "Marcar los ficheros del personaje" }));
    fireEvent.change(within(dialogo).getByRole("textbox", { name: "Nombre del perfil" }), {
      target: { value: "Druida" },
    });
    fireEvent.click(within(dialogo).getByRole("button", { name: "Guardar…" }));

    await within(dialogo).findByText("Guardado como Druida.wowprofile.");
    expect(llamadas[llamadas.length - 1].argumentos).toMatchObject({
      seleccion: ["macros", "dominos-perfil", "nodes-pj"],
    });
  });

  it("las carpetas de addons salen marcadas con el aviso de riesgos y licencia, y se pueden desmarcar todas", async () => {
    const carpeta = (addon: string) =>
      pieza({ id: `carpeta-${addon.toLowerCase()}`, categoria: "addon", tipo: "carpeta", ambito: "instalacion", addon });
    const llamadas = pintar({
      propuesta_exportacion: () => ({
        ...PROPUESTA,
        piezas: [pieza({ id: "macros", categoria: "macros" }), carpeta("Dominos"), carpeta("Details")],
        carpetasNoExportables: [{ addon: "Enorme", motivo: { codigo: "demasiadoGrande" } }],
      }),
      exportar_personaje: () => ({ resultado: "exportada", fichero: "Druida.wowprofile" }),
    });
    const dialogo = await abrir();

    const addons = within(dialogo).getByRole("region", { name: "Addons" });
    const casilla = within(addons).getByRole("checkbox", { name: /Dominos/ });
    expect((casilla as HTMLInputElement).checked).toBe(true);
    const aviso = document.getElementById(casilla.getAttribute("aria-describedby") ?? "")?.textContent;
    expect(aviso).toMatch(/El juego lo ejecuta/);
    expect(aviso).toMatch(/redistribuyan/);
    fireEvent.click(within(addons).getByText(/1 addon no se puede llevar/));
    expect(within(addons).getByText("Enorme: ocupa más de 64 MiB.")).toBeTruthy();

    fireEvent.click(within(addons).getByRole("button", { name: "Desmarcar todos" }));
    fireEvent.change(within(dialogo).getByRole("textbox", { name: "Nombre del perfil" }), {
      target: { value: "Druida" },
    });
    fireEvent.click(within(dialogo).getByRole("button", { name: "Guardar…" }));

    await within(dialogo).findByText("Guardado como Druida.wowprofile.");
    expect(llamadas[llamadas.length - 1].argumentos).toMatchObject({ seleccion: ["macros"] });
  });

  it("dice qué no se puede exportar de cada addon y por qué", async () => {
    pintar({ propuesta_exportacion: () => CON_ADDONS });
    const dialogo = await abrir();

    fireEvent.click(within(dialogo).getByText("1 cosa no se puede exportar"));

    expect(within(dialogo).getByText(/Spy · SpyDB \(de la cuenta\): Este personaje no tiene perfil asignado/)).toBeTruthy();
  });

  it("sin piezas también dice qué no se puede exportar", async () => {
    pintar({ propuesta_exportacion: () => ({ ...CON_ADDONS, piezas: [] }) });
    const dialogo = await abrir();

    expect(within(dialogo).getByText(/no tiene nada que exportar/)).toBeTruthy();
    expect(within(dialogo).getByText("1 cosa no se puede exportar")).toBeTruthy();
  });

  it("si no se puede pedir qué exportar, lo dice junto al botón", async () => {
    pintar({ propuesta_exportacion: () => Promise.reject({ codigo: "personajeNoEncontrado", datos: {} }) });

    fireEvent.click(screen.getByRole("button", { name: "Exportar" }));

    expect((await screen.findByRole("alert")).textContent).toMatch(/ya no está en disco/);
  });
});
