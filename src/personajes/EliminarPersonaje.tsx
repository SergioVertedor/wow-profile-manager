// «Eliminar carpeta» del personaje elegido (spec 021). Pide al núcleo cuánto hay en la carpeta, lo enseña en un
// diálogo con el aviso medido (el juego vuelve a crear la carpeta si el personaje existe) y, si se confirma, el núcleo
// la saca del WTF guardando antes una copia. La interfaz solo pasa la identidad del personaje, nunca una ruta.
import { useId, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { Trash2, TriangleAlert } from "lucide-react";
import {
  eliminarPersonaje,
  pedirResumenEliminacion,
  type PersonajePedido,
  type ResumenEliminacion,
} from "../nucleo";
import { mensajeDeError } from "../textos/errores";
import { tamanoLegible } from "../textos/formato";
import css from "./EliminarPersonaje.module.css";

type Props = Readonly<{
  pedido: PersonajePedido;
  nombre: string;
  /** Reino (si se enseña), cuenta e instalación, ya con sus etiquetas. */
  donde: string[];
  /** Se llama al cerrar tras eliminar: vuelve a leer los personajes. */
  alTerminar: () => Promise<void>;
}>;

type Paso =
  | { tipo: "inicio" }
  /** Pidiendo el resumen: el botón no responde, para no abrir dos veces. */
  | { tipo: "cargando" }
  | { tipo: "preguntando"; resumen: ResumenEliminacion }
  | { tipo: "eliminando"; resumen: ResumenEliminacion }
  | { tipo: "hecho"; id: string }
  /** `enDialogo`: falló al eliminar (se enseña en el diálogo); si no, falló al pedir el resumen. */
  | { tipo: "error"; error: unknown; enDialogo: boolean };

export default function EliminarPersonaje({ pedido, nombre, donde, alTerminar }: Props) {
  const { t } = useTranslation();
  const dialogo = useRef<HTMLDialogElement>(null);
  const [paso, setPaso] = useState<Paso>({ tipo: "inicio" });
  const idTitulo = useId();
  const idAviso = useId();
  const eliminando = paso.tipo === "eliminando";
  const resumen = paso.tipo === "preguntando" || paso.tipo === "eliminando" ? paso.resumen : null;
  const terminado = paso.tipo === "hecho" || (paso.tipo === "error" && paso.enDialogo);

  const abrir = () => {
    setPaso({ tipo: "cargando" });
    pedirResumenEliminacion(pedido)
      .then((resumen) => {
        setPaso({ tipo: "preguntando", resumen });
        dialogo.current?.showModal();
      })
      .catch((error: unknown) => setPaso({ tipo: "error", error, enDialogo: false }));
  };
  const confirmar = (resumen: ResumenEliminacion) => {
    setPaso({ tipo: "eliminando", resumen });
    eliminarPersonaje(pedido)
      .then((id) => setPaso({ tipo: "hecho", id }))
      .catch((error: unknown) => setPaso({ tipo: "error", error, enDialogo: true }));
  };
  const cerrar = () => dialogo.current?.close();
  // Esc y «Cancelar» pasan por aquí. Si se llegó a intentar eliminar (bien o mal), el disco puede haber cambiado: se
  // vuelve a leer la lista.
  const alCerrar = () => {
    setPaso({ tipo: "inicio" });
    if (terminado) {
      void alTerminar();
    }
  };

  return (
    <>
      <button type="button" className={css.boton} onClick={abrir} disabled={paso.tipo === "cargando"}>
        <Trash2 aria-hidden size={15} strokeWidth={1.75} />
        {t("eliminar.boton")}
      </button>
      {paso.tipo === "error" && !paso.enDialogo && (
        <p role="alert" className={css.alerta}>
          {mensajeDeError(t, paso.error)}
        </p>
      )}
      <dialog
        ref={dialogo}
        className={css.dialogo}
        aria-labelledby={idTitulo}
        aria-describedby={resumen === null ? undefined : idAviso}
        onClose={alCerrar}
        // Mientras se elimina no se cierra: Esc dejaría la ventana sin saber cómo acabó.
        onCancel={(evento) => eliminando && evento.preventDefault()}
      >
        <h2 id={idTitulo} className={css.titulo}>
          {t("eliminar.titulo", { nombre })}
        </h2>
        <p className={css.donde}>{donde.join(" · ")}</p>
        <Contenido paso={paso} idAviso={idAviso} />
        <div className={css.acciones}>
          {resumen !== null && (
            <>
              <button type="button" className={css.secundario} onClick={cerrar} disabled={eliminando}>
                {t("eliminar.cancelar")}
              </button>
              <button type="button" className={css.peligro} onClick={() => confirmar(resumen)} disabled={eliminando}>
                {eliminando ? t("eliminar.eliminando") : t("eliminar.confirmar")}
              </button>
            </>
          )}
          {terminado && (
            <button type="button" className={css.secundario} onClick={cerrar}>
              {t("eliminar.cerrar")}
            </button>
          )}
        </div>
      </dialog>
    </>
  );
}

function Contenido({ paso, idAviso }: Readonly<{ paso: Paso; idAviso: string }>) {
  const { t, i18n } = useTranslation();
  switch (paso.tipo) {
    case "preguntando":
    case "eliminando":
      return (
        <>
          <p className={css.ruta}>{paso.resumen.ruta}</p>
          <p>
            {t("eliminar.contenido", {
              count: paso.resumen.ficheros,
              tamano: tamanoLegible(paso.resumen.bytes, i18n.language),
            })}
          </p>
          <p id={idAviso} className={css.aviso}>
            <TriangleAlert aria-hidden size={16} strokeWidth={1.75} />
            {t("eliminar.aviso")}
          </p>
          <p>{t("eliminar.copia")}</p>
        </>
      );
    case "hecho":
      return (
        <p role="status" className={css.exito}>
          {t("eliminar.hecho", { id: paso.id })}
        </p>
      );
    case "error":
      return (
        <p role="alert" className={css.alerta}>
          {mensajeDeError(t, paso.error)}
        </p>
      );
    default:
      return null;
  }
}
