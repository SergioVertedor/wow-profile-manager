import { afterEach, describe, expect, it, vi } from "vitest";
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { emit } from "@tauri-apps/api/event";
import { I18nextProvider } from "react-i18next";
import { EVENTO_LINEA, type EstadoDiagnostico, type LineaDiagnostico } from "../nucleo";
import { crearTextos } from "../textos";
import Diagnostico from "./Diagnostico";
import { juntarLineas } from "./useDiagnostico";

afterEach(async () => {
  cleanup();
  // Al desmontar, las vistas dejan de escuchar eventos en una promesa: los mocks tienen que seguir hasta entonces.
  await new Promise((listo) => setTimeout(listo, 0));
  clearMocks();
});

function linea(n: number, mensaje = `línea ${n}`, nivel: LineaDiagnostico["nivel"] = "INFO"): LineaDiagnostico {
  return { n, instante: "2026-11-19 10:32:05 +01:00", nivel, modulo: "deteccion", mensaje };
}

function pintar(estado: Omit<EstadoDiagnostico, "maximo">, cambiar: () => unknown = () => undefined) {
  const llamadas = { cambiar: vi.fn(), abrir: vi.fn() };
  mockIPC(
    (comando, argumentos) => {
      switch (comando) {
        case "estado_diagnostico":
          return { ...estado, maximo: 1000 };
        case "cambiar_registro":
          llamadas.cambiar(argumentos);
          return cambiar();
        case "abrir_carpeta_registros":
          llamadas.abrir();
          return undefined;
        default:
          return undefined;
      }
    },
    { shouldMockEvents: true },
  );
  render(
    <I18nextProvider i18n={crearTextos("es")}>
      <Diagnostico />
    </I18nextProvider>,
  );
  return llamadas;
}

function interruptor() {
  return screen.getByRole("switch", { name: "Guardar el registro en un fichero .txt" }) as HTMLInputElement;
}

describe("juntarLineas", () => {
  it("ordena por número y no repite", () => {
    const juntas = juntarLineas([linea(2), linea(0)], [linea(1), linea(2, "repetida")], 10);
    expect(juntas.map((l) => l.n)).toEqual([0, 1, 2]);
  });

  it("se queda con las más nuevas", () => {
    const muchas = Array.from({ length: 1005 }, (_, n) => linea(n));
    const juntas = juntarLineas([], muchas, 1000);
    expect(juntas).toHaveLength(1000);
    expect(juntas[0].n).toBe(5);
  });
});

describe("Diagnostico", () => {
  it("enseña las líneas guardadas y el interruptor como esté", async () => {
    pintar({ guarda: false, lineas: [linea(0, "C:\\WoW (Registro): instalación con 1 versiones")] });
    expect(await screen.findByText(/instalación con 1 versiones/)).toBeTruthy();
    await waitFor(() => expect(interruptor().disabled).toBe(false));
    expect(interruptor().checked).toBe(false);
    // Con el registro apagado no hay carpeta que abrir.
    expect(screen.queryByRole("button", { name: "Abrir carpeta de registros" })).toBeNull();
  });

  it("añade en vivo las líneas nuevas sin repetir las que ya tenía", async () => {
    pintar({ guarda: false, lineas: [linea(0)] });
    await screen.findByText(/línea 0/);
    await act(() => emit(EVENTO_LINEA, linea(1, "nueva")));
    await act(() => emit(EVENTO_LINEA, linea(0)));
    expect(await screen.findByText(/nueva/)).toBeTruthy();
    expect(screen.getAllByRole("listitem")).toHaveLength(2);
  });

  it("al encender el interruptor lo guarda y ofrece abrir la carpeta", async () => {
    const llamadas = pintar({ guarda: false, lineas: [] });
    await waitFor(() => expect(interruptor().disabled).toBe(false));
    fireEvent.click(interruptor());
    await waitFor(() => expect(llamadas.cambiar).toHaveBeenCalledWith({ guardar: true }));
    fireEvent.click(await screen.findByRole("button", { name: "Abrir carpeta de registros" }));
    expect(llamadas.abrir).toHaveBeenCalledTimes(1);
  });

  it("si no se puede guardar, el interruptor vuelve a como estaba y se dice", async () => {
    pintar({ guarda: false, lineas: [] }, () => {
      throw { codigo: "escrituraFallida", datos: { ruta: "preferencias.json", detalle: "sin permiso" } };
    });
    await waitFor(() => expect(interruptor().disabled).toBe(false));
    fireEvent.click(interruptor());
    expect(await screen.findByRole("alert")).toBeTruthy();
    expect(interruptor().checked).toBe(false);
  });

  it("sin líneas lo dice", async () => {
    pintar({ guarda: true, lineas: [] });
    expect(await screen.findByText("Todavía no hay líneas en esta sesión.")).toBeTruthy();
  });
});
