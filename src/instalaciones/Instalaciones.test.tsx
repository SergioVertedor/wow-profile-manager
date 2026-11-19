import { afterEach, describe, expect, it } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { I18nextProvider } from "react-i18next";
import type { Deteccion, Instalacion } from "../nucleo";
import { crearTextos, type Idioma } from "../textos";
import Instalaciones from "./Instalaciones";
import { useDeteccion } from "./useDeteccion";

afterEach(() => {
  cleanup();
  clearMocks();
});

type Llamada = Readonly<{ comando: string; argumentos: unknown }>;

/** Simula el núcleo: cada comando responde con lo que diga `respuestas`, y se apunta lo que se ha pedido. */
function pintar(idioma: Idioma, respuestas: Record<string, (argumentos: unknown) => unknown>): Llamada[] {
  const llamadas: Llamada[] = [];
  mockIPC((comando, argumentos) => {
    llamadas.push({ comando, argumentos });
    return respuestas[comando]?.(argumentos);
  });
  render(
    <I18nextProvider i18n={crearTextos(idioma)}>
      <ConDeteccion />
    </I18nextProvider>,
  );
  return llamadas;
}

function ConDeteccion() {
  return <Instalaciones {...useDeteccion()} />;
}

function instalacion(raiz: string, fuente: Instalacion["fuente"], nombre: string | null = null): Instalacion {
  return {
    raiz,
    fuente,
    nombre,
    versiones: [
      {
        carpeta: "_forever_",
        producto: "wowf",
        soportada: true,
        build: null,
        interfaz: 1,
        cuentas: { cuentas: [], ignoradas: 0 },
        addons: { addons: [], ignoradas: 0 },
      },
    ],
  };
}

const DETECCION: Deteccion = {
  instalaciones: [instalacion("C:\\WoW", "registro", "Casa"), instalacion("E:\\Juegos\\WoW", "manual")],
  noEncontradas: [{ raiz: "F:\\Portable", nombre: null }],
  preferencias: null,
};

const tarjeta = (nombre: string) => screen.getByRole("heading", { level: 2, name: nombre }).closest("li") as HTMLElement;
const contar = (llamadas: Llamada[], comando: string) => llamadas.filter((l) => l.comando === comando).length;

