// Textos de la interfaz (#9) con i18next. El inglés es la referencia: define las claves (ver `i18next.d.ts`) y es
// el idioma de reserva. Si al español le falta una clave, se ve la inglesa, nunca la clave.
import i18next, { type i18n, type Resource } from "i18next";
import { initReactI18next } from "react-i18next";
import en from "./en.json";
import es from "./es.json";
import { pedirIdioma } from "../nucleo";

/** Idiomas con textos. Cada uno tiene su `<idioma>.json` aquí y una variante en `src-tauri/src/idioma.rs`. */
export type Idioma = "es" | "en";

export const IDIOMA_RESERVA: Idioma = "en";

export const RECURSOS: Resource = { en: { translation: en }, es: { translation: es } };

/**
 * Crea una instancia de i18next para un idioma. Instancia propia y no la global: los tests crean las suyas sin
 * pisarse. Los recursos van dentro del bundle, así que la carga es síncrona (`initAsync: false`).
 */
export function crearTextos(idioma: Idioma, recursos: Resource = RECURSOS): i18n {
  const instancia = i18next.createInstance();
  void instancia.use(initReactI18next).init({
    lng: idioma,
    fallbackLng: IDIOMA_RESERVA,
    resources: recursos,
    initAsync: false,
    // React ya escapa lo que pinta; escapar aquí también mostraría `&amp;` y similares.
    interpolation: { escapeValue: false },
  });
  return instancia;
}

/** Pide el idioma al núcleo, prepara los textos y fija `<html lang>`. */
export async function iniciarTextos(): Promise<i18n> {
  const idioma = await pedirIdioma();
  document.documentElement.lang = idioma;
  return crearTextos(idioma);
}
