// Lista de instalaciones → versiones → cuentas → personajes. Solo enseña lo que devuelve el núcleo: no decide nada.
import { useEffect, useState } from "react";
import type { TFunction } from "i18next";
import { Trans, useTranslation } from "react-i18next";
import { TriangleAlert } from "lucide-react";
import { calidadDeNombre } from "./estilo/calidad";
import css from "./Instalaciones.module.css";
import MarcoPersonaje, { type Elegido } from "./MarcoPersonaje";
import { pedirInstalaciones, type Addon, type Cuenta, type Instalacion, type Personaje, type VersionJuego } from "./nucleo";
import { mensajeDeError } from "./textos/errores";

type Estado =
  | { tipo: "cargando" }
  | { tipo: "listo"; instalaciones: Instalacion[] }
  | { tipo: "error"; error: unknown };

/** Lo que hace falta para elegir un personaje y saber cuál está elegido. */
type Eleccion = Readonly<{ elegido: Elegido | null; elegir: (elegido: Elegido) => void }>;

export default function Instalaciones() {
  const { t } = useTranslation();
  const [estado, setEstado] = useState<Estado>({ tipo: "cargando" });
  const [elegido, setElegido] = useState<Elegido | null>(null);

  useEffect(() => {
    pedirInstalaciones()
      .then((instalaciones) => setEstado({ tipo: "listo", instalaciones }))
      .catch((error: unknown) => setEstado({ tipo: "error", error }));
  }, []);

  return (
    <section className={css.pantalla}>
      <h2 className={css.titulo}>{t("instalaciones.titulo")}</h2>
      {estado.tipo === "cargando" && <p className={css.nota}>{t("instalaciones.buscando")}</p>}
      {estado.tipo === "error" && (
        <p role="alert" className={css.alerta}>
          {mensajeDeError(t, estado.error)}
        </p>
      )}
      {estado.tipo === "listo" && estado.instalaciones.length > 0 && <MarcoPersonaje key={elegido?.clave} elegido={elegido} />}
      {estado.tipo === "listo" && (
        <ListaInstalaciones instalaciones={estado.instalaciones} eleccion={{ elegido, elegir: setElegido }} />
      )}
    </section>
  );
}

function ListaInstalaciones({ instalaciones, eleccion }: Readonly<{ instalaciones: Instalacion[]; eleccion: Eleccion }>) {
  const { t } = useTranslation();
  if (instalaciones.length === 0) {
    return <p className={css.nota}>{t("instalaciones.ninguna")}</p>;
  }
  return instalaciones.map((instalacion) => (
    <article key={instalacion.raiz} className={css.instalacion}>
      <header>
        <h3 className={css.raiz}>{instalacion.raiz}</h3>
        <p className={css.nota}>{t(`instalaciones.fuente.${instalacion.fuente}`)}</p>
      </header>
      {instalacion.versiones.map((version) => (
        <BloqueVersion key={version.carpeta} raiz={instalacion.raiz} version={version} eleccion={eleccion} />
      ))}
    </article>
  ));
}

function BloqueVersion({
  raiz,
  version,
  eleccion,
}: Readonly<{ raiz: string; version: VersionJuego; eleccion: Eleccion }>) {
  const { t } = useTranslation();
  const { cuentas, ignoradas } = version.cuentas;
  const producto = nombreDeProducto(t, version.producto);
  return (
    <section className={css.version}>
      <header className={css.cabeceraVersion}>
        <h4>{t("instalaciones.version", { producto, carpeta: version.carpeta })}</h4>
        {version.build !== null && <p className={css.nota}>{t("instalaciones.build", { build: version.build })}</p>}
      </header>
      {!version.soportada && <Aviso texto={t("instalaciones.noSoportada")} />}
      {version.soportada && version.interfaz === null && <Aviso texto={t("instalaciones.sinInterfaz")} />}
      {version.soportada && cuentas.length === 0 && <Aviso texto={t("instalaciones.sinCuentas")} />}
      {ignoradas > 0 && <p className={css.nota}>{t("instalaciones.cuentasIgnoradas", { count: ignoradas })}</p>}
      {cuentas.map((cuenta) => (
        <BloqueCuenta
          key={cuenta.carpeta}
          ambito={`${raiz}|${version.carpeta}|${cuenta.carpeta}`}
          cuenta={cuenta}
          producto={producto}
          eleccion={eleccion}
        />
      ))}
      {version.soportada && <BloqueAddons addons={version.addons} />}
    </section>
  );
}

