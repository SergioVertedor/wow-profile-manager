// Ficha del personaje elegido: la pieza llamativa de la app. El retrato es un monograma hasta que la API de Blizzard
// traiga uno de verdad (otra issue).
import { useTranslation } from "react-i18next";
import css from "./MarcoPersonaje.module.css";
import { calidadDeNombre } from "./estilo/calidad";
import type { Personaje } from "./nucleo";

/** `clave` identifica al personaje entre instalaciones, versiones y cuentas: dos carpetas iguales no se confunden. */
export type Elegido = Readonly<{ clave: string; personaje: Personaje; producto: string; cuenta: string }>;

export default function MarcoPersonaje({ elegido }: Readonly<{ elegido: Elegido | null }>) {
  const { t } = useTranslation();
  if (elegido === null) {
    return (
      <section className={`${css.marco} ${css.vacio}`} aria-label={t("marco.etiqueta")}>
        <p>{t("marco.vacio")}</p>
      </section>
    );
  }
  const { personaje, producto, cuenta } = elegido;
  const nombre = personaje.nombreEnDatos;
  const calidad = calidadDeNombre(nombre);
  const carpeta = `${personaje.carpetaReino}/${personaje.carpetaPersonaje}`;
  return (
    <section className={css.marco} aria-label={t("marco.etiqueta")} data-calidad={calidad}>
      <div className={css.retrato} aria-hidden>
        {inicial(nombre?.texto)}
      </div>
      <div className={css.datos}>
        <h3 className={css.nombre} data-calidad={calidad}>
          {nombre?.texto ?? personaje.carpetaPersonaje}
        </h3>
        <p className={css.origen}>{t(`marco.origen.${nombre?.origen ?? "sinResolver"}`)}</p>
        <dl className={css.detalles}>
          <dt>{t("marco.version")}</dt>
          <dd>{producto}</dd>
          <dt>{t("marco.cuenta")}</dt>
          <dd>{cuenta}</dd>
          <dt>{t("marco.carpeta")}</dt>
          <dd>{carpeta}</dd>
        </dl>
      </div>
    </section>
  );
}

/** La primera letra completa (sin partir pares sustitutos); «?» si no hay nombre. */
function inicial(texto: string | undefined): string {
  return Array.from(texto ?? "")[0] ?? "?";
}
