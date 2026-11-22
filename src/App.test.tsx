import { afterEach, describe, expect, it } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { I18nextProvider } from "react-i18next";
import App from "./App";
import type { Deteccion } from "./nucleo";
import { crearTextos, type Idioma } from "./textos";

// `mockIPC` sustituye el canal con Rust: estos tests prueban la interfaz, no el núcleo. Que el comando exista y
// devuelva esos campos lo prueban los tests de `src-tauri/src/info.rs`.
afterEach(async () => {
  cleanup();
  // Al desmontar, las vistas dejan de escuchar eventos en una promesa: los mocks tienen que seguir hasta entonces.
  await new Promise((listo) => setTimeout(listo, 0));
  clearMocks();
});

/** Lo que da `detectar_instalaciones` sin nada instalado. */
const SIN_NADA: Deteccion = { instalaciones: [], noEncontradas: [], preferencias: null };

/** Con los eventos simulados: `App` monta vistas que escuchan eventos del núcleo (#60). */
function simular(respuesta: Parameters<typeof mockIPC>[0]) {
  mockIPC(respuesta, { shouldMockEvents: true });
}

function pintar(idioma: Idioma) {
  render(
    <I18nextProvider i18n={crearTextos(idioma)}>
      <App />
    </I18nextProvider>,
  );
}

describe("App", () => {
  it("pide info_app y enseña nombre y versión en la barra", async () => {
    const pedidos: string[] = [];
    simular((comando) => {
      pedidos.push(comando);
      return comando === "info_app" ? { nombre: "WoW Profile Manager", version: "9.9.9" } : SIN_NADA;
    });

    pintar("es");

    expect(await screen.findByText("WoW Profile Manager")).toBeDefined();
    expect(screen.getByText("Versión 9.9.9")).toBeDefined();
    expect(pedidos).toContain("info_app");
  });

  it.each([
    ["es", "No se pudo contactar con el núcleo: Error: sin núcleo"],
    ["en", "Could not reach the core: Error: sin núcleo"],
  ] as const)("si el núcleo falla, lo dice en su idioma (%s)", async (idioma, esperado) => {
    simular(() => {
      throw new Error("sin núcleo");
    });

    pintar(idioma);

    // También falla la lista de instalaciones: aquí solo importa la alerta de la cabecera, y que sea una.
    const alertas = (await screen.findAllByRole("alert")).filter((a) => a.textContent === esperado);
    expect(alertas).toHaveLength(1);
    // Diagnóstico sigue a mano: es cuando más falta hace.
    expect(screen.getByRole("button", { name: idioma === "es" ? "Diagnóstico" : "Diagnostics" })).toBeDefined();
  });

  it("mientras carga, lo dice en el idioma elegido", () => {
    simular(() => new Promise(() => undefined));

    pintar("en");

    expect(screen.getByText("Loading…")).toBeDefined();
    expect(screen.getByText("Looking for WoW installations…")).toBeDefined();
  });

  it("lleva a «Acerca de» y vuelve a los personajes", async () => {
    simular((comando) =>
      comando === "info_app"
        ? {
            nombre: "WoW Profile Manager",
            version: "9.9.9",
            autor: "Sergio Vertedor",
            repositorio: "https://github.com/SergioVertedor/wow-profile-manager",
            koFi: "https://ko-fi.com/tonotabidev",
            web: "https://tonotabi.dev",
            licencia: "https://www.gnu.org/licenses/gpl-3.0.html",
          }
        : SIN_NADA,
    );

    pintar("es");

    fireEvent.click(await screen.findByRole("button", { name: "Acerca de" }));
    expect(screen.getByRole("heading", { level: 1, name: "Acerca de" })).toBeDefined();
    expect(screen.queryByRole("heading", { level: 1, name: "Personajes" })).toBeNull();

    fireEvent.click(screen.getByRole("button", { name: "Personajes" }));
    expect(await screen.findByRole("heading", { level: 1, name: "Personajes" })).toBeDefined();
    expect(screen.getByRole("button", { name: "Personajes" }).getAttribute("aria-pressed")).toBe("true");
  });
});