describe("Instalaciones", () => {
  it("lista cada instalación con su nombre o «Instalación N», ruta, origen y versiones", async () => {
    pintar("es", { detectar_instalaciones: () => DETECCION });

    await screen.findByRole("heading", { level: 2, name: "Casa" });
    expect(within(tarjeta("Casa")).getByText("C:\\WoW")).toBeDefined();
    expect(within(tarjeta("Casa")).getByText("Registro de Windows · WoW Forever")).toBeDefined();
    expect(within(tarjeta("Instalación 2")).getByText("A mano · WoW Forever")).toBeDefined();
    expect(within(tarjeta("Instalación 3")).getByText(/No encontrada/)).toBeDefined();
  });

  it("solo las añadidas a mano, encontradas o no, se pueden quitar", async () => {
    pintar("es", { detectar_instalaciones: () => DETECCION });

    await screen.findByRole("heading", { level: 2, name: "Casa" });
    expect(within(tarjeta("Casa")).queryByRole("button", { name: /Quitar/ })).toBeNull();
    expect(within(tarjeta("Instalación 2")).getByRole("button", { name: "Quitar Instalación 2 de la lista" })).toBeDefined();
    expect(within(tarjeta("Instalación 3")).getByRole("button", { name: "Quitar Instalación 3 de la lista" })).toBeDefined();
  });

  it("quitar pide al núcleo esa raíz y vuelve a detectar", async () => {
    const llamadas = pintar("es", { detectar_instalaciones: () => DETECCION, quitar_carpeta: () => null });

    fireEvent.click(await screen.findByRole("button", { name: "Quitar Instalación 3 de la lista" }));

    await waitFor(() => expect(contar(llamadas, "detectar_instalaciones")).toBe(2));
    expect(llamadas.find((l) => l.comando === "quitar_carpeta")?.argumentos).toEqual({ raiz: "F:\\Portable" });
  });

  it.each([
    ["noEsInstalacion", /no es una instalación de WoW: le falta el fichero \.build\.info/, 1],
    ["yaEstaba", /ya está en la lista/, 1],
    ["anadida", /Carpeta añadida/, 2],
  ] as const)("al añadir una carpeta (%s) lo dice y solo redetecta si se añadió", async (alta, mensaje, detecciones) => {
    const llamadas = pintar("es", { detectar_instalaciones: () => DETECCION, anadir_carpeta: () => alta });

    fireEvent.click(await screen.findByRole("button", { name: "Añadir carpeta" }));

    const rol = alta === "noEsInstalacion" ? "alert" : "status";
    expect((await screen.findByRole(rol)).textContent).toMatch(mensaje);
    expect(contar(llamadas, "detectar_instalaciones")).toBe(detecciones);
  });

  it("si se cancela el selector no dice nada", async () => {
    const llamadas = pintar("es", { detectar_instalaciones: () => DETECCION, anadir_carpeta: () => "cancelada" });

    fireEvent.click(await screen.findByRole("button", { name: "Añadir carpeta" }));

    await waitFor(() => expect(contar(llamadas, "anadir_carpeta")).toBe(1));
    expect(screen.queryByRole("status")).toBeNull();
    expect(screen.queryByRole("alert")).toBeNull();
  });

  it("renombra con el lápiz: Intro guarda sin volver a detectar, y vacío vuelve a «Instalación N»", async () => {
    const llamadas = pintar("es", { detectar_instalaciones: () => DETECCION, nombrar_instalacion: () => null });

    // El primer lápiz es el de «Casa»: se borra su nombre.
    fireEvent.click((await screen.findAllByRole("button", { name: "Cambiar: Nombre de la instalación" }))[0]);
    const campo = screen.getByRole("textbox", { name: "Nombre de la instalación" });
    expect((campo as HTMLInputElement).value).toBe("Casa");
    fireEvent.change(campo, { target: { value: "   " } });
    fireEvent.submit(campo);

    expect(await screen.findByRole("heading", { level: 2, name: "Instalación 1" })).toBeDefined();
    expect(llamadas.find((l) => l.comando === "nombrar_instalacion")?.argumentos).toEqual({ raiz: "C:\\WoW", nombre: "   " });
    expect(contar(llamadas, "detectar_instalaciones")).toBe(1);
  });

  it("Escape cancela sin pedir nada al núcleo", async () => {
    const llamadas = pintar("es", { detectar_instalaciones: () => DETECCION });

    fireEvent.click((await screen.findAllByRole("button", { name: "Cambiar: Nombre de la instalación" }))[1]);
    fireEvent.keyDown(screen.getByRole("textbox", { name: "Nombre de la instalación" }), { key: "Escape" });

    expect(screen.queryByRole("textbox")).toBeNull();
    expect(contar(llamadas, "nombrar_instalacion")).toBe(0);
    // El foco vuelve al lápiz del que salió.
    expect(document.activeElement).toBe(screen.getAllByRole("button", { name: "Cambiar: Nombre de la instalación" })[1]);
  });

  it("un nombre rechazado dice por qué y el campo sigue abierto", async () => {
    pintar("en", {
      detectar_instalaciones: () => DETECCION,
      nombrar_instalacion: () => {
        throw { codigo: "nombreNoValido", datos: { maximo: 40 } };
      },
    });

    fireEvent.click((await screen.findAllByRole("button", { name: "Rename: Installation name" }))[0]);
    fireEvent.submit(screen.getByRole("textbox", { name: "Installation name" }));

    const alerta = await screen.findByRole("alert");
    expect(alerta.textContent).toBe("A name can have up to 40 characters on a single line.");
    const campo = screen.getByRole("textbox", { name: "Installation name" });
    expect(campo.getAttribute("aria-invalid")).toBe("true");
    expect(campo.getAttribute("aria-describedby")).toBe(alerta.id);
  });

  it("si las preferencias no se pudieron leer, lo dice con la ruta", async () => {
    pintar("es", {
      detectar_instalaciones: () => ({
        ...DETECCION,
        preferencias: { codigo: "preferenciasIlegibles", datos: { ruta: "C:\\datos\\preferencias.json" } },
      }),
    });

    expect((await screen.findByRole("alert")).textContent).toMatch(/^No se entiende C:\\datos\\preferencias\.json/);
  });

  it("sin ninguna instalación invita a añadir la carpeta", async () => {
    pintar("en", { detectar_instalaciones: () => ({ instalaciones: [], noEncontradas: [], preferencias: null }) });

    expect(await screen.findByText("No installations yet. Add your WoW folder by hand.")).toBeDefined();
  });
});
