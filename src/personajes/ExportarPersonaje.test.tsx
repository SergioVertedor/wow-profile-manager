import { afterEach, beforeAll, describe, expect, it } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { I18nextProvider } from "react-i18next";
import type { PersonajePedido, PropuestaExportacion } from "../nucleo";
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

const PROPUESTA: PropuestaExportacion = {
  piezas: [
    { id: "macros", categoria: "macros", ambito: "personaje", bytes: 2_000, quitadas: 0 },
    { id: "teclas", categoria: "teclas", ambito: "cuenta", bytes: 1_600, quitadas: 0 },
    { id: "addons-activos", categoria: "addons-activos", ambito: "personaje", bytes: 400, quitadas: 12 },
  ],
  apartadas: [],
  juegoAbierto: false,
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

  it("si no se puede pedir qué exportar, lo dice junto al botón", async () => {
    pintar({ propuesta_exportacion: () => Promise.reject({ codigo: "personajeNoEncontrado", datos: {} }) });

    fireEvent.click(screen.getByRole("button", { name: "Exportar" }));

    expect((await screen.findByRole("alert")).textContent).toMatch(/ya no está en disco/);
  });
});
