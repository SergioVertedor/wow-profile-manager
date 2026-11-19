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
    { id: "macros", categoria: "macros", tipo: "fichero", ambito: "personaje", deCuenta: false, bytes: 2_000, marcada: true, generica: false },
    { id: "macros-cuenta", categoria: "macros", tipo: "fichero", ambito: "cuenta", deCuenta: true, bytes: 200, marcada: true, generica: false },
    {
      id: "dominos",
      categoria: "config-addon",
      tipo: "fichero",
      ambito: "personaje",
      deCuenta: false,
      bytes: 1,
      marcada: false,
      generica: true,
      addon: "Dominos",
      incompatible: "piezaNoValida",
    },
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
        conflictosPerfil: [],
        noAplicables: [],
        ficheros: [
          {
            entrada: "macros",
            entradas: ["macros"],
            categoria: "macros",
            destino: "personaje/macros-cache.txt",
            deCuenta: false,
            grupo: "modificado",
            accion: "sobrescribir",
            elementos: [{ nombre: "Foco", cambio: "cambiado", antes: "Viejo", despues: "Foco" }],
            apartadas: [],
            avisos: [],
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
    expect(screen.getByText(/podría llevar código/)).toBeTruthy();
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
      pedido: { seleccion: ["macros", "macros-cuenta"], cuentas: [], perfiles: [] },
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
      perfiles: [],
    });
  });

  it("cada entrada que se importa dice qué trae, también en las decisiones de cuenta", async () => {
    const resumen: ResumenPaquete = {
      ...RESUMEN,
      entradas: [
        { ...RESUMEN.entradas[0], contenido: { tipo: "lista", elementos: ["Foco", "#showtooltip"] } },
        { ...RESUMEN.entradas[1], contenido: { tipo: "lista", elementos: ["Monta"] } },
        {
          id: "addons-activos",
          categoria: "addons-activos",
          ambito: "personaje",
          deCuenta: false,
          bytes: 90,
          tipo: "fichero-filtrado",
          marcada: true,
          generica: false,
          contenido: { tipo: "addons", activados: ["Dominos", "Details"], desactivados: ["WeakAuras"] },
        },
        {
          id: "ventanas",
          categoria: "ventanas",
          tipo: "fichero",
          ambito: "personaje",
          deCuenta: false,
          bytes: 30,
          marcada: true,
          generica: false,
        },
        RESUMEN.entradas[2],
      ],
    };
    pintar({ abrir_paquete: () => ({ resultado: "abierta", resumen }) });
    fireEvent.click(screen.getByRole("button", { name: /Abrir perfil/ }));
    fireEvent.click(await screen.findByRole("checkbox", { name: /Goldberg Joestar/ }));
    fireEvent.click(screen.getByRole("button", { name: "Siguiente" }));

    // Macros, macros de la cuenta, addons y ventanas; la configuración de addons no se importa y no dice nada.
    const queTrae = screen.getAllByText("Qué trae");
    expect(queTrae).toHaveLength(5);
    expect(screen.getAllByText("Foco")).toHaveLength(1);
    expect(screen.getByText("Se activan 2")).toBeTruthy();
    expect(screen.getByText("Se desactiva 1")).toBeTruthy();
    expect(screen.getByText("WeakAuras")).toBeTruthy();
    expect(screen.getByText(/^Dónde has colocado las ventanas/)).toBeTruthy();
    const decision = screen.getByRole("group", { name: /Macros de la cuenta en la cuenta 1#1/ });
    // El lector de pantalla oye de qué entrada es cada «Qué trae».
    expect(within(decision).getByLabelText("Qué trae: Macros de la cuenta")).toBeTruthy();
    expect(screen.getByLabelText("Qué trae: Addons activos")).toBeTruthy();
    expect(within(decision).getByText("Monta")).toBeTruthy();
  });

  it("la vista previa agrupa lo apartado por motivo; los addons que faltan llevan su propia frase", async () => {
    const fichero = VISTA.plan.personajes[0].ficheros[0];
    const conApartadas: VistaPrevia = {
      ...VISTA,
      plan: {
        ...VISTA.plan,
        personajes: [
          {
            ...VISTA.plan.personajes[0],
            ficheros: [
              {
                ...fichero,
                entrada: "opciones",
                categoria: "opciones",
                apartadas: [
                  { motivo: "seguridad", claves: ["AllowDangerousScripts"] },
                  { motivo: "estado", claves: ["trackedQuests"] },
                ],
              },
              {
                ...fichero,
                entrada: "addons-activos",
                categoria: "addons-activos",
                apartadas: [{ motivo: "addonNoInstalado", claves: ["Dominos"] }],
              },
            ],
          },
        ],
      },
    };
    pintar({
      abrir_paquete: () => ({ resultado: "abierta", resumen: RESUMEN }),
      plan_importacion: () => conApartadas,
    });
    fireEvent.click(screen.getByRole("button", { name: /Abrir perfil/ }));
    fireEvent.click(await screen.findByRole("checkbox", { name: /Goldberg Joestar/ }));
    fireEvent.click(screen.getByRole("button", { name: "Siguiente" }));
    fireEvent.click(screen.getByRole("button", { name: "Siguiente" }));

    expect(await screen.findByText(/^Por seguridad:/)).toBeTruthy();
    expect(screen.getByText(/^Son estado de la partida/)).toBeTruthy();
    expect(screen.getByText("trackedQuests")).toBeTruthy();
    expect(screen.getByText("Se activan, pero no están instalados en este equipo.")).toBeTruthy();
    expect(screen.getByText("Dominos")).toBeTruthy();
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

  it("el Modo Edición se importa sin decidir la cuenta y la vista previa dice qué diseño se añade y se usa", async () => {
    const resumen: ResumenPaquete = {
      ...RESUMEN,
      entradas: [
        {
          id: "modo-edicion",
          categoria: "modo-edicion",
          tipo: "adaptador",
          ambito: "cuenta",
          deCuenta: false,
          bytes: 2_700,
          marcada: true,
          generica: false,
          perfil: "Sanador",
        },
      ],
    };
    const fichero = VISTA.plan.personajes[0].ficheros[0];
    const vista: VistaPrevia = {
      ...VISTA,
      juegoAbierto: false,
      plan: {
        incompatibles: [],
        personajes: [
          {
            ...VISTA.plan.personajes[0],
            conflictos: [],
            ficheros: [
              {
                ...fichero,
                entrada: "modo-edicion",
                entradas: ["modo-edicion"],
                categoria: "modo-edicion",
                destino: "cuenta/edit-mode-cache-account.txt",
                accion: "fusionar",
                elementos: [{ nombre: "Sanador", cambio: "anadido", clase: "diseno" }],
              },
              {
                ...fichero,
                entrada: "modo-edicion",
                entradas: ["modo-edicion"],
                categoria: "modo-edicion",
                destino: "personaje/edit-mode-cache-character.txt",
                accion: "fusionar",
                elementos: [{ nombre: "Sanador", cambio: "cambiado", clase: "disenoActivo" }],
              },
            ],
          },
        ],
      },
    };
    pintar({ abrir_paquete: () => ({ resultado: "abierta", resumen }), plan_importacion: () => vista });
    fireEvent.click(screen.getByRole("button", { name: /Abrir perfil/ }));
    fireEvent.click(await screen.findByRole("checkbox", { name: /Goldberg Joestar/ }));
    fireEvent.click(screen.getByRole("button", { name: "Siguiente" }));

    const casilla = screen.getByRole("checkbox", { name: /^Modo Edición/ });
    expect(casilla.closest("label")?.textContent).not.toMatch(/de la cuenta/);
    // No hay que decidir nada por cuenta: no sale ninguna decisión.
    expect(screen.queryAllByRole("radio")).toHaveLength(0);
    fireEvent.click(screen.getByText("Qué trae"));
    expect(screen.getByText(/El diseño «Sanador» del Modo Edición/)).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Siguiente" }));

    expect(await screen.findByText("Diseño «Sanador» del Modo Edición")).toBeTruthy();
    expect(screen.getByText("Pasa a usar el diseño «Sanador»")).toBeTruthy();
  });

  it("la configuración de addons va por addon, con los genéricos desmarcados, y un perfil que existe se decide", async () => {
    const deAddon = { categoria: "config-addon", ambito: "cuenta", deCuenta: false, bytes: 50, addon: "Dominos" } as const;
    const resumen: ResumenPaquete = {
      ...RESUMEN,
      entradas: [
        RESUMEN.entradas[0],
        {
          ...deAddon,
          id: "addon-dominos-perfil",
          tipo: "perfil-acedb",
          variable: "DominosDB",
          perfil: "@@WOWPROFILE:PERSONAJE@@",
          marcada: true,
          generica: false,
        },
        { ...deAddon, id: "addon-dominos-cuenta", tipo: "fichero", deCuenta: true, marcada: false, generica: true },
      ],
    };
    const conflicto = { entrada: "addon-dominos-perfil", perfil: "Default", propuesto: "Default (2)", usadoPor: ["Otro Pj"] };
    const vista: VistaPrevia = {
      ...VISTA,
      juegoAbierto: false,
      plan: {
        incompatibles: [],
        personajes: [
          {
            ...VISTA.plan.personajes[0],
            conflictos: [],
            conflictosPerfil: [conflicto],
            noAplicables: [{ entrada: "addon-dominos-perfil", motivo: "sinNombre" }],
            ficheros: [
              {
                ...VISTA.plan.personajes[0].ficheros[0],
                entrada: "addon-dominos-perfil",
                entradas: ["addon-dominos-perfil"],
                categoria: "config-addon",
                destino: "cuenta/SavedVariables/Dominos.lua",
                elementos: [{ nombre: "Goldberg Joestar", cambio: "cambiado", antes: "Default", despues: "Tanque", clase: "perfilDe" }],
                avisos: [{ tipo: "faltaAddon", addon: "Dominos" }],
              },
            ],
          },
        ],
      },
    };
    const { llamadas } = pintar({
      abrir_paquete: () => ({ resultado: "abierta", resumen }),
      plan_importacion: () => vista,
    });
    fireEvent.click(screen.getByRole("button", { name: /Abrir perfil/ }));
    fireEvent.click(await screen.findByRole("checkbox", { name: /Goldberg Joestar/ }));
    fireEvent.click(screen.getByRole("button", { name: "Siguiente" }));

    const dominos = screen.getByRole("group", { name: "Dominos" });
    const perfil = within(dominos).getByRole<HTMLInputElement>("checkbox", { name: /Perfil con el nombre del personaje/ });
    const fichero = within(dominos).getByRole<HTMLInputElement>("checkbox", { name: /Fichero de la cuenta/ });
    expect({ perfil: perfil.checked, fichero: fichero.checked }).toEqual({ perfil: true, fichero: false });
    // El aviso del fichero entero está ligado a su casilla para el lector de pantalla.
    expect(fichero.getAttribute("aria-describedby")).toBeTruthy();
    expect(within(dominos).getByText(/Fichero entero/)).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Siguiente" }));

    expect(await screen.findByText(/ya hay un perfil «Default» distinto/)).toBeTruthy();
    expect(screen.getByText(/También lo usan Otro Pj/)).toBeTruthy();
    // Sobrescribir lleva el aviso de quién más lo usa como descripción para el lector de pantalla.
    expect(screen.getByRole("radio", { name: "Sobrescribir «Default»", description: /También lo usan Otro Pj/ })).toBeTruthy();
    expect(screen.getByText("Perfil de Goldberg Joestar")).toBeTruthy();
    expect(screen.getByText(/Dominos no está instalado aquí/)).toBeTruthy();
    expect(screen.getByText(/no se sabe cómo se llama este personaje/)).toBeTruthy();
    expect(screen.getByText(/Decide antes lo que falta/)).toBeTruthy();

    fireEvent.click(screen.getByRole("radio", { name: "Importarlo como «Default (2)»" }));
    await waitFor(() => expect(llamadas.filter((l) => l.comando === "plan_importacion")).toHaveLength(2));
    const planes = llamadas.filter((l) => l.comando === "plan_importacion");
    const ultimo = planes[planes.length - 1];
    expect((ultimo?.argumentos as { pedido: unknown }).pedido).toEqual({
      seleccion: ["macros", "addon-dominos-perfil"],
      cuentas: [],
      perfiles: [{ cuenta: "1#1", entrada: "addon-dominos-perfil", accion: "renombrar" }],
    });
  });
});
