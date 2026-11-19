import { afterEach, describe, expect, it } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { I18nextProvider } from "react-i18next";
import { crearTextos } from "../textos";
import AvisoActualizacion from "./AvisoActualizacion";

afterEach(() => {
  cleanup();
  clearMocks();
});

const NUEVA = { version: "0.2.0", notas: "Novedades" };

function pintar(respuestas: Record<string, () => unknown>) {
  const pedidos: string[] = [];
  mockIPC((comando) => {
    pedidos.push(comando);
    return respuestas[comando]?.();
  });
  render(
    <I18nextProvider i18n={crearTextos("es")}>
      <AvisoActualizacion />
    </I18nextProvider>,
  );
  return pedidos;
}

describe("AvisoActualizacion", () => {
  it("sin versión nueva no enseña nada", async () => {
    const pedidos = pintar({ buscar_actualizacion: () => null });

    await waitFor(() => expect(pedidos).toEqual(["buscar_actualizacion"]));
    expect(screen.queryByRole("status")).toBeNull();
  });

  it("con versión nueva pregunta y solo instala si se pide", async () => {
    const pedidos = pintar({ buscar_actualizacion: () => NUEVA, instalar_actualizacion: () => new Promise(() => {}) });

    expect(await screen.findByText("Hay una versión nueva de la app: 0.2.0.")).toBeDefined();
    expect(pedidos).not.toContain("instalar_actualizacion");

    fireEvent.click(screen.getByRole("button", { name: "Actualizar y reiniciar" }));

    expect(await screen.findByRole("button", { name: "Actualizando…" })).toHaveProperty("disabled", true);
    expect(pedidos).toContain("instalar_actualizacion");
  });

  it("«Ahora no» lo quita", async () => {
    pintar({ buscar_actualizacion: () => NUEVA });

    fireEvent.click(await screen.findByRole("button", { name: "Ahora no" }));

    expect(screen.queryByText(/Hay una versión nueva/)).toBeNull();
  });

  it("si la firma no vale, lo dice y se puede volver a intentar", async () => {
    pintar({
      buscar_actualizacion: () => NUEVA,
      instalar_actualizacion: () => {
        throw { codigo: "actualizacionSinFirmaValida", datos: {} };
      },
    });

    fireEvent.click(await screen.findByRole("button", { name: "Actualizar y reiniciar" }));

    expect((await screen.findByRole("alert")).textContent).toMatch(/no trae la firma de la app/);
    expect(screen.getByRole("button", { name: "Actualizar y reiniciar" })).toHaveProperty("disabled", false);
  });

  it("sin conexión no dice nada", async () => {
    const pedidos = pintar({
      buscar_actualizacion: () => {
        throw new Error("sin red");
      },
    });

    await waitFor(() => expect(pedidos).toEqual(["buscar_actualizacion"]));
    expect(screen.queryByRole("alert")).toBeNull();
  });
});
