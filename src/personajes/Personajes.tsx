// Pantalla «Personajes» (#45), al estilo de la selección de personaje del juego: selector de versión arriba, el
// personaje elegido en el centro y la lista de la cuenta a la derecha. Solo enseña lo que devuelve el núcleo. Con más
// de una instalación, cada versión lleva el nombre de la suya (#34).
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { TriangleAlert } from "lucide-react";
import { conNombreDeCuenta, etiquetaDeInstalacion, nombreDeProducto, normalizarNombre } from "../instalaciones/nombres";
import type { UsoDeteccion } from "../instalaciones/useDeteccion";
import { nombrarCuenta, type Cuenta, type Deteccion, type Personaje, type VersionJuego } from "../nucleo";
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

export default function Personajes({ estado, actualizar, recargar }: UsoDeteccion) {
  const { t } = useTranslation();

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
  return <Seleccion deteccion={estado.deteccion} actualizar={actualizar} recargar={recargar} />;
}

type PropsSeleccion = Readonly<{
  deteccion: Deteccion;
  actualizar: UsoDeteccion["actualizar"];
  recargar: UsoDeteccion["recargar"];
}>;

function Seleccion({ deteccion, actualizar, recargar }: PropsSeleccion) {
  const { t } = useTranslation();
  const [eleccion, setEleccion] = useState<Eleccion>(SIN_ELEGIR);
  const entradas = entradasDeVersion(deteccion.instalaciones);
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
  const instalacion = etiquetaDeInstalacion(t, deteccion, entrada.instalacion);
  // Con una sola instalación su nombre no distingue nada: solo se antepone cuando hay varias.
  const etiquetaDeEntrada = (otra: EntradaVersion) => {
    const version = nombreDeProducto(t, otra.version.producto);
    return deteccion.instalaciones.length > 1
      ? t("personajes.versionDeInstalacion", {
          instalacion: etiquetaDeInstalacion(t, deteccion, otra.instalacion),
          version,
        })
      : version;
  };
  const renombrarCuenta = (carpeta: string) => (nombre: string) => {
    const donde = { raiz: entrada.instalacion.raiz, version: entrada.version.carpeta, cuenta: carpeta };
    return nombrarCuenta(donde.raiz, donde.version, donde.cuenta, nombre).then(() =>
      actualizar((d) => conNombreDeCuenta(d, donde, normalizarNombre(nombre))),
    );
  };
  return (
    <section className={css.pantalla}>
      <header className={css.cabecera}>
        <h1 className={css.titulo}>{t("personajes.titulo")}</h1>
        <SelectorVersion
          entradas={entradas}
          actual={entrada.clave}
          etiqueta={etiquetaDeEntrada}
          elegir={(clave) => setEleccion({ ...SIN_ELEGIR, version: clave })}
        />
      </header>
      <Avisos version={entrada.version} />
      {cuenta !== null && (
        <Cuerpo
          raiz={entrada.instalacion.raiz}
          recargar={recargar}
          instalacion={instalacion}
          version={entrada.version}
          cuenta={cuenta}
          personaje={personaje}
          cambiar={(cambio) => setEleccion({ ...eleccion, version: entrada.clave, ...cambio })}
          renombrarCuenta={renombrarCuenta(cuenta.carpeta)}
        />
      )}
    </section>
  );
}

function Cuerpo({
  raiz,
  recargar,
  instalacion,
  version,
  cuenta,
  personaje,
  cambiar,
  renombrarCuenta,
}: Readonly<{
  raiz: string;
  recargar: () => Promise<void>;
  instalacion: string;
  version: VersionJuego;
  cuenta: Cuenta;
  personaje: Personaje | null;
  cambiar: (cambio: Partial<Eleccion>) => void;
  renombrarCuenta: (nombre: string) => Promise<void>;
}>) {
  const { t } = useTranslation();
  const cuentas = version.cuentas.cuentas;
  const numeros = numerosDeCuenta(cuentas.map((c) => c.carpeta));
  const porDefecto = (carpeta: string) => {
    const numero = numeros.get(carpeta) ?? null;
    return numero === null ? carpeta : t("plantel.cuenta", { numero });
  };
  const etiquetaCuenta = (carpeta: string) => cuentas.find((c) => c.carpeta === carpeta)?.nombre ?? porDefecto(carpeta);
  return (
    <div className={css.cuerpo}>
      <Ficha
        personaje={personaje}
        version={version}
        cuenta={etiquetaCuenta(cuenta.carpeta)}
        instalacion={instalacion}
        raiz={raiz}
        carpetaCuenta={cuenta.carpeta}
        alEliminar={recargar}
      />
      <Plantel
        key={cuenta.carpeta}
        cuentas={cuentas}
        cuenta={cuenta}
        producto={version.producto}
        etiquetaCuenta={etiquetaCuenta}
        cuentaPorDefecto={porDefecto(cuenta.carpeta)}
        renombrarCuenta={renombrarCuenta}
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
  etiqueta,
  elegir,
}: Readonly<{
  entradas: EntradaVersion[];
  actual: string;
  etiqueta: (entrada: EntradaVersion) => string;
  elegir: (clave: string) => void;
}>) {
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
          {etiqueta(entrada)} <span className={css.recuento}>{contarPersonajes(entrada.version)}</span>
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
