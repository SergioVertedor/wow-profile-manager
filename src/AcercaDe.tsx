// Pantalla «Acerca de». El aviso de autoría sigue la forma del término 7(b) de `NOTICE.md` («…, de Sergio Vertedor —
// <repositorio>»); el «Basado en» lo añade quien distribuya una versión derivada.
import { useEffect, useId, useState, type MouseEvent } from "react";
import { useTranslation } from "react-i18next";
import css from "./AcercaDe.module.css";
import tazaKoFi from "./assets/kofi-taza.png";
import { abrirEnlace, cambiarActualizaciones, estadoActualizaciones, type Enlace, type InfoApp } from "./nucleo";
import { mensajeDeError } from "./textos/errores";

export default function AcercaDe({ info }: Readonly<{ info: InfoApp }>) {
  const { t } = useTranslation();
  const [error, setError] = useState<unknown>(null);

  function abrir(enlace: Enlace) {
    setError(null);
    abrirEnlace(enlace).catch(setError);
  }

  return (
    <section className={css.pantalla}>
      <h1 className={css.titulo}>{t("acercaDe.titulo")}</h1>
      <div className={css.pergamino}>
        <p className={css.version}>{t("cabecera", { nombre: info.nombre, version: info.version })}</p>
        <p>
          {t("acercaDe.autoria", { nombre: info.nombre, autor: info.autor })}
          <EnlaceExterno url={info.repositorio} enlace="repositorio" abrir={abrir} />
        </p>
        <p>
          {t("acercaDe.web")} <EnlaceExterno url={info.web} enlace="web" abrir={abrir} texto="tonotabi.dev" />
        </p>
        <p>
          {t("acercaDe.licencia")}{" "}
          <EnlaceExterno url={info.licencia} enlace="licencia" abrir={abrir} texto={t("acercaDe.verLicencia")} />
        </p>
        <p className={css.koFi}>
          <img src={tazaKoFi} alt="" className={css.tazaKoFi} />
          <span>
            {t("acercaDe.koFi")} <EnlaceExterno url={info.koFi} enlace="koFi" abrir={abrir} />
          </span>
        </p>
        <Actualizaciones alFallar={setError} />
        {error !== null && (
          <p role="alert" className={css.alerta}>
            {mensajeDeError(t, error)}
          </p>
        )}
      </div>
      <p className={css.marca}>{t("acercaDe.marca", { nombre: info.nombre })}</p>
    </section>
  );
}

/**
 * Un enlace que se abre en el navegador del sistema. El `href` está para que se vea y se pueda copiar; ningún clic
 * (tampoco el de la rueda) navega la ventana de la app: se pide al núcleo que abra el enlace por su nombre.
 */
function EnlaceExterno({
  url,
  enlace,
  abrir,
  texto = url,
}: Readonly<{ url: string; enlace: Enlace; abrir: (enlace: Enlace) => void; texto?: string }>) {
  function alPulsar(evento: MouseEvent<HTMLAnchorElement>) {
    evento.preventDefault();
    abrir(enlace);
  }
  return (
    <a href={url} onClick={alPulsar} onAuxClick={alPulsar}>
      {texto}
    </a>
  );
}

/** El interruptor «Buscar actualizaciones al arrancar» (spec 035). Hasta saber cómo está, desactivado. */
function Actualizaciones({ alFallar }: Readonly<{ alFallar: (error: unknown) => void }>) {
  const { t } = useTranslation();
  const idAyuda = useId();
  const [encendidas, setEncendidas] = useState<boolean | null>(null);

  useEffect(() => {
    estadoActualizaciones().then(setEncendidas).catch(alFallar);
  }, [alFallar]);

  function cambiar(valor: boolean) {
    setEncendidas(valor);
    cambiarActualizaciones(valor).catch((error: unknown) => {
      setEncendidas(!valor);
      alFallar(error);
    });
  }

  return (
    <div className={css.actualizaciones}>
      <label className={css.interruptor}>
        <input
          type="checkbox"
          role="switch"
          checked={encendidas ?? false}
          disabled={encendidas === null}
          aria-describedby={idAyuda}
          onChange={(evento) => cambiar(evento.target.checked)}
        />
        {t("acercaDe.buscarActualizaciones")}
      </label>
      <p id={idAyuda} className={css.ayuda}>
        {t("acercaDe.ayudaActualizaciones")}
      </p>
    </div>
  );
}
