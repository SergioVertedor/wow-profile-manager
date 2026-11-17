// Pantalla «Acerca de». El aviso de autoría sigue la forma del término 7(b) de `NOTICE.md` («…, de Sergio Vertedor —
// <repositorio>»); el «Basado en» lo añade quien distribuya una versión derivada.
import { useState, type MouseEvent } from "react";
import { useTranslation } from "react-i18next";
import { abrirEnlace, type Enlace, type InfoApp } from "./nucleo";
import { mensajeDeError } from "./textos/errores";

export default function AcercaDe({ info }: Readonly<{ info: InfoApp }>) {
  const { t } = useTranslation();
  const [error, setError] = useState<unknown>(null);

  function abrir(enlace: Enlace) {
    setError(null);
    abrirEnlace(enlace).catch(setError);
  }

  return (
    <section>
      <h2>{t("acercaDe.titulo")}</h2>
      <p>{t("cabecera", { nombre: info.nombre, version: info.version })}</p>
      <p>
        {t("acercaDe.autoria", { nombre: info.nombre, autor: info.autor })}
        <EnlaceExterno url={info.repositorio} enlace="repositorio" abrir={abrir} />
      </p>
      <p>
        {t("acercaDe.licencia")}{" "}
        <EnlaceExterno url={info.licencia} enlace="licencia" abrir={abrir} texto={t("acercaDe.verLicencia")} />
      </p>
      <p>
        {t("acercaDe.koFi")} <EnlaceExterno url={info.koFi} enlace="koFi" abrir={abrir} />
      </p>
      <p>{t("acercaDe.marca", { nombre: info.nombre })}</p>
      {error !== null && <p role="alert">{mensajeDeError(t, error)}</p>}
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
