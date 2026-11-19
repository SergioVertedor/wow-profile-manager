// Pestaña «Diagnóstico» (spec 029): lo que la app va haciendo, en vivo, y el interruptor que lo guarda además en un
// `.txt`. Es para encontrar fallos, no para el uso normal: por eso vive aparte de las vistas del jugador.
import { useEffect, useId, useRef, useState, type UIEvent } from "react";
import { useTranslation } from "react-i18next";
import { FolderOpen, ShieldAlert } from "lucide-react";
import { abrirCarpetaRegistros, type LineaDiagnostico } from "../nucleo";
import { mensajeDeError } from "../textos/errores";
import css from "./Diagnostico.module.css";
import { useDiagnostico } from "./useDiagnostico";

/** Distancia al final, en px, por debajo de la cual la consola se considera «al final» y sigue bajando sola. */
const MARGEN_FINAL = 24;

const CLASE_NIVEL: Record<LineaDiagnostico["nivel"], string> = {
  ERROR: css.error,
  WARN: css.aviso,
  INFO: css.info,
};

export default function Diagnostico() {
  const { t } = useTranslation();
  const { lineas, interruptor, error, cambiar } = useDiagnostico();
  const [errorCarpeta, setErrorCarpeta] = useState<unknown>(null);
  const idAyuda = useId();
  const guarda = interruptor.tipo !== "cargando" && interruptor.guarda;

  const abrirCarpeta = () => {
    setErrorCarpeta(null);
    abrirCarpetaRegistros().catch(setErrorCarpeta);
  };

  return (
    <section className={css.pantalla}>
      <h1 className={css.titulo}>{t("diagnostico.titulo")}</h1>
      <p className={css.nota}>{t("diagnostico.explicacion")}</p>
      <div className={css.opciones}>
        <label className={css.interruptor}>
          <input
            type="checkbox"
            role="switch"
            checked={guarda}
            disabled={interruptor.tipo !== "listo"}
            aria-describedby={idAyuda}
            onChange={(evento) => cambiar(evento.target.checked)}
          />
          {t("diagnostico.guardar")}
        </label>
        {guarda && (
          <button type="button" className={css.accion} onClick={abrirCarpeta}>
            <FolderOpen aria-hidden size={14} strokeWidth={1.75} />
            {t("diagnostico.abrirCarpeta")}
          </button>
        )}
      </div>
      <p id={idAyuda} className={css.privacidad}>
        <ShieldAlert aria-hidden size={16} strokeWidth={1.75} />
        {t("diagnostico.privacidad")}
      </p>
      {error !== null && (
        <p role="alert" className={css.alerta}>
          {mensajeDeError(t, error)}
        </p>
      )}
      {errorCarpeta !== null && (
        <p role="alert" className={css.alerta}>
          {mensajeDeError(t, errorCarpeta)}
        </p>
      )}
      <Consola lineas={lineas} />
    </section>
  );
}

/** Las líneas, de la más antigua a la más nueva. Baja sola con cada línea nueva salvo si el usuario ha subido a leer. */
function Consola({ lineas }: Readonly<{ lineas: LineaDiagnostico[] }>) {
  const { t } = useTranslation();
  const consola = useRef<HTMLOListElement>(null);
  const alFinal = useRef(true);

  useEffect(() => {
    const elemento = consola.current;
    if (elemento && alFinal.current) {
      elemento.scrollTop = elemento.scrollHeight;
    }
  }, [lineas]);

  const alDesplazar = (evento: UIEvent<HTMLOListElement>) => {
    const { scrollTop, scrollHeight, clientHeight } = evento.currentTarget;
    alFinal.current = scrollHeight - scrollTop - clientHeight < MARGEN_FINAL;
  };

  if (lineas.length === 0) {
    return <p className={css.nota}>{t("diagnostico.vacio")}</p>;
  }
  return (
    <ol ref={consola} className={css.consola} role="log" aria-label={t("diagnostico.consola")} onScroll={alDesplazar}>
      {lineas.map((linea) => (
        <li key={linea.n} className={CLASE_NIVEL[linea.nivel]}>
          <span className={css.instante}>{linea.instante}</span> <span className={css.nivel}>{linea.nivel}</span>{" "}
          <span className={css.modulo}>[{linea.modulo}]</span> {linea.mensaje}
        </li>
      ))}
    </ol>
  );
}
