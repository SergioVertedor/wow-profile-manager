// «Aplicar» (spec 025): botón y confirmación al pie de la vista previa. Manda al núcleo la huella del plan que se ve: si
// algo cambió desde entonces, no escribe nada y la vista previa se vuelve a pedir. Una operación por personaje.
import { useId, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { Download } from "lucide-react";
import {
  aplicarImportacion,
  type PedidoImportacion,
  type PlanPersonaje,
  type ResultadoImportacion,
  type ResumenPaquete,
  type VistaPrevia,
} from "../nucleo";
import estiloDialogo from "../estilo/Dialogo.module.css";
import { mensajeDeError } from "../textos/errores";
import { aEscribir, type Destino } from "./importacion";
import css from "./Importar.module.css";

type Props = Readonly<{
  resumen: ResumenPaquete;
  vista: VistaPrevia;
  destinos: readonly Destino[];
  pedido: PedidoImportacion;
  nombreDe: (p: PlanPersonaje) => string;
  alAplicar: (resultados: ResultadoImportacion[]) => void;
  /** Tras un fallo: el plan que se ve puede no ser ya el del disco. */
  alFallar: () => void;
}>;

type Paso = { tipo: "preguntando" } | { tipo: "aplicando" } | { tipo: "error"; error: unknown };

/** Por qué no se puede aplicar, o `null` si se puede. */
function bloqueo(vista: VistaPrevia): "juegoAbierto" | "conflictos" | "nada" | null {
  const { personajes } = vista.plan;
  if (vista.juegoAbierto) {
    return "juegoAbierto";
  }
  if (personajes.some((p) => p.conflictos.length > 0)) {
    return "conflictos";
  }
  return personajes.some((p) => aEscribir(p).length > 0) ? null : "nada";
}

export default function AplicarImportacion({ resumen, vista, destinos, pedido, nombreDe, alAplicar, alFallar }: Props) {
  const { t } = useTranslation();
  const dialogo = useRef<HTMLDialogElement>(null);
  const [paso, setPaso] = useState<Paso>({ tipo: "preguntando" });
  const idTitulo = useId();
  const motivo = bloqueo(vista);
  const aplicando = paso.tipo === "aplicando";
  const conCuenta = vista.plan.personajes.some((p) => aEscribir(p).some((f) => f.deCuenta));

  const abrir = () => {
    setPaso({ tipo: "preguntando" });
    dialogo.current?.showModal();
  };
  const confirmar = () => {
    setPaso({ tipo: "aplicando" });
    aplicarImportacion(
      resumen.sha256,
      vista.huella,
      destinos.map((d) => d.pedido),
      pedido,
    )
      .then((resultados) => {
        dialogo.current?.close();
        alAplicar(resultados);
      })
      .catch((error: unknown) => setPaso({ tipo: "error", error }));
  };
  const cerrar = () => dialogo.current?.close();
  const alCerrar = () => {
    if (paso.tipo === "error") {
      alFallar();
    }
  };

  return (
    <div className={css.aplicar}>
      {motivo !== null && <p className={css.nota}>{t(`importar.noSePuedeAplicar.${motivo}`)}</p>}
      <button type="button" className={css.boton} onClick={abrir} disabled={motivo !== null}>
        <Download aria-hidden size={16} strokeWidth={2} />
        {t("importar.aplicarImportacion")}
      </button>
      <dialog
        ref={dialogo}
        className={estiloDialogo.dialogo}
        aria-labelledby={idTitulo}
        onClose={alCerrar}
        onCancel={(evento) => aplicando && evento.preventDefault()}
      >
        <h2 id={idTitulo} className={estiloDialogo.titulo}>
          {t("importar.confirmar.titulo", { perfil: resumen.manifiesto.perfil.nombre })}
        </h2>
        <ul className={estiloDialogo.ficheros}>
          {vista.plan.personajes.map((p) => (
            <li key={`${p.cuenta}|${p.reino}|${p.personaje}`}>
              {t("importar.confirmar.personaje", { nombre: nombreDe(p), count: aEscribir(p).length })}
            </li>
          ))}
        </ul>
        {conCuenta && <p className={estiloDialogo.aviso}>{t("importar.confirmar.cuenta")}</p>}
        <p>{t("importar.confirmar.copia")}</p>
        {paso.tipo === "error" && (
          <p role="alert" className={estiloDialogo.alerta}>
            {mensajeDeError(t, paso.error)}
          </p>
        )}
        <div className={estiloDialogo.acciones}>
          <button type="button" className={estiloDialogo.secundario} onClick={cerrar} disabled={aplicando}>
            {t("importar.confirmar.cancelar")}
          </button>
          {paso.tipo !== "error" && (
            <button type="button" className={estiloDialogo.principal} onClick={confirmar} disabled={aplicando}>
              {aplicando ? t("importar.confirmar.aplicando") : t("importar.confirmar.aplicar")}
            </button>
          )}
        </div>
      </dialog>
    </div>
  );
}
