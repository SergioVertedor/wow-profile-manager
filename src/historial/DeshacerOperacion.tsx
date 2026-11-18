// «Deshacer» una eliminación del historial (spec 022). Pide al núcleo el plan (qué vuelve y qué cambia si el juego ya
// recreó la carpeta), lo enseña en un diálogo y, si se confirma, deshace con la huella del plan: si el disco cambió
// entre medias, el núcleo no escribe nada. La interfaz solo manda el id de la operación.
import { useId, useRef, useState } from "react";
import type { TFunction } from "i18next";
import { useTranslation } from "react-i18next";
import { Undo2, TriangleAlert } from "lucide-react";
import { deshacerOperacion, pedirPlanDeshacer, type PlanDeshacer } from "../nucleo";
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
  | { tipo: "inicio" }
  | { tipo: "cargando" }
  | { tipo: "preguntando"; plan: PlanDeshacer }
  | { tipo: "deshaciendo"; plan: PlanDeshacer }
  | { tipo: "hecho"; id: string }
  /** `enDialogo`: falló al deshacer; si no, falló al pedir el plan. */
  | { tipo: "error"; error: unknown; enDialogo: boolean };

export default function DeshacerOperacion({ id, nombre, donde, alTerminar }: Props) {
  const { t } = useTranslation();
  const dialogo = useRef<HTMLDialogElement>(null);
  const [paso, setPaso] = useState<Paso>({ tipo: "inicio" });
  const idTitulo = useId();
  const deshaciendo = paso.tipo === "deshaciendo";
  const plan = paso.tipo === "preguntando" || paso.tipo === "deshaciendo" ? paso.plan : null;
  const terminado = paso.tipo === "hecho" || (paso.tipo === "error" && paso.enDialogo);

  const abrir = () => {
    setPaso({ tipo: "cargando" });
    pedirPlanDeshacer(id)
      .then((plan) => {
        setPaso({ tipo: "preguntando", plan });
        dialogo.current?.showModal();
      })
      .catch((error: unknown) => setPaso({ tipo: "error", error, enDialogo: false }));
  };
  const confirmar = (plan: PlanDeshacer) => {
    setPaso({ tipo: "deshaciendo", plan });
    deshacerOperacion(id, plan.huella)
      .then((nuevo) => setPaso({ tipo: "hecho", id: nuevo }))
      .catch((error: unknown) => setPaso({ tipo: "error", error, enDialogo: true }));
  };
  const cerrar = () => dialogo.current?.close();
  const alCerrar = () => {
    setPaso({ tipo: "inicio" });
    if (terminado) {
      alTerminar();
    }
  };

  return (
    <>
      <button type="button" className={css.accion} onClick={abrir} disabled={paso.tipo === "cargando"}>
        <Undo2 aria-hidden size={14} strokeWidth={1.75} />
        {t("historial.deshacer")}
      </button>
      {paso.tipo === "error" && !paso.enDialogo && (
        <p role="alert" className={css.alerta}>
          {mensajeDeError(t, paso.error)}
        </p>
      )}
      <dialog
        ref={dialogo}
        className={estiloDialogo.dialogo}
        aria-labelledby={idTitulo}
        onClose={alCerrar}
        onCancel={(evento) => deshaciendo && evento.preventDefault()}
      >
        <h2 id={idTitulo} className={estiloDialogo.titulo}>
          {t("deshacer.titulo", { nombre })}
        </h2>
        <p className={estiloDialogo.donde}>{donde.join(" · ")}</p>
        <Contenido paso={paso} />
        <div className={estiloDialogo.acciones}>
          {plan !== null && (
            <>
              <button type="button" className={estiloDialogo.secundario} onClick={cerrar} disabled={deshaciendo}>
                {t("deshacer.cancelar")}
              </button>
              <button
                type="button"
                className={estiloDialogo.principal}
                onClick={() => confirmar(plan)}
                disabled={deshaciendo}
              >
                {deshaciendo ? t("deshacer.deshaciendo") : t("deshacer.confirmar")}
              </button>
            </>
          )}
          {terminado && (
            <button type="button" className={estiloDialogo.secundario} onClick={cerrar}>
              {t("deshacer.cerrar")}
            </button>
          )}
        </div>
      </dialog>
    </>
  );
}

/** Lo que vuelve y lo que se quita, o que no hay nada que escribir. */
function resumenDelPlan(t: TFunction, plan: PlanDeshacer): string {
  if (plan.vuelven + plan.sobran === 0) {
    return t("deshacer.nada");
  }
  const partes = plan.vuelven > 0 ? [t("deshacer.contenido", { count: plan.vuelven })] : [];
  if (plan.sobran > 0) {
    partes.push(t("deshacer.sobran", { count: plan.sobran }));
  }
  return partes.join(" ");
}

function Contenido({ paso }: Readonly<{ paso: Paso }>) {
  const { t } = useTranslation();
  switch (paso.tipo) {
    case "preguntando":
    case "deshaciendo": {
      const escribe = paso.plan.vuelven + paso.plan.sobran > 0;
      return (
        <>
          <p className={estiloDialogo.ruta}>{resumenDelPlan(t, paso.plan)}</p>
          {paso.plan.cambian.length > 0 && (
            <div className={estiloDialogo.aviso}>
              <TriangleAlert aria-hidden size={16} strokeWidth={1.75} />
              <div>
                <p>{t("deshacer.aviso")}</p>
                <ul className={estiloDialogo.ficheros}>
                  {paso.plan.cambian.map((ruta) => (
                    <li key={ruta}>{ruta}</li>
                  ))}
                </ul>
              </div>
            </div>
          )}
          {escribe && <p>{t("deshacer.copia")}</p>}
        </>
      );
    }
    case "hecho":
      return (
        <p role="status" className={estiloDialogo.exito}>
          {t("deshacer.hecho", { id: paso.id })}
        </p>
      );
    case "error":
      return (
        <p role="alert" className={estiloDialogo.alerta}>
          {mensajeDeError(t, paso.error)}
        </p>
      );
    default:
      return null;
  }
}
