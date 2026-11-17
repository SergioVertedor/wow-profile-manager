import { afterEach, describe, expect, it } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import App from "./App";

// `mockIPC` sustituye el canal con Rust: estos tests prueban la interfaz, no el núcleo. Que el comando exista y
// devuelva esos campos lo prueban los tests de `src-tauri/src/info.rs`.
afterEach(() => {
  cleanup();
  clearMocks();
});

describe("App", () => {
  it("pide info_app y enseña nombre y versión", async () => {
    const pedidos: string[] = [];
    mockIPC((comando) => {
      pedidos.push(comando);
      return { nombre: "WoW Profile Manager", version: "9.9.9" };
    });

    render(<App />);

    expect(await screen.findByRole("heading")).toHaveProperty("textContent", "WoW Profile Manager 9.9.9");
    expect(pedidos).toEqual(["info_app"]);
  });

  it("si el núcleo falla, lo dice en vez de quedarse cargando", async () => {
    mockIPC(() => {
      throw new Error("sin núcleo");
    });

    render(<App />);

    expect((await screen.findByRole("alert")).textContent).toContain("sin núcleo");
  });
});
