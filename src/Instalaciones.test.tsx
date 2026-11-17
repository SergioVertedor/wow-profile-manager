import { afterEach, describe, expect, it } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { I18nextProvider } from "react-i18next";
import Instalaciones from "./Instalaciones";
import type { Instalacion, VersionJuego } from "./nucleo";
import { crearTextos, type Idioma } from "./textos";

afterEach(() => {
  cleanup();
  clearMocks();
});

function pintar(idioma: Idioma, respuesta: () => unknown) {
  mockIPC((comando) => (comando === "detectar_instalaciones" ? respuesta() : undefined));
  render(
    <I18nextProvider i18n={crearTextos(idioma)}>
      <Instalaciones />
    </I18nextProvider>,
  );
}

const FOREVER: VersionJuego = {
  carpeta: "_forever_",
  producto: "wowf",
  soportada: true,
  build: "1.60.1.70938",
  interfaz: "16001",
  cuentas: {
    ignoradas: 1,
    cuentas: [
      {
        carpeta: "1#1",
        reinosIgnorados: 2,
        personajes: [
          { carpetaReino: "93", carpetaPersonaje: "Goldberg-Joestar" },
          { carpetaReino: "93", carpetaPersonaje: "Goldberg-Sp" },
        ],
      },
    ],
  },
};

const CUENTA = FOREVER.cuentas.cuentas[0];

function instalacion(...versiones: VersionJuego[]): Instalacion[] {
  return [{ raiz: "C:\\Juegos\\World of Warcraft", fuente: "registro", versiones }];
}

describe("Instalaciones", () => {
  it("enseña versiones, cuentas y personajes con los recuentos en plural", async () => {
    pintar("es", () => instalacion(FOREVER));

    expect(await screen.findByText("WoW Forever · _forever_")).toBeDefined();
    expect(screen.getByText("Encontrada en el registro de Windows")).toBeDefined();
    expect(screen.getByText("Build 1.60.1.70938")).toBeDefined();
    expect(screen.getByText("Cuenta 1#1 · 2 personajes")).toBeDefined();
    expect(screen.getByText("1 carpeta de WTF/Account no es una cuenta y se ha saltado.")).toBeDefined();
    expect(screen.getByText("2 carpetas de reino son de la otra versión del juego y se han saltado.")).toBeDefined();
    expect(screen.getAllByRole("listitem").map((li) => li.textContent)).toEqual([
      "Goldberg-Joestar · 93",
      "Goldberg-Sp · 93",
    ]);
  });

  it("usa el singular con un solo personaje", async () => {
    const unPersonaje: VersionJuego = {
      ...FOREVER,
      cuentas: { ignoradas: 0, cuentas: [{ ...CUENTA, reinosIgnorados: 0, personajes: [CUENTA.personajes[0]] }] },
    };
    pintar("en", () => instalacion(unPersonaje));

    expect(await screen.findByText("Account 1#1 · 1 character")).toBeDefined();
    expect(screen.queryByText(/skipped/)).toBeNull();
  });

  it("avisa de lo que no se puede usar: producto no soportado y versión sin interfaz", async () => {
    const clasica: VersionJuego = {
      ...FOREVER,
      carpeta: "_classic_",
      producto: "wow_classic",
      soportada: false,
      cuentas: { ignoradas: 0, cuentas: [] },
    };
    const sinInterfaz: VersionJuego = { ...FOREVER, carpeta: "_retail_", producto: "wow", interfaz: null };
    pintar("en", () => instalacion(clasica, sinInterfaz));

    expect(await screen.findByText("wow_classic · _classic_")).toBeDefined();
    expect(screen.getByText("This app does not support this game version.")).toBeDefined();
    expect(screen.getByText("World of Warcraft (Retail) · _retail_")).toBeDefined();
    expect(screen.getByText("Open the game once with this version: until then it cannot be exported.")).toBeDefined();
  });

  it("si no hay ninguna instalación, lo dice", async () => {
    pintar("en", () => []);

    expect(await screen.findByText("No WoW installation found on this computer.")).toBeDefined();
  });

  it("traduce el error del núcleo con su código y sus datos", async () => {
    pintar("es", () => {
      throw { codigo: "lecturaFallida", datos: { ruta: "C:\\WoW\\WTF", detalle: "Acceso denegado" } };
    });

    expect((await screen.findByRole("alert")).textContent).toBe("No se pudo leer C:\\WoW\\WTF: Acceso denegado");
  });

  it("un código de error que la interfaz no conoce no se enseña como clave", async () => {
    pintar("en", () => {
      throw { codigo: "codigoFuturo", datos: {} };
    });

    expect((await screen.findByRole("alert")).textContent).toBe("Unexpected error: codigoFuturo");
  });

  it("un fallo que no es del núcleo se enseña como inesperado, sin ocultarlo", async () => {
    pintar("en", () => {
      throw new Error("canal roto");
    });

    expect((await screen.findByRole("alert")).textContent).toBe("Unexpected error: Error: canal roto");
  });
});
