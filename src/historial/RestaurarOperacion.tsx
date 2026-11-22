// «Restaurar» una operación sin resolver (#63): una importación que se cortó a mitad (no se
// restaura sola al arrancar, se pregunta) o un reintento de una que no se pudo restaurar del todo. Devuelve, desde sus
// copias, los ficheros que la operación llegó a cambiar; lo que no llegó a tocar no se toca.
import { useId, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { RotateCcw } from "lucide-react";
import { restaurarOperacion } from "../nucleo";
import estiloDialogo from "../estilo/Dialogo.module.css";
import { mensajeDeError } from "../textos/errores";
import css from "./Historial.module.css";

type Props = Readonly<{
  id: string;
  nombre: string;
  /** Producto y cuenta, ya con sus etiquetas. */
  donde: string[];
  /** Se llama al cerrar tras intentarlo: la lista y los personajes pueden haber cambiado. */
  alTerminar: () => void;
}>;

type Paso =
  | { tipo: "preguntando" }
  | { tipo: "restaurando" }
  | { tipo: "hecho" }
  | { tipo: "error"; error: unknown };

export default function RestaurarOperacion({ id, nombre, donde, alTerminar }: Props) {
  const { t } = useTranslation();
  const dialogo = useRef<HTMLDialogElement>(null);
  const [paso, setPaso] = useState<Paso>({ tipo: "preguntando" });
  const idTitulo = useId();
  const restaurando = paso.tipo === "restaurando";
  const terminado = paso.tipo === "hecho" || paso.tipo === "error";

  const abrir = () => {
    setPaso({ tipo: "preguntando" });
    dialogo.current?.showModal();
  };
  const confirmar = () => {
    setPaso({ tipo: "restaurando" });
    restaurarOperacion(id)
      .then(() => setPaso({ tipo: "hecho" }))
      .catch((error: unknown) => setPaso({ tipo: "error", error }));
  };
  const cerrar = () => dialogo.current?.close();
  const alCerrar = () => {
    if (terminado) {
      alTerminar();
    }
  };

  return (
    <>
      <button type="button" className={css.accion} onClick={abrir}>
        <RotateCcw aria-hidden size={14} strokeWidth={1.75} />
        {t("historial.restaurar")}
      </button>
      <dialog
        ref={dialogo}
        className={estiloDialogo.dialogo}
        aria-labelledby={idTitulo}
        onClose={alCerrar}
        onCancel={(evento) => restaurando && evento.preventDefault()}
      >
        <h2 id={idTitulo} className={estiloDialogo.titulo}>
          {t("restaurar.titulo", { nombre })}
        </h2>
        <p className={estiloDialogo.donde}>{donde.join(" · ")}</p>
        {(paso.tipo === "preguntando" || restaurando) && <p>{t("restaurar.explicacion")}</p>}
        {paso.tipo === "hecho" && (
          <p role="status" className={estiloDialogo.exito}>
            {t("restaurar.hecho")}
          </p>
        )}
        {paso.tipo === "error" && (
          <p role="alert" className={estiloDialogo.alerta}>
            {mensajeDeError(t, paso.error)}
          </p>
        )}
        <div className={estiloDialogo.acciones}>
          {terminado ? (
            <button type="button" className={estiloDialogo.secundario} onClick={cerrar}>
              {t("restaurar.cerrar")}
            </button>
          ) : (
            <>
              <button type="button" className={estiloDialogo.secundario} onClick={cerrar} disabled={restaurando}>
                {t("restaurar.cancelar")}
              </button>
              <button type="button" className={estiloDialogo.principal} onClick={confirmar} disabled={restaurando}>
                {restaurando ? t("restaurar.restaurando") : t("restaurar.confirmar")}
              </button>
            </>
          )}
        </div>
      </dialog>
    </>
  );
}