/** `ambito` (instalación, versión y cuenta) más la carpeta del personaje dan su clave única. */
function BloqueCuenta({
  ambito,
  cuenta,
  producto,
  eleccion,
}: Readonly<{ ambito: string; cuenta: Cuenta; producto: string; eleccion: Eleccion }>) {
  const { t } = useTranslation();
  return (
    <section className={css.cuenta}>
      <h5 className={css.tituloCuenta}>
        {t("instalaciones.cuenta", { carpeta: cuenta.carpeta, count: cuenta.personajes.length })}
      </h5>
      {cuenta.reinosIgnorados > 0 && (
        <p className={css.nota}>{t("instalaciones.reinosIgnorados", { count: cuenta.reinosIgnorados })}</p>
      )}
      <ul className={css.filas}>
        {cuenta.personajes.map((personaje) => {
          const carpeta = `${personaje.carpetaReino}/${personaje.carpetaPersonaje}`;
          const clave = `${ambito}|${carpeta}`;
          return (
            <li key={carpeta}>
              <FilaPersonaje
                personaje={personaje}
                elegido={eleccion.elegido?.clave === clave}
                elegir={() => eleccion.elegir({ clave, personaje, producto, cuenta: cuenta.carpeta })}
              />
            </li>
          );
        })}
      </ul>
    </section>
  );
}

/** El nombre con el color de su calidad y su carpeta; si se dedujo de la carpeta o no se resolvió, lo dice. */
function FilaPersonaje({
  personaje,
  elegido,
  elegir,
}: Readonly<{ personaje: Personaje; elegido: boolean; elegir: () => void }>) {
  const carpeta = `${personaje.carpetaReino}/${personaje.carpetaPersonaje}`;
  const nombre = personaje.nombreEnDatos;
  const calidad = calidadDeNombre(nombre);
  // El nombre viene de los SavedVariables: va como nodo, nunca interpolado dentro del marcado de la traducción.
  const marcaNombre = (
    <span className={css.nombre} data-calidad={calidad}>
      {nombre?.texto}
    </span>
  );
  return (
    <button type="button" className={css.fila} aria-pressed={elegido} data-calidad={calidad} onClick={elegir}>
      {nombre === null ? (
        <Trans i18nKey="instalaciones.personaje.sinResolver" values={{ carpeta }} components={{ nombre: marcaNombre }} />
      ) : (
        <Trans
          i18nKey={`instalaciones.personaje.${nombre.origen}`}
          values={{ carpeta }}
          components={{ nombre: marcaNombre }}
        />
      )}
    </button>
  );
}

/** Un aviso: color ámbar y un icono, para que no dependa solo del color. */
function Aviso({ texto }: Readonly<{ texto: string }>) {
  return (
    <p className={css.aviso}>
      <TriangleAlert aria-hidden size={16} strokeWidth={1.75} />
      {texto}
    </p>
  );
}

/** Plegado: una versión puede traer más de 100 addons. */
function BloqueAddons({ addons }: Readonly<{ addons: VersionJuego["addons"] }>) {
  const { t } = useTranslation();
  return (
    <details className={css.addons}>
      <summary>{t("instalaciones.addons", { count: addons.addons.length })}</summary>
      {addons.ignoradas > 0 && <p className={css.nota}>{t("instalaciones.addonsIgnorados", { count: addons.ignoradas })}</p>}
      <ul className={css.listaAddons}>
        {addons.addons.map((addon) => (
          <li key={addon.carpeta}>{nombreDeAddon(t, addon)}</li>
        ))}
      </ul>
    </details>
  );
}

/** El título del `.toc` o, si no trae, la carpeta; con la versión si la hay. */
function nombreDeAddon(t: TFunction, addon: Addon): string {
  const titulo = addon.titulo ?? addon.carpeta;
  return addon.version === null ? titulo : t("instalaciones.addon", { titulo, version: addon.version });
}

/** Nombre legible de los productos conocidos; los demás, tal cual vienen de `.flavor.info`. */
function nombreDeProducto(t: TFunction, producto: string): string {
  return producto === "wow" || producto === "wowf" ? t(`productos.${producto}`) : producto;
}
