import { afterEach, describe, expect, it } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { I18nextProvider } from "react-i18next";
import App from "./App";
import { crearTextos, type Idioma } from "./textos";

// `mockIPC` sustituye el canal con Rust: estos tests prueban la interfaz, no el núcleo. Que el comando exista y
// devuelva esos campos lo prueban los tests de `src-tauri/src/info.rs`.
afterEach(() => {
  cleanup();
  clearMocks();
});

function pintar(idioma: Idioma) {
  render(
    <I18nextProvider i18n={crearTextos(idioma)}>
      <App />
    </I18nextProvider>,
  );
}

describe("App", () => {
  it("pide info_app y enseña nombre y versión", async () => {
    const pedidos: string[] = [];
    mockIPC((comando) => {
      pedidos.push(comando);
      return comando === "info_app" ? { nombre: "WoW Profile Manager", version: "9.9.9" } : [];
    });

    pintar("es");

    expect(await screen.findByRole("heading", { level: 1 })).toHaveProperty("textContent", "WoW Profile Manager 9.9.9");
    expect(pedidos).toContain("info_app");
  });

  it.each([
    ["es", "No se pudo contactar con el núcleo: Error: sin núcleo"],
    ["en", "Could not reach the core: Error: sin núcleo"],
  ] as const)("si el núcleo falla, lo dice en su idioma (%s)", async (idioma, esperado) => {
    mockIPC(() => {
      throw new Error("sin núcleo");
    });

    pintar(idioma);

    // También falla la lista de instalaciones: aquí solo importa la alerta de la cabecera, y que sea una.
    const alertas = (await screen.findAllByRole("alert")).filter((a) => a.textContent === esperado);
    expect(alertas).toHaveLength(1);
  });

  it("mientras carga, lo dice en el idioma elegido", () => {
    mockIPC(() => new Promise(() => undefined));

    pintar("en");

    expect(screen.getByText("Loading…")).toBeDefined();
    expect(screen.getByText("Looking for WoW installations…")).toBeDefined();
  });
});
