import { afterEach, beforeAll, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { I18nextProvider } from "react-i18next";
import type { Historial as DatosHistorial, Operacion } from "../nucleo";
import { crearTextos } from "../textos";
import Historial from "./Historial";

// jsdom no implementa el diálogo modal: lo justo para abrirlo y cerrarlo (con su evento `close`, como el navegador).
beforeAll(() => {
  HTMLDialogElement.prototype.showModal = function (this: HTMLDialogElement) {
    this.open = true;
  };
  HTMLDialogElement.prototype.close = function (this: HTMLDialogElement) {
    this.open = false;
    this.dispatchEvent(new Event("close"));
  };
});

afterEach(() => {
  cleanup();
  clearMocks();
});

const ELIMINACION: Operacion = {
  id: "20261118-093549-0000abcd",
  tipo: "eliminacion",
  creada: "2026-11-18T09:35:49Z",
  terminada: "2026-11-18T09:35:50Z",
  estado: "completa",
  error: null,
  destino: {
    raizVersion: "C:\\WoW\\_forever_",
    producto: "wowf",
    cuenta: "107710745#1",
    carpetaReino: "93",
    carpetaPersonaje: "Goldberg-Soulreaper",
    nombreEnDatos: "Goldberg",
  },
  deshace: null,
  deshechaPor: null,
  deshaceBorrada: false,
  ficheros: 54,
  bytes: 120_000,
  deshacible: true,
  aMedias: false,
  porResolver: false,
  paquete: null,
  sinDevolver: [],
};

const IMPORTACION: Operacion = {
  ...ELIMINACION,
  id: "20261118-120000-00000003",
  tipo: "importacion",
  paquete: { nombre: "Druida feral", sha256: "a".repeat(64) },
};

const IMPORTACION_A_MEDIAS: Operacion = {
  ...IMPORTACION,
  id: "20261118-130000-00000004",
  estado: "escribiendo",
  terminada: null,
  deshacible: false,
  aMedias: true,
  porResolver: true,
  sinDevolver: [{ ruta: "C:\\WoW\\_forever_\\WTF\\Account\\1#1\\config-cache.wtf", copia: "C:\\datos\\copias\\0" }],
};

const SIN_RESTAURAR: Operacion = {
  ...ELIMINACION,
  id: "20261117-080000-0000beef",
  creada: "2026-11-17T08:00:00Z",
  estado: "fallida-sin-restaurar",
  error: "interrumpida. Sin restaurar: …",
  deshacible: false,
  porResolver: true,
  destino: { ...ELIMINACION.destino, carpetaPersonaje: "Goldberg-Joestar", nombreEnDatos: undefined },
  sinDevolver: [{ ruta: "C:\\WoW\\_forever_\\WTF\\x.lua", copia: "C:\\datos\\operaciones\\20261117-080000-0000beef\\copias\\3" }],
};

const A_MEDIAS: Operacion = {
  ...ELIMINACION,
  id: "20261116-080000-00000001",
  estado: "escribiendo",
  deshacible: false,
  destino: { ...ELIMINACION.destino, nombreEnDatos: "Joestar" },
};

function historial(operaciones: Operacion[], extra: Partial<DatosHistorial> = {}): DatosHistorial {
  return { operaciones, bytes: operaciones.reduce((total, o) => total + o.bytes, 0), ilegibles: [], ...extra };
}

function pintar(respuestas: Record<string, (argumentos: unknown) => unknown>) {
  const llamadas: { comando: string; argumentos: unknown }[] = [];
  mockIPC((comando, argumentos) => {
    llamadas.push({ comando, argumentos });
    return respuestas[comando]?.(argumentos);
  });
  const alCambiarWtf = vi.fn(() => Promise.resolve());
  render(
    <I18nextProvider i18n={crearTextos("es")}>
      <Historial alCambiarWtf={alCambiarWtf} />
    </I18nextProvider>,
  );
  return { llamadas, alCambiarWtf };
}

const veces = (llamadas: { comando: string }[], comando: string) =>
  llamadas.filter((llamada) => llamada.comando === comando).length;

describe("Historial", () => {
  it("lista las operaciones con lo que ocupan y solo ofrece lo que se puede hacer", async () => {
    pintar({ listar_operaciones: () => historial([ELIMINACION, A_MEDIAS]) });

    const fila = await screen.findByRole("listitem", { name: "Carpeta eliminada: Goldberg" });
    expect(screen.getByText(/^Ocupa 240\skB en total$/)).toBeTruthy();
    expect(within(fila).getByText("Hecha")).toBeTruthy();
    expect(within(fila).getByText(/WoW Forever · 107710745#1 · 54 ficheros · 120\skB/)).toBeTruthy();
    expect(within(fila).getByRole("button", { name: "Deshacer" })).toBeTruthy();
    expect(within(fila).getByRole("button", { name: "Borrar" })).toBeTruthy();

    const aMedias = screen.getByRole("listitem", { name: "Carpeta eliminada: Joestar" });
    expect(within(aMedias).queryByRole("button", { name: "Deshacer" })).toBeNull();
    expect(within(aMedias).queryByRole("button", { name: "Borrar" })).toBeNull();
    expect(within(aMedias).getByRole("button", { name: "Abrir carpeta" })).toBeTruthy();
  });

  it("sin operaciones lo dice", async () => {
    pintar({ listar_operaciones: () => historial([]) });
    expect(await screen.findByText("Aún no hay operaciones.")).toBeTruthy();
  });

  it("una sin restaurar dice qué falta y dónde está su copia", async () => {
    pintar({ listar_operaciones: () => historial([SIN_RESTAURAR]) });

    const fila = await screen.findByRole("listitem", { name: "Carpeta eliminada: Goldberg-Joestar" });
    expect(within(fila).getByText("Falló sin poder restaurar")).toBeTruthy();
    expect(within(fila).getByRole("alert").textContent).toContain(
      "C:\\WoW\\_forever_\\WTF\\x.lua ← C:\\datos\\operaciones\\20261117-080000-0000beef\\copias\\3",
    );
    expect(within(fila).getByRole("button", { name: "Restaurar" })).toBeTruthy();
  });

  it("cuenta aparte los diarios ilegibles y deja abrir su carpeta", async () => {
    const { llamadas } = pintar({
      listar_operaciones: () => historial([], { bytes: 6, ilegibles: [{ id: "20261118-000000-00000000", bytes: 6 }] }),
    });

    expect(await screen.findByText(/Hay una operación cuyo diario no se entiende/)).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Abrir carpeta" }));
    expect(llamadas.find((l) => l.comando === "abrir_carpeta_operacion")?.argumentos).toEqual({
      id: "20261118-000000-00000000",
    });
  });

  it("deshacer enseña lo que se pisa, manda la huella del plan y vuelve a leer al cerrar", async () => {
    const { llamadas, alCambiarWtf } = pintar({
      listar_operaciones: () => historial([ELIMINACION]),
      plan_deshacer: () => ({
        vuelven: 2,
        sobran: 1,
        cambian: ["WTF/Account/1#1/93/Goldberg-Soulreaper/AddOns.txt"],
        deCuenta: [],
        addons: [],
        addonsCambiados: [],
        huella: "h1",
      }),
      deshacer_operacion: () => "20261118-100000-00000002",
    });
    fireEvent.click(await screen.findByRole("button", { name: "Deshacer" }));

    const dialogo = await screen.findByRole("dialog");
    expect(within(dialogo).getByRole("heading", { name: "¿Deshacer la eliminación de Goldberg?" })).toBeTruthy();
    expect(
      within(dialogo).getByText(
        "Vuelven 2 ficheros a como estaban. Se quita 1 fichero que antes no estaba.",
      ),
    ).toBeTruthy();
    expect(within(dialogo).getByText("WTF/Account/1#1/93/Goldberg-Soulreaper/AddOns.txt")).toBeTruthy();
    fireEvent.click(within(dialogo).getByRole("button", { name: "Deshacer" }));

    expect(await within(dialogo).findByRole("status")).toBeTruthy();
    expect(llamadas.find((l) => l.comando === "deshacer_operacion")?.argumentos).toEqual({
      id: ELIMINACION.id,
      huella: "h1",
    });
    fireEvent.click(within(dialogo).getByRole("button", { name: "Cerrar" }));
    expect(alCambiarWtf).toHaveBeenCalledOnce();
    await waitFor(() => expect(veces(llamadas, "listar_operaciones")).toBe(2));
  });

  it("deshacer una importación con addons dice cuántos se quitan y qué se queda", async () => {
    pintar({
      listar_operaciones: () => historial([IMPORTACION]),
      plan_deshacer: () => ({
        vuelven: 0,
        sobran: 0,
        cambian: [],
        deCuenta: [],
        addons: ["Interface/AddOns/Dominos"],
        addonsCambiados: ["C:/WoW/_forever_/Interface/AddOns/Dominos/Dominos.toc"],
        huella: "h1",
      }),
    });
    fireEvent.click(await screen.findByRole("button", { name: "Deshacer" }));

    const dialogo = await screen.findByRole("dialog");
    expect(within(dialogo).getByText("Se quita 1 addon que se instaló.")).toBeTruthy();
    expect(within(dialogo).getByText(/cambiaron o se añadieron después: se quedan/)).toBeTruthy();
    expect(within(dialogo).getByText("C:/WoW/_forever_/Interface/AddOns/Dominos/Dominos.toc")).toBeTruthy();
  });

  it("si el plan ya no vale, lo dice en el diálogo", async () => {
    pintar({
      listar_operaciones: () => historial([ELIMINACION]),
      plan_deshacer: () => ({ vuelven: 2, sobran: 0, cambian: [], deCuenta: [], addons: [], addonsCambiados: [], huella: "h1" }),
      deshacer_operacion: () => Promise.reject({ codigo: "planCaducado", datos: {} }),
    });
    fireEvent.click(await screen.findByRole("button", { name: "Deshacer" }));
    const dialogo = await screen.findByRole("dialog");
    fireEvent.click(within(dialogo).getByRole("button", { name: "Deshacer" }));

    expect((await within(dialogo).findByRole("alert")).textContent).toMatch(/Algún fichero ha cambiado/);
  });

  it("una importación dice su perfil y al deshacerla avisa de lo que es de la cuenta", async () => {
    pintar({
      listar_operaciones: () => historial([IMPORTACION]),
      plan_deshacer: () => ({
        vuelven: 1,
        sobran: 1,
        cambian: ["WTF/Account/1#1/config-cache.wtf"],
        deCuenta: ["WTF/Account/1#1/config-cache.wtf"],
        addons: [],
        addonsCambiados: [],
        huella: "h1",
      }),
    });

    const fila = await screen.findByRole("listitem", { name: "Importación: Goldberg" });
    expect(within(fila).getByText("Perfil «Druida feral»")).toBeTruthy();
    fireEvent.click(within(fila).getByRole("button", { name: "Deshacer" }));
    const dialogo = await screen.findByRole("dialog");
    expect(within(dialogo).getByRole("heading", { name: "¿Deshacer la importación en Goldberg?" })).toBeTruthy();
    expect(within(dialogo).getByText(/Estos son de la cuenta/)).toBeTruthy();
  });

  it("una importación a medias no se deshace ni se borra: se restaura al confirmarlo", async () => {
    const { llamadas, alCambiarWtf } = pintar({
      listar_operaciones: () => historial([IMPORTACION_A_MEDIAS]),
      restaurar_operacion: () => null,
    });

    const fila = await screen.findByRole("listitem", { name: "Importación: Goldberg" });
    expect(within(fila).getByText("A medias")).toBeTruthy();
    expect(within(fila).getByRole("alert").textContent).toMatch(/Se cortó a mitad de escribir/);
    expect(within(fila).queryByRole("button", { name: "Deshacer" })).toBeNull();
    expect(within(fila).queryByRole("button", { name: "Borrar" })).toBeNull();
    fireEvent.click(within(fila).getByRole("button", { name: "Restaurar" }));
    expect(veces(llamadas, "restaurar_operacion")).toBe(0);

    const dialogo = await screen.findByRole("dialog");
    fireEvent.click(within(dialogo).getByRole("button", { name: "Restaurar" }));
    expect((await within(dialogo).findByRole("status")).textContent).toBe("Hecho: todo está como antes de la operación.");
    expect(llamadas.find((l) => l.comando === "restaurar_operacion")?.argumentos).toEqual({
      id: IMPORTACION_A_MEDIAS.id,
    });
    fireEvent.click(within(dialogo).getByRole("button", { name: "Cerrar" }));
    expect(alCambiarWtf).toHaveBeenCalledOnce();
  });

  it("borrar una sin restaurar exige confirmar que se restauró a mano", async () => {
    const { llamadas } = pintar({
      listar_operaciones: () => historial([SIN_RESTAURAR]),
      borrar_operacion: () => null,
    });
    fireEvent.click(await screen.findByRole("button", { name: "Borrar" }));

    const dialogo = await screen.findByRole("dialog");
    expect(within(dialogo).getByText(/Libera 120\skB/)).toBeTruthy();
    const confirmar = within(dialogo).getByRole<HTMLButtonElement>("button", { name: "Borrar" });
    expect(confirmar.disabled).toBe(true);
    fireEvent.click(within(dialogo).getByRole("checkbox", { name: "Ya he restaurado a mano lo que faltaba" }));
    fireEvent.click(confirmar);

    await waitFor(() => expect(veces(llamadas, "listar_operaciones")).toBe(2));
    expect(llamadas.find((l) => l.comando === "borrar_operacion")?.argumentos).toEqual({
      id: SIN_RESTAURAR.id,
      restauradaAMano: true,
    });
  });

  it("si no se puede abrir una carpeta, lo dice sin perder la lista", async () => {
    pintar({
      listar_operaciones: () => historial([ELIMINACION]),
      abrir_carpeta_operacion: () =>
        Promise.reject({ codigo: "carpetaNoAbierta", datos: { ruta: "C:\\datos\\x", detalle: "sin explorador" } }),
    });
    fireEvent.click(await screen.findByRole("button", { name: "Abrir carpeta" }));

    expect((await screen.findByRole("alert")).textContent).toBe("No se pudo abrir C:\\datos\\x: sin explorador");
    expect(screen.getByRole("listitem")).toBeTruthy();
  });
});
