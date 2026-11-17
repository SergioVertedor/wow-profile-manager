// Pantalla «Personajes» (spec 017), al estilo de la selección de personaje del juego: selector de versión arriba, el
// personaje elegido en el centro y la lista de la cuenta a la derecha. Solo enseña lo que devuelve el núcleo.
import { useEffect, useState } from "react";
import type { TFunction } from "i18next";
import { useTranslation } from "react-i18next";
import { TriangleAlert } from "lucide-react";
import { pedirInstalaciones, type Cuenta, type Instalacion, type Personaje, type VersionJuego } from "../nucleo";
import { mensajeDeError } from "../textos/errores";
import Ficha from "./Ficha";
import css from "./Personajes.module.css";
import Plantel from "./Plantel";
import { numerosDeCuenta } from "./lista";
import {
  contarPersonajes,
  entradasDeVersion,
  resolverEleccion,
  SIN_ELEGIR,
  type Eleccion,
  type EntradaVersion,
} from "./seleccion";

type Estado =
  | { tipo: "cargando" }
  | { tipo: "listo"; instalaciones: Instalacion[] }
  | { tipo: "error"; error: unknown };

export default function Personajes() {
  const { t } = useTranslation();
  const [estado, setEstado] = useState<Estado>({ tipo: "cargando" });

  useEffect(() => {
    pedirInstalaciones()
      .then((instalaciones) => setEstado({ tipo: "listo", instalaciones }))
      .catch((error: unknown) => setEstado({ tipo: "error", error }));
  }, []);

  if (estado.tipo !== "listo") {
    return (
      <section className={css.pantalla}>
        <h1 className={css.titulo}>{t("personajes.titulo")}</h1>
        {estado.tipo === "cargando" && <p className={css.nota}>{t("personajes.buscando")}</p>}
        {estado.tipo === "error" && (
          <p role="alert" className={css.alerta}>
            {mensajeDeError(t, estado.error)}
          </p>
        )}
      </section>
    );
  }
  return <Seleccion entradas={entradasDeVersion(estado.instalaciones)} />;
}

function Seleccion({ entradas }: Readonly<{ entradas: EntradaVersion[] }>) {
  const { t } = useTranslation();
  const [eleccion, setEleccion] = useState<Eleccion>(SIN_ELEGIR);
  const elegido = resolverEleccion(entradas, eleccion);

  if (elegido === null) {
    return (
      <section className={css.pantalla}>
        <h1 className={css.titulo}>{t("personajes.titulo")}</h1>
        <p className={css.nota}>{t("personajes.ninguna")}</p>
      </section>
    );
  }
  const { entrada, cuenta, personaje } = elegido;
  return (
    <section className={css.pantalla}>
      <header className={css.cabecera}>
        <h1 className={css.titulo}>{t("personajes.titulo")}</h1>
        <SelectorVersion
          entradas={entradas}
          actual={entrada.clave}
          elegir={(clave) => setEleccion({ ...SIN_ELEGIR, version: clave })}
        />
      </header>
      <Avisos version={entrada.version} />
      {cuenta !== null && (
        <Cuerpo
          version={entrada.version}
          cuenta={cuenta}
          personaje={personaje}
          cambiar={(cambio) => setEleccion({ ...eleccion, version: entrada.clave, ...cambio })}
        />
      )}
      <Pie entrada={entrada} />
    </section>
  );
}

function Cuerpo({
  version,
  cuenta,
  personaje,
  cambiar,
}: Readonly<{
  version: VersionJuego;
  cuenta: Cuenta;
  personaje: Personaje | null;
  cambiar: (cambio: Partial<Eleccion>) => void;
}>) {
  const { t } = useTranslation();
  const cuentas = version.cuentas.cuentas;
  const numeros = numerosDeCuenta(cuentas.map((c) => c.carpeta));
  const etiquetaCuenta = (carpeta: string) => {
    const numero = numeros.get(carpeta) ?? null;
    return numero === null ? carpeta : t("plantel.cuenta", { numero });
  };
  return (
    <div className={css.cuerpo}>
      <Ficha personaje={personaje} version={version} cuenta={etiquetaCuenta(cuenta.carpeta)} />
      <Plantel
        key={cuenta.carpeta}
        cuentas={cuentas}
        cuenta={cuenta}
        producto={version.producto}
        etiquetaCuenta={etiquetaCuenta}
        elegirCuenta={(carpeta) => cambiar({ cuenta: carpeta, personaje: null })}
        personaje={personaje}
        elegirPersonaje={(clave) => cambiar({ cuenta: cuenta.carpeta, personaje: clave })}
      />
    </div>
  );
}

function SelectorVersion({
  entradas,
  actual,
  elegir,
}: Readonly<{ entradas: EntradaVersion[]; actual: string; elegir: (clave: string) => void }>) {
  const { t } = useTranslation();
  return (
    <div className={css.versiones} role="group" aria-label={t("personajes.versiones")}>
      {entradas.map((entrada) => (
        <button
          key={entrada.clave}
          type="button"
          aria-pressed={entrada.clave === actual}
          title={entrada.instalacion.raiz}
          onClick={() => elegir(entrada.clave)}
        >
          {nombreDeProducto(t, entrada.version.producto)}{" "}
          <span className={css.recuento}>{contarPersonajes(entrada.version)}</span>
        </button>
      ))}
    </div>
  );
}

/** Por qué una versión no tiene personajes que exportar: ámbar y con icono, para que no dependa solo del color. */
function Avisos({ version }: Readonly<{ version: VersionJuego }>) {
  const { t } = useTranslation();
  const avisos = [
    !version.soportada && t("personajes.noSoportada"),
    version.soportada && version.interfaz === null && t("personajes.sinInterfaz"),
    version.soportada && version.cuentas.cuentas.length === 0 && t("personajes.sinCuentas"),
  ].filter((aviso) => aviso !== false);
  return avisos.map((aviso) => (
    <p key={aviso} className={css.aviso}>
      <TriangleAlert aria-hidden size={16} strokeWidth={1.75} />
      {aviso}
    </p>
  ));
}

/** De dónde sale la versión y lo que se ha saltado al leerla. */
function Pie({ entrada }: Readonly<{ entrada: EntradaVersion }>) {
  const { t } = useTranslation();
  const { instalacion, version } = entrada;
  return (
    <footer className={css.pie}>
      <p>
        {instalacion.raiz} — {t(`personajes.fuente.${instalacion.fuente}`)}
      </p>
      {version.cuentas.ignoradas > 0 && (
        <p>{t("personajes.cuentasIgnoradas", { count: version.cuentas.ignoradas })}</p>
      )}
      {version.soportada && version.addons.ignoradas > 0 && (
        <p>{t("personajes.addonsIgnorados", { count: version.addons.ignoradas })}</p>
      )}
    </footer>
  );
}

/** Nombre legible de los productos conocidos; los demás, tal cual vienen de `.flavor.info`. */
function nombreDeProducto(t: TFunction, producto: string): string {
  return producto === "wow" || producto === "wowf" ? t(`productos.${producto}`) : producto;
}
