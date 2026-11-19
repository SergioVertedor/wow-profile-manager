// «Borrar» una operación terminada del historial (#54): libera su espacio y quita la posibilidad de deshacerla, y la
// confirmación lo dice así. Una que no se pudo restaurar del todo exige además marcar que ya se restauró a mano.
import { useId, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { Trash2, TriangleAlert } from "lucide-react";
import { borrarOperacion } from "../nucleo";
import estiloDialogo from "../estilo/Dialogo.module.css";
import { mensajeDeError } from "../textos/errores";
import { tamanoLegible } from "../textos/formato";
import css from "./Historial.module.css";

type Props = Readonly<{
  id: string;
  bytes: number;
  /** Una `fallida-sin-restaurar`: sus copias pueden ser lo único que queda de esos ficheros. */
  sinRestaurar: boolean;
  /** Un `deshacer`: sus copias son lo que el juego guardó después de eliminar, y se pierden. */
  esDeshacer: boolean;
  /** Se llama tras borrarla: la lista ya no la tiene. */
  alBorrar: () => void;
}>;

type Paso = { tipo: "preguntando" } | { tipo: "borrando" } | { tipo: "error"; error: unknown };

export default function BorrarOperacion({ id, bytes, sinRestaurar, esDeshacer, alBorrar }: Props) {
  const { t, i18n } = useTranslation();
  const dialogo = useRef<HTMLDialogElement>(null);
  const [paso, setPaso] = useState<Paso>({ tipo: "preguntando" });
  const [restauradaAMano, setRestauradaAMano] = useState(false);
  const idTitulo = useId();
  const idCasilla = useId();
  const borrando = paso.tipo === "borrando";

  const abrir = () => {
    setPaso({ tipo: "preguntando" });
    setRestauradaAMano(false);
    dialogo.current?.showModal();
  };
  const confirmar = () => {
    setPaso({ tipo: "borrando" });
    borrarOperacion(id, restauradaAMano)
      .then(() => {
        dialogo.current?.close();
        alBorrar();
      })
      .catch((error: unknown) => setPaso({ tipo: "error", error }));
  };

  return (
    <>
      <button type="button" className={`${css.accion} ${css.peligro}`} onClick={abrir}>
        <Trash2 aria-hidden size={14} strokeWidth={1.75} />
        {t("historial.borrar")}
      </button>
      <dialog
        ref={dialogo}
        className={estiloDialogo.dialogo}
        aria-labelledby={idTitulo}
        onCancel={(evento) => borrando && evento.preventDefault()}
      >
        <h2 id={idTitulo} className={estiloDialogo.titulo}>
          {t("borrar.titulo", { id })}
        </h2>
        <p>{t("borrar.explicacion", { tamano: tamanoLegible(bytes, i18n.language) })}</p>
        {esDeshacer && <p>{t("borrar.deshacer")}</p>}
        {sinRestaurar && (
          <>
            <p className={estiloDialogo.aviso}>
              <TriangleAlert aria-hidden size={16} strokeWidth={1.75} />
              {t("borrar.sinRestaurar")}
            </p>
            <label id={idCasilla} className={estiloDialogo.casilla}>
              <input
                type="checkbox"
                checked={restauradaAMano}
                onChange={(evento) => setRestauradaAMano(evento.target.checked)}
                disabled={borrando}
              />
              {t("borrar.confirmarManual")}
            </label>
          </>
        )}
        {paso.tipo === "error" && (
          <p role="alert" className={estiloDialogo.alerta}>
            {mensajeDeError(t, paso.error)}
          </p>
        )}
        <div className={estiloDialogo.acciones}>
          <button
            type="button"
            className={estiloDialogo.secundario}
            onClick={() => dialogo.current?.close()}
            disabled={borrando}
          >
            {t("borrar.cancelar")}
          </button>
          <button
            type="button"
            className={estiloDialogo.peligro}
            onClick={confirmar}
            aria-describedby={sinRestaurar ? idCasilla : undefined}
            disabled={borrando || (sinRestaurar && !restauradaAMano)}
          >
            {borrando ? t("borrar.borrando") : t("borrar.confirmar")}
          </button>
        </div>
      </dialog>
    </>
  );
}
