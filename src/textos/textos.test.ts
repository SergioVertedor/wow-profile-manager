import { afterEach, describe, expect, it } from "vitest";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import en from "./en.json";
import es from "./es.json";
import { crearTextos, iniciarTextos } from ".";

afterEach(() => clearMocks());

describe("textos", () => {
  it("traduce al idioma pedido", () => {
    expect(crearTextos("es").t("cargando")).toBe("Cargando…");
    expect(crearTextos("en").t("cargando")).toBe("Loading…");
  });

  it("si al español le falta la clave, usa el inglés y nunca la clave", () => {
    const textos = crearTextos("es", { en: { translation: en }, es: { translation: {} } });
    expect(textos.t("cargando")).toBe("Loading…");
  });

  it("sustituye las marcas por sus datos", () => {
    expect(crearTextos("en").t("cabecera", { nombre: "App", version: "1.2.3" })).toBe("App 1.2.3");
  });

  it("no escapa lo que interpola: de eso se encarga React", () => {
    expect(crearTextos("en").t("errorNucleo", { mensaje: "a & b" })).toBe("Could not reach the core: a & b");
  });

  it("el español no tiene claves que no existan en inglés", () => {
    const sobrantes = Object.keys(es).filter((clave) => !(clave in en));
    expect(sobrantes).toEqual([]);
  });
});

describe("iniciarTextos", () => {
  it("usa el idioma del núcleo y lo fija en <html lang>", async () => {
    mockIPC((comando) => (comando === "idioma_interfaz" ? "es" : undefined));

    const textos = await iniciarTextos();

    expect(textos.language).toBe("es");
    expect(document.documentElement.lang).toBe("es");
  });

  it("si el núcleo no responde, arranca en inglés", async () => {
    mockIPC(() => {
      throw new Error("sin núcleo");
    });

    const textos = await iniciarTextos();

    expect(textos.t("cargando")).toBe("Loading…");
    expect(document.documentElement.lang).toBe("en");
  });
});
