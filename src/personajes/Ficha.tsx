// El personaje elegido, en el centro y con un foco de luz: la pieza llamativa de la app (spec 017). El retrato es un
// monograma: el de la API de Blizzard se descartó (#50), porque sus condiciones piden una clave secreta que no cabe en
// una app de repositorio público.
import type { TFunction } from "i18next";
import { useTranslation } from "react-i18next";
import type { Addon, Personaje, VersionJuego } from "../nucleo";
import EliminarPersonaje from "./EliminarPersonaje";
import ExportarPersonaje from "./ExportarPersonaje";
import css from "./Ficha.module.css";
import { claveDePersonaje, inicial, muestraReinos, nombreVisible } from "./lista";

type Props = Readonly<{
  personaje: Personaje | null;
  version: VersionJuego;
  cuenta: string;
  instalacion: string;
  /** Dónde está en disco, para pedirle al núcleo que lo exporte o lo elimine. */
  raiz: string;
  carpetaCuenta: string;
  /** Tras eliminar su carpeta: vuelve a leer los personajes. */
  alEliminar: () => Promise<void>;
}>;

/** La instalación va siempre: dos personajes con la misma carpeta en dos instalaciones son personajes distintos. */
export default function Ficha({ personaje, version, cuenta, instalacion, raiz, carpetaCuenta, alEliminar }: Props) {
  const { t } = useTranslation();
  // Una cuenta sin personajes: el escenario queda vacío y la lista dice por qué.
  if (personaje === null) {
    return <div className={css.escenario} aria-hidden />;
  }
  const nombre = nombreVisible(personaje);
  const { addons } = version.addons;
  const reino = muestraReinos(version.producto) ? [personaje.carpetaReino] : [];
  // Cada hermano con su propia clave: si se repiten, React no sabe cuál quitar y se acumulan retratos (#58).
  const clave = claveDePersonaje(personaje);
  const pedido = {
    raiz,
    version: version.carpeta,
    cuenta: carpetaCuenta,
    reino: personaje.carpetaReino,
    personaje: personaje.carpetaPersonaje,
  };
  const donde = [...reino, cuenta, instalacion];
  return (
    <section className={css.escenario} aria-label={t("ficha.etiqueta")}>
      <div key={`retrato-${clave}`} className={css.retrato} aria-hidden>
        {inicial(nombre)}
      </div>
      <h2 className={css.nombre}>{nombre}</h2>
      <p className={css.sub}>
        {donde.map((parte) => (
          <span key={parte}>{parte}</span>
        ))}
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
      <ExportarPersonaje key={`exportar-${clave}`} pedido={pedido} nombre={nombre} donde={donde} />
      <EliminarPersonaje
        key={`eliminar-${clave}`}
        pedido={pedido}
        nombre={nombre}
        donde={donde}
        alTerminar={alEliminar}
      />
    </section>
  );
}

/** El título del `.toc` o, si no trae, la carpeta; con la versión si la hay. */
function nombreDeAddon(t: TFunction, addon: Addon): string {
  const titulo = addon.titulo ?? addon.carpeta;
  return addon.version === null ? titulo : t("personajes.addon", { titulo, version: addon.version });
}
