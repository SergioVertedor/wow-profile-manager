import { afterEach, describe, expect, it } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { I18nextProvider } from "react-i18next";
import AcercaDe from "./AcercaDe";
import type { InfoApp } from "./nucleo";
import { crearTextos, type Idioma } from "./textos";

afterEach(() => {
  cleanup();
  clearMocks();
});

const INFO: InfoApp = {
  nombre: "WoW Profile Manager",
  version: "9.9.9",
  autor: "Sergio Vertedor",
  repositorio: "https://github.com/SergioVertedor/wow-profile-manager",
  koFi: "https://ko-fi.com/tonotabidev",
  licencia: "https://www.gnu.org/licenses/gpl-3.0.html",
};

type Pedido = { comando: string; argumentos: unknown };

function pintar(idioma: Idioma, responder: () => unknown = () => null): Pedido[] {
  const pedidos: Pedido[] = [];
  mockIPC((comando, argumentos) => {
    pedidos.push({ comando, argumentos });
    return responder();
  });
  render(
    <I18nextProvider i18n={crearTextos(idioma)}>
      <AcercaDe info={INFO} />
    </I18nextProvider>,
  );
  return pedidos;
}

describe("AcercaDe", () => {
  it("enseña autoría, versión, licencia, Ko-fi, repositorio y el aviso de marca", () => {
    pintar("es");

    expect(screen.getByText("WoW Profile Manager 9.9.9")).toBeDefined();
    // Forma del aviso de autoría de `NOTICE.md` (término 7(b)).
    expect(screen.getByText(INFO.repositorio).closest("p")?.textContent).toBe(
      "WoW Profile Manager, de Sergio Vertedor — https://github.com/SergioVertedor/wow-profile-manager",
    );
    expect(screen.getByRole("link", { name: INFO.repositorio })).toHaveProperty("href", `${INFO.repositorio}`);
    expect(screen.getByRole("link", { name: INFO.koFi })).toBeDefined();
    expect(screen.getByRole("link", { name: "Leer la licencia" })).toBeDefined();
    expect(screen.getByText(/no está hecho, respaldado ni apoyado por Blizzard Entertainment/)).toBeDefined();
  });

  it.each([
    [INFO.repositorio, "repositorio"],
    [INFO.koFi, "koFi"],
    ["Read the license", "licencia"],
  ])("al pulsar «%s» pide al núcleo el enlace por su nombre, nunca la URL", async (texto, enlace) => {
    const pedidos = pintar("en");

    fireEvent.click(screen.getByRole("link", { name: texto }));

    await waitFor(() => expect(pedidos).toEqual([{ comando: "abrir_enlace", argumentos: { enlace } }]));
  });

  it("el clic con la rueda tampoco navega la ventana: también lo abre el núcleo", async () => {
    const pedidos = pintar("en");
    const enlace = screen.getByRole("link", { name: INFO.koFi });

    const navegaria = fireEvent(enlace, new MouseEvent("auxclick", { bubbles: true, cancelable: true, button: 1 }));

    expect(navegaria).toBe(false);
    await waitFor(() => expect(pedidos).toEqual([{ comando: "abrir_enlace", argumentos: { enlace: "koFi" } }]));
  });

  it("si el núcleo no puede abrir el enlace, lo dice traducido", async () => {
    pintar("en", () => {
      throw { codigo: "enlaceNoAbierto", datos: { url: INFO.koFi, detalle: "no browser" } };
    });

    fireEvent.click(screen.getByRole("link", { name: INFO.koFi }));

    expect((await screen.findByRole("alert")).textContent).toBe(
      "Could not open https://ko-fi.com/tonotabidev: no browser",
    );
  });
});
