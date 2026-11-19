import { afterEach, describe, expect, it } from "vitest";
import { cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { I18nextProvider } from "react-i18next";
import { useDeteccion } from "../instalaciones/useDeteccion";
import type { Addon, Cuenta, Deteccion, Personaje, VersionJuego } from "../nucleo";
import { crearTextos, type Idioma } from "../textos";
import Personajes from "./Personajes";

afterEach(() => {
  cleanup();
  clearMocks();
});

function pintar(idioma: Idioma, respuesta: () => unknown) {
  mockIPC((comando) => (comando === "detectar_instalaciones" ? respuesta() : undefined));
  render(
    <I18nextProvider i18n={crearTextos(idioma)}>
      <ConDeteccion />
    </I18nextProvider>,
  );
}

/** Como en `App`: la detección la da el hook y Personajes la recibe. */
function ConDeteccion() {
  return <Personajes {...useDeteccion()} />;
}

function pj(carpetaReino: string, carpetaPersonaje: string, texto: string | null = null): Personaje {
  return { carpetaReino, carpetaPersonaje, nombreEnDatos: texto === null ? null : { texto, origen: "datos" } };
}

function addon(carpeta: string, titulo: string | null, version: string | null): Addon {
  return { carpeta, toc: `${carpeta}.toc`, titulo, version, enlace: null, dependencias: [], svCuenta: [], svPersonaje: [] };
}

const FOREVER: VersionJuego = {
  carpeta: "_forever_",
  producto: "wowf",
  soportada: true,
  build: "1.60.1.70938",
  interfaz: 16001,
  cuentas: {
    ignoradas: 1,
    cuentas: [
      {
        carpeta: "107710745#1",
        nombre: null,
        reinosIgnorados: 2,
        personajes: [
          pj("93", "Goldberg-Joestar", "Goldberg Joestar"),
          pj("93", "Goldberg-Sp", "Goldberg Sp"),
          pj("93", "Goldber8D31D"),
        ],
      },
    ],
  },
  addons: { addons: [addon("APR", "Azeroth Pilot Reloaded", "v7.7.1"), addon("SinTitulo", null, null)], ignoradas: 1 },
};

const RETAIL_CUENTA_1: Cuenta = {
  carpeta: "107710745#1",
  nombre: null,
  reinosIgnorados: 0,
  personajes: [
    pj("Dun Modr", "Allagan"),
    pj("Dun Modr", "Apañerouno"),
    pj("C'Thun", "Angharad"),
    ...Array.from({ length: 9 }, (_, i) => pj("C'Thun", `Relleno${i}`)),
  ],
};

const RETAIL: VersionJuego = {
  ...FOREVER,
  carpeta: "_retail_",
  producto: "wow",
  build: "12.1.5.70961",
  cuentas: { ignoradas: 0, cuentas: [RETAIL_CUENTA_1, { carpeta: "107710745#2", nombre: null, reinosIgnorados: 0, personajes: [pj("Uldum", "Mordekaiser")] }] },
  addons: { addons: [], ignoradas: 0 },
};

const SIN_NADA: Deteccion = { instalaciones: [], noEncontradas: [], preferencias: null };

function instalacion(...versiones: VersionJuego[]): Deteccion {
  return {
    ...SIN_NADA,
    instalaciones: [{ raiz: "C:\\Juegos\\World of Warcraft", fuente: "registro", nombre: null, versiones }],
  };
}

const ficha = () => screen.getByRole("region", { name: /Personaje elegido|Selected character/ });
const lista = () => screen.getByRole("region", { name: /Personajes de la cuenta|Characters in the account/ });
const versiones = () => screen.getByRole("group", { name: /Versión del juego|Game version/ });

describe("Personajes", () => {
  it("elige sin preguntar el primer personaje de la primera versión con personajes", async () => {
    pintar("es", () => instalacion(FOREVER, RETAIL));

    await screen.findByRole("heading", { level: 1, name: "Personajes" });
    expect(within(ficha()).getByRole("heading", { name: "Goldberg Joestar" })).toBeDefined();
    expect(within(lista()).getByRole("button", { name: "Goldberg Joestar" }).getAttribute("aria-pressed")).toBe("true");
    expect(within(versiones()).getByRole("button", { name: "WoW Forever 3" }).getAttribute("aria-pressed")).toBe("true");
    expect(within(versiones()).getByRole("button", { name: "World of Warcraft (Retail) 13" })).toBeDefined();
  });

  it("en Forever no hay reinos: ni en la ficha ni en la lista", async () => {
    pintar("es", () => instalacion(FOREVER));

    await screen.findByRole("heading", { name: "Goldberg Joestar" });
    expect(within(ficha()).queryByText("93")).toBeNull();
    expect(within(ficha()).getByText("Cuenta 1")).toBeDefined();
    expect(within(lista()).queryByRole("heading", { name: /93/ })).toBeNull();
  });

  it("un nombre sin resolver se enseña por su carpeta, sin etiqueta de origen", async () => {
    pintar("es", () => instalacion(FOREVER));

    fireEvent.click(await screen.findByRole("button", { name: "Goldber8D31D" }));

    expect(within(ficha()).getByRole("heading", { name: "Goldber8D31D" })).toBeDefined();
    expect(screen.queryByText(/deducido|sin resolver/)).toBeNull();
  });

  it("elegir otro personaje desmarca el anterior y cambia la ficha", async () => {
    pintar("es", () => instalacion(FOREVER));

    fireEvent.click(await screen.findByRole("button", { name: "Goldberg Sp" }));

    expect(within(lista()).getAllByRole("button", { pressed: true }).map((b) => b.textContent)).toEqual(["GGoldberg Sp"]);
    expect(within(ficha()).getByRole("heading", { name: "Goldberg Sp" })).toBeDefined();
  });

  // #58: el retrato y el botón de eliminar compartían `key`; al cambiar de personaje se acumulaban retratos.
  it("al cambiar varias veces de personaje, la ficha tiene un solo retrato", async () => {
    pintar("es", () => instalacion(FOREVER));
    await screen.findByRole("button", { name: "Goldberg Sp" });
    const personajes = within(lista()).getAllByRole("button", { pressed: false });

    for (const boton of [...personajes, ...personajes]) {
      fireEvent.click(boton);
    }

    expect(within(ficha()).getAllByText(/^\p{Lu}$/u)).toHaveLength(1);
  });

  it("con una sola cuenta no hay desplegable; dice su carpeta y cuántos personajes tiene", async () => {
    pintar("es", () => instalacion(FOREVER));

    await screen.findByRole("heading", { name: "Goldberg Joestar" });
    expect(within(lista()).getByRole("heading", { name: "Cuenta 1" })).toBeDefined();
    expect(screen.queryByRole("combobox")).toBeNull();
    expect(within(lista()).getByText("107710745#1")).toBeDefined();
    expect(within(lista()).getByText("3 personajes")).toBeDefined();
  });

  it("al cambiar de versión, se elige el primer personaje de su primera cuenta", async () => {
    pintar("en", () => instalacion(FOREVER, RETAIL));

    fireEvent.click(within(await screen.findByRole("group", { name: "Game version" })).getByRole("button", { name: /Retail/ }));

    expect(within(ficha()).getByRole("heading", { name: "Allagan" })).toBeDefined();
    expect(within(ficha()).getByText("Dun Modr")).toBeDefined();
    expect(within(ficha()).getByText("Account 1")).toBeDefined();
  });

  it("con varias cuentas, el desplegable cambia la lista y elige el primero de la nueva", async () => {
    pintar("en", () => instalacion(RETAIL));

    const cuenta = await screen.findByRole("combobox", { name: "Account" });
    expect(within(cuenta).getAllByRole("option").map((o) => o.textContent)).toEqual(["Account 1", "Account 2"]);

    fireEvent.change(cuenta, { target: { value: "107710745#2" } });

    expect(within(ficha()).getByRole("heading", { name: "Mordekaiser" })).toBeDefined();
    expect(within(ficha()).getByText("Account 2")).toBeDefined();
    expect(within(lista()).getByText("1 character")).toBeDefined();
  });

  it("fuera de Forever agrupa por reino, con su recuento", async () => {
    pintar("en", () => instalacion(RETAIL));

    await screen.findByRole("heading", { name: "Allagan" });
    const reinos = within(lista()).getAllByRole("heading", { level: 3 }).map((h) => h.textContent);
    expect(reinos).toEqual(["Dun Modr2", "C'Thun10"]);
    expect(within(screen.getByRole("region", { name: "Dun Modr" })).getAllByRole("listitem").map((b) => b.textContent)).toEqual([
      "AAllagan",
      "AApañerouno",
    ]);
  });

  it("la cabecera de un reino lo pliega y lo despliega, y el recuento se sigue viendo", async () => {
    pintar("es", () => instalacion(RETAIL));

    const cabecera = await screen.findByRole("button", { name: "Dun Modr2", expanded: true });
    fireEvent.click(cabecera);

    expect(cabecera.getAttribute("aria-expanded")).toBe("false");
    expect(within(lista()).queryByRole("button", { name: "Allagan" })).toBeNull();
    expect(within(lista()).getByRole("button", { name: "Angharad" })).toBeDefined();
    expect(document.getElementById(cabecera.getAttribute("aria-controls") ?? "")?.hidden).toBe(true);

    fireEvent.click(cabecera);
    expect(within(lista()).getByRole("button", { name: "Allagan" })).toBeDefined();
  });

  it("plegar el reino del elegido no cambia la elección", async () => {
    pintar("es", () => instalacion(RETAIL));

    fireEvent.click(await screen.findByRole("button", { name: "Dun Modr2" }));

    expect(within(ficha()).getByRole("heading", { name: "Allagan" })).toBeDefined();
  });

  it("mientras se busca, un reino plegado enseña sus resultados", async () => {
    pintar("es", () => instalacion(RETAIL));

    fireEvent.click(await screen.findByRole("button", { name: "Dun Modr2" }));
    fireEvent.change(screen.getByRole("searchbox", { name: "Buscar personaje" }), { target: { value: "apañ" } });

    expect(within(lista()).getAllByRole("listitem").map((b) => b.textContent)).toEqual(["AApañerouno"]);
    expect(screen.getByRole("button", { name: "Dun Modr1" }).getAttribute("aria-expanded")).toBe("true");
  });

  it("con más de 10 personajes hay buscador: sin mayúsculas ni acentos, y avisa si no hay nada", async () => {
    pintar("es", () => instalacion(RETAIL));

    const buscar = await screen.findByRole("searchbox", { name: "Buscar personaje" });
    fireEvent.change(buscar, { target: { value: "APANE" } });
    expect(within(lista()).getAllByRole("listitem").map((b) => b.textContent)).toEqual(["AApañerouno"]);

    fireEvent.change(buscar, { target: { value: "zz" } });
    expect(within(lista()).getByText("Ningún personaje coincide con «zz».")).toBeDefined();
  });

  it("con 10 personajes o menos no hay buscador", async () => {
    pintar("es", () => instalacion(FOREVER));

    await screen.findByRole("heading", { name: "Goldberg Joestar" });
    expect(screen.queryByRole("searchbox")).toBeNull();
  });

  it("la ficha dice addons y build, y los addons se despliegan", async () => {
    pintar("es", () => instalacion(FOREVER));

    await screen.findByRole("heading", { name: "Goldberg Joestar" });
    expect(within(ficha()).getByText("1.60.1.70938")).toBeDefined();
    expect(within(ficha()).getByText("Ver los 2 addons instalados")).toBeDefined();
    expect(within(ficha()).getByText("Azeroth Pilot Reloaded · v7.7.1")).toBeDefined();
    expect(within(ficha()).getByText("SinTitulo")).toBeDefined();
  });

  it("dice los reinos saltados, con su plural; la raíz y las carpetas saltadas van a Diagnóstico", async () => {
    pintar("es", () => instalacion(FOREVER));

    await screen.findByRole("heading", { name: "Goldberg Joestar" });
    expect(screen.getByText("2 carpetas de reino son de la otra versión del juego y se han saltado.")).toBeDefined();
    // Spec 029: el pie de diagnóstico ya no está, aunque haya carpetas saltadas.
    expect(screen.queryByText(/WTF\/Account/)).toBeNull();
    expect(screen.queryByText(/Interface\/AddOns/)).toBeNull();
    expect(screen.queryByText(/C:\\Juegos\\World of Warcraft/)).toBeNull();
  });

  it("una versión no compatible se puede elegir y dice por qué no hay personajes", async () => {
    const clasica: VersionJuego = {
      ...FOREVER,
      carpeta: "_classic_",
      producto: "wow_classic",
      soportada: false,
      cuentas: { ignoradas: 0, cuentas: [] },
    };
    pintar("en", () => instalacion(FOREVER, clasica));

    fireEvent.click(within(await screen.findByRole("group", { name: "Game version" })).getByRole("button", { name: "wow_classic 0" }));

    expect(screen.getByText("This app does not support this game version.")).toBeDefined();
    expect(screen.queryByRole("region", { name: "Selected character" })).toBeNull();
    expect(screen.queryByText(/Interface\/AddOns/)).toBeNull();
  });

  it("avisa de una versión sin interfaz y de una sin cuentas", async () => {
    const sinInterfaz: VersionJuego = { ...FOREVER, interfaz: null };
    const sinCuentas: VersionJuego = { ...FOREVER, carpeta: "_retail_", producto: "wow", cuentas: { ignoradas: 0, cuentas: [] } };
    pintar("en", () => instalacion(sinInterfaz, sinCuentas));

    expect(await screen.findByText("Open the game once with this version: until then it cannot be exported.")).toBeDefined();

    fireEvent.click(within(versiones()).getByRole("button", { name: /Retail/ }));
    expect(screen.getByText("No accounts yet: log in to the game once.")).toBeDefined();
  });

  it("una cuenta sin personajes lo dice en la lista", async () => {
    const vacia: VersionJuego = {
      ...FOREVER,
      cuentas: { ignoradas: 0, cuentas: [{ carpeta: "1#1", nombre: null, reinosIgnorados: 0, personajes: [] }] },
    };
    pintar("es", () => instalacion(vacia));

    expect(await screen.findByText("Esta cuenta aún no tiene personajes: entra al juego con uno.")).toBeDefined();
    expect(screen.queryByRole("region", { name: "Personaje elegido" })).toBeNull();
  });

  it("con varias instalaciones, cada versión y la ficha dicen de cuál son", async () => {
    pintar("es", () => ({
      ...SIN_NADA,
      instalaciones: [
        { raiz: "C:\\WoW", fuente: "registro", nombre: "Casa", versiones: [FOREVER] },
        { raiz: "E:\\WoW", fuente: "manual", nombre: null, versiones: [RETAIL] },
      ],
    }));

    await screen.findByRole("group", { name: "Versión del juego" });
    expect(within(versiones()).getByRole("button", { name: "Casa · WoW Forever 3" })).toBeDefined();
    expect(within(versiones()).getByRole("button", { name: "Instalación 2 · World of Warcraft (Retail) 13" })).toBeDefined();
    expect(within(ficha()).getByText("Casa")).toBeDefined();
  });

  it("con una sola instalación, la versión va sin su nombre pero la ficha lo dice", async () => {
    pintar("es", () => instalacion(FOREVER));

    await screen.findByRole("group", { name: "Versión del juego" });
    expect(within(versiones()).getByRole("button", { name: "WoW Forever 3" })).toBeDefined();
    expect(within(ficha()).getByText("Instalación 1")).toBeDefined();
  });

  it("renombra la cuenta desde la lista sin perder el personaje elegido", async () => {
    const llamadas: { comando: string; argumentos: unknown }[] = [];
    mockIPC((comando, argumentos) => {
      llamadas.push({ comando, argumentos });
      return comando === "detectar_instalaciones" ? instalacion(RETAIL) : null;
    });
    render(
      <I18nextProvider i18n={crearTextos("es")}>
        <ConDeteccion />
      </I18nextProvider>,
    );

    await screen.findByRole("region", { name: "Personajes de la cuenta" });
    fireEvent.click(within(lista()).getByRole("button", { name: "Angharad" }));
    fireEvent.click(within(lista()).getByRole("button", { name: "Cambiar: Nombre de la cuenta" }));
    const campo = within(lista()).getByRole("textbox", { name: "Nombre de la cuenta" });
    expect(campo.getAttribute("placeholder")).toBe("Cuenta 1");
    fireEvent.change(campo, { target: { value: " Principal " } });
    fireEvent.submit(campo);

    expect(await within(ficha()).findByText("Principal")).toBeDefined();
    expect(within(lista()).getByRole("option", { name: "Principal" })).toBeDefined();
    expect(within(ficha()).getByRole("heading", { name: "Angharad" })).toBeDefined();
    expect(llamadas.find((l) => l.comando === "nombrar_cuenta")?.argumentos).toEqual({
      raiz: "C:\\Juegos\\World of Warcraft",
      version: "_retail_",
      cuenta: "107710745#1",
      nombre: " Principal ",
    });
    expect(llamadas.filter((l) => l.comando === "detectar_instalaciones")).toHaveLength(1);
  });

  it("si no hay ninguna instalación, lo dice", async () => {
    pintar("en", () => SIN_NADA);

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
