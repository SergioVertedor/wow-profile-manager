// Lista de instalaciones → versiones → cuentas → personajes. Solo enseña lo que devuelve el núcleo: no decide nada.
import { useEffect, useState } from "react";
import type { TFunction } from "i18next";
import { useTranslation } from "react-i18next";
import { pedirInstalaciones, type Cuenta, type Instalacion, type VersionJuego } from "./nucleo";
import { mensajeDeError } from "./textos/errores";

type Estado =
  | { tipo: "cargando" }
  | { tipo: "listo"; instalaciones: Instalacion[] }
  | { tipo: "error"; error: unknown };

export default function Instalaciones() {
  const { t } = useTranslation();
  const [estado, setEstado] = useState<Estado>({ tipo: "cargando" });

  useEffect(() => {
    pedirInstalaciones()
      .then((instalaciones) => setEstado({ tipo: "listo", instalaciones }))
      .catch((error: unknown) => setEstado({ tipo: "error", error }));
  }, []);

  return (
    <section>
      <h2>{t("instalaciones.titulo")}</h2>
      {estado.tipo === "cargando" && <p>{t("instalaciones.buscando")}</p>}
      {estado.tipo === "error" && <p role="alert">{mensajeDeError(t, estado.error)}</p>}
      {estado.tipo === "listo" && <ListaInstalaciones instalaciones={estado.instalaciones} />}
    </section>
  );
}

function ListaInstalaciones({ instalaciones }: Readonly<{ instalaciones: Instalacion[] }>) {
  const { t } = useTranslation();
  if (instalaciones.length === 0) {
    return <p>{t("instalaciones.ninguna")}</p>;
  }
  return instalaciones.map((instalacion) => (
    <article key={instalacion.raiz}>
      <h3>{instalacion.raiz}</h3>
      <p>{t(`instalaciones.fuente.${instalacion.fuente}`)}</p>
      {instalacion.versiones.map((version) => (
        <BloqueVersion key={version.carpeta} version={version} />
      ))}
    </article>
  ));
}

function BloqueVersion({ version }: Readonly<{ version: VersionJuego }>) {
  const { t } = useTranslation();
  const { cuentas, ignoradas } = version.cuentas;
  return (
    <section>
      <h4>{t("instalaciones.version", { producto: nombreDeProducto(t, version.producto), carpeta: version.carpeta })}</h4>
      {version.build !== null && <p>{t("instalaciones.build", { build: version.build })}</p>}
      {!version.soportada && <p>{t("instalaciones.noSoportada")}</p>}
      {version.soportada && version.interfaz === null && <p>{t("instalaciones.sinInterfaz")}</p>}
      {version.soportada && cuentas.length === 0 && <p>{t("instalaciones.sinCuentas")}</p>}
      {ignoradas > 0 && <p>{t("instalaciones.cuentasIgnoradas", { count: ignoradas })}</p>}
      {cuentas.map((cuenta) => (
        <BloqueCuenta key={cuenta.carpeta} cuenta={cuenta} />
      ))}
    </section>
  );
}

function BloqueCuenta({ cuenta }: Readonly<{ cuenta: Cuenta }>) {
  const { t } = useTranslation();
  return (
    <section>
      <h5>{t("instalaciones.cuenta", { carpeta: cuenta.carpeta, count: cuenta.personajes.length })}</h5>
      {cuenta.reinosIgnorados > 0 && (
        <p>{t("instalaciones.reinosIgnorados", { count: cuenta.reinosIgnorados })}</p>
      )}
      <ul>
        {cuenta.personajes.map((p) => (
          <li key={`${p.carpetaReino}/${p.carpetaPersonaje}`}>
            {t("instalaciones.personaje", { personaje: p.carpetaPersonaje, reino: p.carpetaReino })}
          </li>
        ))}
      </ul>
    </section>
  );
}

/** Nombre legible de los productos conocidos; los demás, tal cual vienen de `.flavor.info`. */
function nombreDeProducto(t: TFunction, producto: string): string {
  return producto === "wow" || producto === "wowf" ? t(`productos.${producto}`) : producto;
}
