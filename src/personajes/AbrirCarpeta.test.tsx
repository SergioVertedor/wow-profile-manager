import { afterEach, describe, expect, it } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { I18nextProvider } from "react-i18next";
import type { PersonajePedido } from "../nucleo";
import { crearTextos } from "../textos";
import AbrirCarpeta from "./AbrirCarpeta";

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

function pintar(respuesta: () => unknown) {
  const llamadas: { comando: string; argumentos: unknown }[] = [];
  mockIPC((comando, argumentos) => {
    llamadas.push({ comando, argumentos });
    return respuesta();
  });
  render(
    <I18nextProvider i18n={crearTextos("es")}>
      <AbrirCarpeta pedido={PEDIDO} />
    </I18nextProvider>,
  );
  return llamadas;
}

describe("AbrirCarpeta", () => {
  it("pide al núcleo abrir la carpeta del personaje, sin pasarle una ruta", async () => {
    const llamadas = pintar(() => undefined);

    fireEvent.click(screen.getByRole("button", { name: "Abrir carpeta" }));

    await waitFor(() =>
      expect(llamadas).toEqual([
        {
          comando: "abrir_carpeta_personaje",
          argumentos: { personaje: PEDIDO },
        },
      ]),
    );
    expect(screen.queryByRole("alert")).toBeNull();
  });

  it("si no se puede abrir, lo dice", async () => {
    pintar(() => Promise.reject({ codigo: "personajeNoEncontrado", datos: {} }));

    fireEvent.click(screen.getByRole("button", { name: "Abrir carpeta" }));

    expect((await screen.findByRole("alert")).textContent).toBe(
      "Ese personaje ya no está en disco. Vuelve a cargar la lista.",
    );
  });
});
