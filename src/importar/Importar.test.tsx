import { afterEach, beforeAll, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { I18nextProvider } from "react-i18next";
import type { UsoDeteccion } from "../instalaciones/useDeteccion";
import type { Deteccion, ResumenPaquete, VistaPrevia } from "../nucleo";
import { crearTextos } from "../textos";
import Importar from "./Importar";

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

const RAIZ = "C:\\Juegos\\World of Warcraft";

const DETECCION: Deteccion = {
  instalaciones: [
    {
      raiz: RAIZ,
      fuente: "registro",
      nombre: null,
      versiones: [
        {
          carpeta: "_forever_",
          producto: "wowf",
          soportada: true,
          build: "1.60.1.70938",
          interfaz: 16002,
          cuentas: {
            ignoradas: 0,
            cuentas: [
              {
                carpeta: "1#1",
                nombre: null,
                reinosIgnorados: 0,
                personajes: [
                  { carpetaReino: "93", carpetaPersonaje: "Goldberg-Joestar", nombreEnDatos: { texto: "Goldberg Joestar", origen: "datos" } },
                  { carpetaReino: "93", carpetaPersonaje: "Otro", nombreEnDatos: null },
                ],
              },
            ],
          },
          addons: { addons: [], ignoradas: 0 },
        },
      ],
    },
  ],
  noEncontradas: [],
  preferencias: null,
};

const RESUMEN: ResumenPaquete = {
  sha256: "a".repeat(64),
  manifiesto: {
    formato: { nombre: "wowprofile", version: "1.0" },
    perfil: { nombre: "Druida feral", descripcion: "Para la hermandad", creado: "2026-11-18T10:00:00Z" },
    origen: { producto: "wowf", build: "1.60.1.70938", interfaz: 16001 },
    addons: [],
  },
  entradas: [
    { id: "macros", categoria: "macros", ambito: "personaje", deCuenta: false, bytes: 2_000 },
    { id: "macros-cuenta", categoria: "macros", ambito: "cuenta", deCuenta: true, bytes: 200 },
    { id: "dominos", categoria: "config-addon", ambito: "personaje", deCuenta: false, bytes: 1, incompatible: "noSeImporta" },
  ],
};

const VISTA: VistaPrevia = {
  juegoAbierto: true,
  interfazDestino: 16002,
  huella: "h".repeat(64),
  plan: {
    incompatibles: [],
    personajes: [
      {
        cuenta: "1#1",
        reino: "93",
        personaje: "Goldberg-Joestar",
        conflictos: ["macros-cuenta"],
        ficheros: [
          {
            entrada: "macros",
            categoria: "macros",
            destino: "personaje/macros-cache.txt",
            deCuenta: false,
            grupo: "modificado",
            accion: "sobrescribir",
            elementos: [{ nombre: "Foco", cambio: "cambiado", antes: "Viejo", despues: "Foco" }],
            apartadas: [],
            bytesAntes: 100,
            bytesDespues: 2_000,
          },
        ],
      },
    ],
  },
};

const USO: UsoDeteccion = {
  estado: { tipo: "listo", deteccion: DETECCION },
  actualizar: () => undefined,
  recargar: () => Promise.resolve(),
};

function pintar(respuestas: Record<string, () => unknown>) {
  const llamadas: { comando: string; argumentos: unknown }[] = [];
  mockIPC((comando, argumentos) => {
    llamadas.push({ comando, argumentos });
    return respuestas[comando]?.();
  });
  const irAHistorial = vi.fn();
  const vista = render(
    <I18nextProvider i18n={crearTextos("es")}>
      <Importar {...USO} irAHistorial={irAHistorial} />
    </I18nextProvider>,
  );
  return { llamadas, vista, irAHistorial };
}

describe("Importar", () => {
  it("abre un perfil, marca destinos, decide lo de cuenta y enseña la vista previa sin aplicar", async () => {
    const { llamadas } = pintar({
      abrir_paquete: () => ({ resultado: "abierta", resumen: RESUMEN }),
      plan_importacion: () => VISTA,
    });

    fireEvent.click(screen.getByRole("button", { name: /Abrir perfil/ }));
    expect(await screen.findByRole("heading", { name: "Druida feral" })).toBeTruthy();
    expect(screen.getByText(/interfaz 16001/)).toBeTruthy();

    const siguiente = screen.getByRole("button", { name: "Siguiente" }) as HTMLButtonElement;
    expect(siguiente.disabled).toBe(true);
    fireEvent.click(screen.getByRole("checkbox", { name: /Goldberg Joestar/ }));
    fireEvent.click(screen.getByRole("checkbox", { name: /Otro/ }));
    expect(screen.getByText("2 personajes marcados")).toBeTruthy();
    fireEvent.click(siguiente);

    const dominos = screen.getByRole("checkbox", { name: /Configuración de addons/ }) as HTMLInputElement;
    expect({ marcada: dominos.checked, desactivada: dominos.disabled }).toEqual({ marcada: false, desactivada: true });
    expect(screen.getByText(/aún no lo importa/)).toBeTruthy();
    const decision = screen.getByRole("group", { name: /Macros de la cuenta en la cuenta 1#1/ });
    expect(within(decision).getByText(/Goldberg Joestar, Otro/, { exact: false })).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Siguiente" }));

    expect(await screen.findByText("Modificados")).toBeTruthy();
    expect(screen.getByText(/WoW está abierto/)).toBeTruthy();
    expect(screen.getByText("Macros de la cuenta: decide si se aplica en esta cuenta.")).toBeTruthy();
    expect(screen.getByText("Viejo → Foco", { exact: false })).toBeTruthy();
    expect(screen.getByRole<HTMLButtonElement>("button", { name: "Aplicar…" }).disabled).toBe(true);
    expect(screen.getByText("Cierra WoW para poder aplicar.")).toBeTruthy();

    const plan = llamadas.find((l) => l.comando === "plan_importacion");
    expect(plan?.argumentos).toEqual({
      sha256: RESUMEN.sha256,
      personajes: [
        { raiz: RAIZ, version: "_forever_", cuenta: "1#1", reino: "93", personaje: "Goldberg-Joestar" },
        { raiz: RAIZ, version: "_forever_", cuenta: "1#1", reino: "93", personaje: "Otro" },
      ],
      pedido: { seleccion: ["macros", "macros-cuenta"], cuentas: [] },
    });
  });

  it("manda la decisión de cuenta y vuelve atrás sin perder lo elegido", async () => {
    const { llamadas } = pintar({
      abrir_paquete: () => ({ resultado: "abierta", resumen: RESUMEN }),
      plan_importacion: () => VISTA,
    });
    fireEvent.click(screen.getByRole("button", { name: /Abrir perfil/ }));
    fireEvent.click(await screen.findByRole("checkbox", { name: /Goldberg Joestar/ }));
    fireEvent.click(screen.getByRole("button", { name: "Siguiente" }));
    fireEvent.click(screen.getByRole("radio", { name: "No aplicar" }));
    fireEvent.click(screen.getByRole("button", { name: "Atrás" }));
    expect((screen.getByRole("checkbox", { name: /Goldberg Joestar/ }) as HTMLInputElement).checked).toBe(true);
    fireEvent.click(screen.getByRole("button", { name: "Siguiente" }));
    expect((screen.getByRole("radio", { name: "No aplicar" }) as HTMLInputElement).checked).toBe(true);
    fireEvent.click(screen.getByRole("button", { name: "Siguiente" }));

    await waitFor(() => expect(llamadas.some((l) => l.comando === "plan_importacion")).toBe(true));
    const plan = llamadas.find((l) => l.comando === "plan_importacion");
    expect((plan?.argumentos as { pedido: unknown }).pedido).toEqual({
      seleccion: ["macros", "macros-cuenta"],
      cuentas: [{ cuenta: "1#1", entrada: "macros-cuenta", aceptar: false }],
    });
  });

  it("aplica con la huella de la vista previa y enseña el resultado con «Deshacer»", async () => {
    const lista: VistaPrevia = {
      ...VISTA,
      juegoAbierto: false,
      plan: { ...VISTA.plan, personajes: VISTA.plan.personajes.map((p) => ({ ...p, conflictos: [] })) },
    };
    const { llamadas, irAHistorial } = pintar({
      abrir_paquete: () => ({ resultado: "abierta", resumen: RESUMEN }),
      plan_importacion: () => lista,
      aplicar_importacion: () => [
        { cuenta: "1#1", reino: "93", personaje: "Goldberg-Joestar", resultado: "completa", operacion: "op1", ficheros: 1 },
      ],
    });
    fireEvent.click(screen.getByRole("button", { name: /Abrir perfil/ }));
    fireEvent.click(await screen.findByRole("checkbox", { name: /Goldberg Joestar/ }));
    fireEvent.click(screen.getByRole("button", { name: "Siguiente" }));
    fireEvent.click(screen.getByRole("button", { name: "Siguiente" }));

    fireEvent.click(await screen.findByRole("button", { name: "Aplicar…" }));
    const dialogo = screen.getByRole("dialog");
    expect(within(dialogo).getByText("Goldberg Joestar: 1 fichero")).toBeTruthy();
    expect(llamadas.some((l) => l.comando === "aplicar_importacion")).toBe(false);
    fireEvent.click(within(dialogo).getByRole("button", { name: "Aplicar" }));

    expect(await screen.findByText("Hecho: 1 fichero escrito.")).toBeTruthy();
    expect(screen.getByRole("button", { name: "Deshacer" })).toBeTruthy();
    expect(screen.queryByRole("button", { name: "Atrás" })).toBeNull();
    const aplicar = llamadas.find((l) => l.comando === "aplicar_importacion");
    expect(aplicar?.argumentos).toMatchObject({ sha256: RESUMEN.sha256, huella: VISTA.huella });
    fireEvent.click(screen.getByRole("button", { name: "Ver historial" }));
    expect(irAHistorial).toHaveBeenCalledOnce();
  });

  it("si aplicar falla, lo dice y vuelve a pedir la vista previa", async () => {
    const lista: VistaPrevia = {
      ...VISTA,
      juegoAbierto: false,
      plan: { ...VISTA.plan, personajes: VISTA.plan.personajes.map((p) => ({ ...p, conflictos: [] })) },
    };
    const { llamadas } = pintar({
      abrir_paquete: () => ({ resultado: "abierta", resumen: RESUMEN }),
      plan_importacion: () => lista,
      aplicar_importacion: () => Promise.reject({ codigo: "planCaducado", datos: {} }),
    });
    fireEvent.click(screen.getByRole("button", { name: /Abrir perfil/ }));
    fireEvent.click(await screen.findByRole("checkbox", { name: /Goldberg Joestar/ }));
    fireEvent.click(screen.getByRole("button", { name: "Siguiente" }));
    fireEvent.click(screen.getByRole("button", { name: "Siguiente" }));
    fireEvent.click(await screen.findByRole("button", { name: "Aplicar…" }));
    const dialogo = screen.getByRole("dialog");
    fireEvent.click(within(dialogo).getByRole("button", { name: "Aplicar" }));

    expect((await within(dialogo).findByRole("alert")).textContent).toMatch(/Algún fichero ha cambiado/);
    fireEvent.click(within(dialogo).getByRole("button", { name: "Cancelar" }));
    await waitFor(() => expect(llamadas.filter((l) => l.comando === "plan_importacion")).toHaveLength(2));
    expect((await screen.findByRole("alert")).textContent).toMatch(/Algún fichero ha cambiado/);
  });

  it("dice por qué un paquete no vale y no deja nada abierto", async () => {
    pintar({
      abrir_paquete: () => {
        throw { codigo: "paqueteInvalido", datos: { motivo: "piezaDanada", detalle: "datos/macros" } };
      },
    });
    fireEvent.click(screen.getByRole("button", { name: /Abrir perfil/ }));
    const alerta = await screen.findByRole("alert");
    expect(alerta.textContent).toBe(
      "El fichero no es un perfil válido: uno de sus ficheros está dañado o se ha modificado.",
    );
    expect(screen.getByText(/Abre un fichero .wowprofile/)).toBeTruthy();
  });

  it("sin instalación de la versión del perfil lo dice y no ofrece destinos", async () => {
    const retail = { ...RESUMEN, manifiesto: { ...RESUMEN.manifiesto, origen: { ...RESUMEN.manifiesto.origen, producto: "wow" } } };
    pintar({ abrir_paquete: () => ({ resultado: "abierta", resumen: retail }) });
    fireEvent.click(screen.getByRole("button", { name: /Abrir perfil/ }));
    expect(await screen.findByText(/no hay ninguna instalación de esa versión/)).toBeTruthy();
    expect(screen.queryByRole("checkbox")).toBeNull();
  });

  it("al salir de la vista se cierra el paquete en el núcleo", async () => {
    const { llamadas, vista } = pintar({ abrir_paquete: () => ({ resultado: "cancelada" }) });
    fireEvent.click(screen.getByRole("button", { name: /Abrir perfil/ }));
    await waitFor(() => expect(llamadas.some((l) => l.comando === "abrir_paquete")).toBe(true));
    vista.unmount();
    expect(llamadas.some((l) => l.comando === "cerrar_paquete")).toBe(true);
  });
});
