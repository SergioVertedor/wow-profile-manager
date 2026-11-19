// Aviso al cerrar la ventana con una operación escribiendo en el WTF (spec 029). Quien decide si hay que preguntar es
// el núcleo: mira el cerrojo de las operaciones, impide el cierre y manda el evento. Aquí solo se pregunta.
import { useEffect, useId, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { listen } from "@tauri-apps/api/event";
import { cerrarVentana, EVENTO_CIERRE_BLOQUEADO } from "../nucleo";
import estiloDialogo from "../estilo/Dialogo.module.css";
import { mensajeDeError } from "../textos/errores";

export default function AvisoCierre() {
  const { t } = useTranslation();
  const dialogo = useRef<HTMLDialogElement>(null);
  const [error, setError] = useState<unknown>(null);
  const idTitulo = useId();

  useEffect(() => {
    const dejarDeEscuchar = listen(EVENTO_CIERRE_BLOQUEADO, () => {
      setError(null);
      if (!dialogo.current?.open) {
        dialogo.current?.showModal();
      }
    });
    return () => {
      void dejarDeEscuchar.then((dejar) => dejar());
    };
  }, []);

  const cerrar = () => {
    setError(null);
    cerrarVentana().catch(setError);
  };

  return (
    <dialog ref={dialogo} className={estiloDialogo.dialogo} aria-labelledby={idTitulo}>
      <h2 id={idTitulo} className={estiloDialogo.titulo}>
        {t("cierre.titulo")}
      </h2>
      <p>{t("cierre.explicacion")}</p>
      {error !== null && (
        <p role="alert" className={estiloDialogo.alerta}>
          {mensajeDeError(t, error)}
        </p>
      )}
      <div className={estiloDialogo.acciones}>
        {/* Primero y con el foco: lo seguro es esperar a que termine. */}
        <button type="button" className={estiloDialogo.secundario} autoFocus onClick={() => dialogo.current?.close()}>
          {t("cierre.seguir")}
        </button>
        <button type="button" className={estiloDialogo.peligro} onClick={cerrar}>
          {t("cierre.cerrar")}
        </button>
      </div>
    </dialog>
  );
}
