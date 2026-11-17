import { afterEach, describe, expect, it } from "vitest";
import { cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { I18nextProvider } from "react-i18next";
import Instalaciones from "./Instalaciones";
import type { Addon, Instalacion, VersionJuego } from "./nucleo";
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
          {
            carpetaReino: "93",
            carpetaPersonaje: "Goldberg-Joestar",
            nombreEnDatos: { texto: "Goldberg Joestar", origen: "datos" },
          },
          {
            carpetaReino: "93",
            carpetaPersonaje: "Goldberg-Sp",
            nombreEnDatos: { texto: "Goldberg Sp", origen: "carpeta" },
          },
          { carpetaReino: "93", carpetaPersonaje: "Goldber8D31D", nombreEnDatos: null },
        ],
      },
    ],
  },
  addons: { addons: [], ignoradas: 0 },
};

function addon(carpeta: string, titulo: string | null, version: string | null): Addon {
  return { carpeta, toc: `${carpeta}.toc`, titulo, version, enlace: null, dependencias: [], svCuenta: [], svPersonaje: [] };
}

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
    expect(screen.getByText("Cuenta 1#1 · 3 personajes")).toBeDefined();
    expect(screen.getByText("1 carpeta de WTF/Account no es una cuenta y se ha saltado.")).toBeDefined();
    expect(screen.getByText("2 carpetas de reino son de la otra versión del juego y se han saltado.")).toBeDefined();
    expect(screen.getAllByRole("listitem").map((li) => li.textContent)).toEqual([
      "Goldberg Joestar · 93/Goldberg-Joestar",
      "Goldberg Sp (deducido de la carpeta) · 93/Goldberg-Sp",
      "93/Goldber8D31D · nombre sin resolver",
    ]);
  });

  it("cada nombre lleva la calidad de su origen: datos raro, carpeta poco común, sin resolver pobre", async () => {
    pintar("es", () => instalacion(FOREVER));

    const filas = await screen.findAllByRole("button", { pressed: false });
    expect(filas.map((fila) => fila.dataset.calidad)).toEqual(["raro", "pocoComun", "pobre"]);
  });

  it("sin elegir, el marco invita a elegir; al elegir, enseña la ficha de ese personaje", async () => {
    pintar("es", () => instalacion(FOREVER));

    const marco = await screen.findByRole("region", { name: "Ficha del personaje" });
    expect(marco.textContent).toBe("Elige un personaje de la lista para ver su ficha.");

    fireEvent.click(screen.getByRole("button", { name: /Goldberg Sp/ }));

    const ficha = screen.getByRole("region", { name: "Ficha del personaje" });
    expect(within(ficha).getByRole("heading", { name: "Goldberg Sp" }).dataset.calidad).toBe("pocoComun");
    expect(within(ficha).getByText("Nombre deducido de la carpeta")).toBeDefined();
    expect(within(ficha).getByText("WoW Forever")).toBeDefined();
    expect(within(ficha).getByText("93/Goldberg-Sp")).toBeDefined();
    expect(screen.getByRole("button", { name: /Goldberg Sp/ }).getAttribute("aria-pressed")).toBe("true");
  });

  it("elegir otro personaje desmarca el anterior y cambia la ficha", async () => {
    pintar("es", () => instalacion(FOREVER));

    fireEvent.click(await screen.findByRole("button", { name: /Goldberg Joestar/ }));
    fireEvent.click(screen.getByRole("button", { name: /Goldberg Sp/ }));

    expect(screen.getAllByRole("button", { pressed: true }).map((b) => b.textContent)).toEqual([
      "Goldberg Sp (deducido de la carpeta) · 93/Goldberg-Sp",
    ]);
    const ficha = screen.getByRole("region", { name: "Ficha del personaje" });
    expect(within(ficha).getByRole("heading", { name: "Goldberg Sp" })).toBeDefined();
  });

  it("un personaje sin resolver se enseña por su carpeta y con «?» de retrato", async () => {
    pintar("en", () => instalacion(FOREVER));

    fireEvent.click(await screen.findByRole("button", { name: /Goldber8D31D/ }));

    const ficha = screen.getByRole("region", { name: "Character sheet" });
    expect(within(ficha).getByRole("heading", { name: "Goldber8D31D" }).dataset.calidad).toBe("pobre");
    expect(within(ficha).getByText("Name not resolved: only the folder is known")).toBeDefined();
    expect(within(ficha).getByText("?")).toBeDefined();
  });

  it("sin instalaciones no hay marco de personaje", async () => {
    pintar("en", () => []);

    expect(await screen.findByText("No WoW installation found on this computer.")).toBeDefined();
    expect(screen.queryByRole("region", { name: "Character sheet" })).toBeNull();
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

  it("enseña los addons con su versión, la carpeta si el .toc no trae título y las carpetas saltadas", async () => {
    const conAddons: VersionJuego = {
      ...FOREVER,
      addons: { addons: [addon("APR", "Azeroth Pilot Reloaded", "v7.7.1"), addon("SinTitulo", null, null)], ignoradas: 1 },
    };
    pintar("es", () => instalacion(conAddons));

    expect(await screen.findByText("2 addons instalados")).toBeDefined();
    expect(
      screen.getByText("1 carpeta de Interface/AddOns no tiene un .toc que el juego cargue y se ha saltado."),
    ).toBeDefined();
    expect(screen.getByText("Azeroth Pilot Reloaded · v7.7.1")).toBeDefined();
    expect(screen.getByText("SinTitulo")).toBeDefined();
  });

  it("no enseña addons de una versión no soportada y usa el singular", async () => {
    const unAddon: VersionJuego = { ...FOREVER, addons: { addons: [addon("Details", "Details!", null)], ignoradas: 0 } };
    const clasica: VersionJuego = { ...unAddon, carpeta: "_classic_", producto: "wow_classic", soportada: false };
    pintar("en", () => instalacion(unAddon, clasica));

    expect(await screen.findByText("1 addon installed")).toBeDefined();
    expect(screen.getAllByText("Details!")).toHaveLength(1);
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
