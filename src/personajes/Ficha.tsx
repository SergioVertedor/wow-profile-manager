// El personaje elegido, en el centro y con un foco de luz: la pieza llamativa de la app (spec 017). El retrato es un
// monograma hasta que la API de Blizzard traiga uno de verdad (otra issue).
import type { TFunction } from "i18next";
import { useTranslation } from "react-i18next";
import type { Addon, Personaje, VersionJuego } from "../nucleo";
import css from "./Ficha.module.css";
import { claveDePersonaje, inicial, muestraReinos, nombreVisible } from "./lista";

type Props = Readonly<{ personaje: Personaje | null; version: VersionJuego; cuenta: string }>;

export default function Ficha({ personaje, version, cuenta }: Props) {
  const { t } = useTranslation();
  // Una cuenta sin personajes: el escenario queda vacío y la lista dice por qué.
  if (personaje === null) {
    return <div className={css.escenario} aria-hidden />;
  }
  const nombre = nombreVisible(personaje);
  const { addons } = version.addons;
  return (
    <section className={css.escenario} aria-label={t("ficha.etiqueta")}>
      <div key={claveDePersonaje(personaje)} className={css.retrato} aria-hidden>
        {inicial(nombre)}
      </div>
      <h2 className={css.nombre}>{nombre}</h2>
      <p className={css.sub}>
        {muestraReinos(version.producto) && <span>{personaje.carpetaReino}</span>}
        <span>{cuenta}</span>
      </p>
      <dl className={css.datos}>
        <div>
          <dt>{t("personajes.addons")}</dt>
          <dd>{addons.length}</dd>
        </div>
        {version.build !== null && (
          <div>
            <dt>{t("personajes.build")}</dt>
            <dd>{version.build}</dd>
          </div>
        )}
      </dl>
      {addons.length > 0 && (
        <details className={css.addons}>
          <summary>{t("personajes.verAddons", { count: addons.length })}</summary>
          <ul>
            {addons.map((addon) => (
              <li key={addon.carpeta}>{nombreDeAddon(t, addon)}</li>
            ))}
          </ul>
        </details>
      )}
    </section>
  );
}

/** El título del `.toc` o, si no trae, la carpeta; con la versión si la hay. */
function nombreDeAddon(t: TFunction, addon: Addon): string {
  const titulo = addon.titulo ?? addon.carpeta;
  return addon.version === null ? titulo : t("personajes.addon", { titulo, version: addon.version });
}
