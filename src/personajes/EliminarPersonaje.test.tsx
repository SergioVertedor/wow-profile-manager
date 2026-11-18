import { afterEach, beforeAll, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { I18nextProvider } from "react-i18next";
import type { PersonajePedido } from "../nucleo";
import { crearTextos } from "../textos";
import EliminarPersonaje from "./EliminarPersonaje";

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
  personaje: "Goldberg-Soulreaper",
};

const RESUMEN = {
  ruta: "C:\\Juegos\\World of Warcraft\\_forever_\\WTF\\Account\\107710745#1\\93\\Goldberg-Soulreaper",
  ficheros: 54,
  bytes: 120_000,
};

function pintar(respuestas: Record<string, () => unknown>) {
  const llamadas: { comando: string; argumentos: unknown }[] = [];
  mockIPC((comando, argumentos) => {
    llamadas.push({ comando, argumentos });
    return respuestas[comando]?.();
  });
  const alTerminar = vi.fn(() => Promise.resolve());
  render(
    <I18nextProvider i18n={crearTextos("es")}>
      <EliminarPersonaje
        pedido={PEDIDO}
        nombre="Goldberg Soulreaper"
        donde={["Cuenta 1", "Instalación 1"]}
        alTerminar={alTerminar}
      />
    </I18nextProvider>,
  );
  return { llamadas, alTerminar };
}

describe("EliminarPersonaje", () => {
  it("enseña lo que hay en la carpeta y el aviso antes de confirmar", async () => {
    pintar({ resumen_eliminacion: () => RESUMEN });

    fireEvent.click(screen.getByRole("button", { name: "Eliminar carpeta" }));

    expect(await screen.findByText(RESUMEN.ruta)).toBeTruthy();
    expect(screen.getByRole("heading", { name: "¿Eliminar la carpeta de Goldberg Soulreaper?" })).toBeTruthy();
    expect(screen.getByText("Cuenta 1 · Instalación 1")).toBeTruthy();
    expect(screen.getByText(/^54 ficheros, 120\skB\.$/)).toBeTruthy();
    expect(screen.getByText(/el juego volverá a crear la carpeta/)).toBeTruthy();
  });

  it("al confirmar pide eliminar ese personaje y, al cerrar, vuelve a leer la lista", async () => {
    const { llamadas, alTerminar } = pintar({
      resumen_eliminacion: () => RESUMEN,
      eliminar_personaje: () => "20261118-093549-0000abcd",
    });
    fireEvent.click(screen.getByRole("button", { name: "Eliminar carpeta" }));
    await screen.findByText(RESUMEN.ruta);

    const dialogo = screen.getByRole("dialog");
    fireEvent.click(within(dialogo).getByRole("button", { name: "Eliminar carpeta" }));

    expect(await screen.findByRole("status")).toHaveProperty(
      "textContent",
      "Carpeta eliminada. La copia está guardada en la operación 20261118-093549-0000abcd.",
    );
    expect(llamadas.find((l) => l.comando === "eliminar_personaje")?.argumentos).toEqual({ personaje: PEDIDO });
    expect(alTerminar).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole("button", { name: "Cerrar" }));
    await waitFor(() => expect(alTerminar).toHaveBeenCalledTimes(1));
  });

  it("un doble clic pide el resumen una sola vez", async () => {
    const { llamadas } = pintar({ resumen_eliminacion: () => RESUMEN });
    const boton = screen.getByRole("button", { name: "Eliminar carpeta" });

    fireEvent.click(boton);
    fireEvent.click(boton);

    await screen.findByText(RESUMEN.ruta);
    expect(llamadas.filter((l) => l.comando === "resumen_eliminacion")).toHaveLength(1);
  });

  it("cancelar no elimina nada ni vuelve a leer", async () => {
    const { llamadas, alTerminar } = pintar({ resumen_eliminacion: () => RESUMEN });
    fireEvent.click(screen.getByRole("button", { name: "Eliminar carpeta" }));
    await screen.findByText(RESUMEN.ruta);

    fireEvent.click(screen.getByRole("button", { name: "Cancelar" }));

    expect(llamadas.some((l) => l.comando === "eliminar_personaje")).toBe(false);
    expect(alTerminar).not.toHaveBeenCalled();
    expect(screen.queryByText(RESUMEN.ruta)).toBeNull();
  });

  it("con el juego abierto lo dice dentro del diálogo", async () => {
    pintar({ resumen_eliminacion: () => RESUMEN, eliminar_personaje: errorDelNucleo("juegoAbierto") });
    fireEvent.click(screen.getByRole("button", { name: "Eliminar carpeta" }));
    await screen.findByText(RESUMEN.ruta);

    fireEvent.click(within(screen.getByRole("dialog")).getByRole("button", { name: "Eliminar carpeta" }));

    expect((await screen.findByRole("alert")).textContent).toMatch(/^WoW está abierto/);
  });

  it("dice si una operación fallida dejó todo como estaba", async () => {
    pintar({
      resumen_eliminacion: () => RESUMEN,
      eliminar_personaje: errorDelNucleo("operacionFallida", { id: "x", restaurada: true, detalle: "bloqueado" }),
    });
    fireEvent.click(screen.getByRole("button", { name: "Eliminar carpeta" }));
    await screen.findByText(RESUMEN.ruta);

    fireEvent.click(within(screen.getByRole("dialog")).getByRole("button", { name: "Eliminar carpeta" }));

    expect((await screen.findByRole("alert")).textContent).toBe(
      "No se pudo terminar y se ha dejado todo como estaba (operación x): bloqueado",
    );
  });

  it("si el personaje ya no está, lo dice sin abrir el diálogo", async () => {
    pintar({ resumen_eliminacion: errorDelNucleo("personajeNoEncontrado") });

    fireEvent.click(screen.getByRole("button", { name: "Eliminar carpeta" }));

    expect((await screen.findByRole("alert")).textContent).toBe(
      "Ese personaje ya no está en disco. Vuelve a cargar la lista.",
    );
    expect(screen.queryByText(RESUMEN.ruta)).toBeNull();
  });
});

/** Un error del núcleo tal como lo rechaza `invoke`: el objeto `{ codigo, datos }`. */
function errorDelNucleo(codigo: string, datos: object = {}) {
  return () => Promise.reject({ codigo, datos });
}
