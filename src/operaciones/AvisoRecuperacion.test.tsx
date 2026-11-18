import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { I18nextProvider } from "react-i18next";
import type { Recuperacion } from "../nucleo";
import { crearTextos } from "../textos";
import AvisoRecuperacion from "./AvisoRecuperacion";

afterEach(() => {
  cleanup();
  clearMocks();
});

const NADA: Recuperacion = { canceladas: 0, restauradas: 0, sinRestaurar: 0, pendientes: 0, ilegibles: 0 };

function pintar(recuperacion: Recuperacion) {
  const pedida = vi.fn();
  mockIPC((comando) => {
    if (comando === "recuperar_operaciones") {
      pedida();
      return recuperacion;
    }
    return undefined;
  });
  const alRestaurar = vi.fn(() => Promise.resolve());
  const { container } = render(
    <I18nextProvider i18n={crearTextos("es")}>
      <AvisoRecuperacion alRestaurar={alRestaurar} />
    </I18nextProvider>,
  );
  return { alRestaurar, container, pedida };
}

describe("AvisoRecuperacion", () => {
  it("sin nada que terminar no enseña nada ni relee", async () => {
    const { alRestaurar, container, pedida } = pintar(NADA);
    // Primero que responda el núcleo: antes de eso tampoco se pinta nada y la comprobación no diría nada.
    await waitFor(() => expect(pedida).toHaveBeenCalledTimes(1));
    await new Promise((listo) => setTimeout(listo, 0));
    expect(container.textContent).toBe("");
    expect(alRestaurar).not.toHaveBeenCalled();
  });

  it("si devolvió ficheros lo dice y vuelve a leer los personajes", async () => {
    const { alRestaurar } = pintar({ ...NADA, restauradas: 1 });
    expect((await screen.findByRole("status")).textContent).toBe(
      "Una operación se cortó a mitad y se ha dejado todo como estaba.",
    );
    expect(alRestaurar).toHaveBeenCalledTimes(1);
  });

  it("lo que pide algo al usuario va como alerta y primero", async () => {
    pintar({ ...NADA, canceladas: 2, pendientes: 1 });
    expect((await screen.findByRole("alert")).textContent).toMatch(/^Hay una operación a medias, pero WoW está abierto/);
    expect(screen.getByRole("status").textContent).toBe(
      "2 operaciones se cortaron antes de tocar nada y se han cancelado.",
    );
  });
});
