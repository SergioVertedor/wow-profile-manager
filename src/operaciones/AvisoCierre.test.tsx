import { afterEach, beforeAll, describe, expect, it, vi } from "vitest";
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { emit } from "@tauri-apps/api/event";
import { I18nextProvider } from "react-i18next";
import { EVENTO_CIERRE_BLOQUEADO } from "../nucleo";
import { crearTextos } from "../textos";
import AvisoCierre from "./AvisoCierre";

// jsdom no implementa el diálogo modal: lo justo para abrirlo y cerrarlo.
beforeAll(() => {
  HTMLDialogElement.prototype.showModal = function (this: HTMLDialogElement) {
    this.open = true;
  };
  HTMLDialogElement.prototype.close = function (this: HTMLDialogElement) {
    this.open = false;
  };
});

afterEach(async () => {
  cleanup();
  // Al desmontar, las vistas dejan de escuchar eventos en una promesa: los mocks tienen que seguir hasta entonces.
  await new Promise((listo) => setTimeout(listo, 0));
  clearMocks();
});

function pintar(respuesta: () => unknown = () => undefined) {
  const cerrada = vi.fn();
  mockIPC(
    (comando) => {
      if (comando === "cerrar_ventana") {
        cerrada();
        return respuesta();
      }
      return undefined;
    },
    { shouldMockEvents: true },
  );
  render(
    <I18nextProvider i18n={crearTextos("es")}>
      <AvisoCierre />
    </I18nextProvider>,
  );
  return { cerrada };
}

function dialogo() {
  return screen.getByRole("dialog", { hidden: true }) as HTMLDialogElement;
}

/** El núcleo impide el cierre y manda el evento; hay que esperar a que la vista esté escuchando. */
async function pedirCierre() {
  await waitFor(async () => {
    await act(() => emit(EVENTO_CIERRE_BLOQUEADO));
    expect(dialogo().open).toBe(true);
  });
}

describe("AvisoCierre", () => {
  it("no pregunta nada hasta que el núcleo bloquea un cierre", () => {
    pintar();
    expect(dialogo().open).toBe(false);
  });

  it("al bloquear un cierre pregunta, y esperar deja la ventana abierta", async () => {
    const { cerrada } = pintar();
    await pedirCierre();
    expect(screen.getByText("Hay una operación en marcha")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Esperar" }));
    expect(dialogo().open).toBe(false);
    expect(cerrada).not.toHaveBeenCalled();
  });

  it("cerrar igualmente pide al núcleo que cierre la ventana", async () => {
    const { cerrada } = pintar();
    await pedirCierre();
    fireEvent.click(screen.getByRole("button", { name: "Cerrar igualmente" }));
    await waitFor(() => expect(cerrada).toHaveBeenCalledTimes(1));
  });

  it("si no se puede cerrar, lo dice", async () => {
    pintar(() => {
      throw { codigo: "juegoAbierto", datos: {} };
    });
    await pedirCierre();
    fireEvent.click(screen.getByRole("button", { name: "Cerrar igualmente" }));
    expect(await screen.findByRole("alert", { hidden: true })).toBeTruthy();
  });
});
