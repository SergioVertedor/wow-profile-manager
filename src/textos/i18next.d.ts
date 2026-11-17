// Tipado de las claves: `t("clave")` solo compila si la clave existe en `en.json`, y las marcas `{{dato}}` se
// comprueban contra lo que se pasa.
import "i18next";
import type en from "./en.json";

declare module "i18next" {
  interface CustomTypeOptions {
    defaultNS: "translation";
    resources: { translation: typeof en };
  }
}
