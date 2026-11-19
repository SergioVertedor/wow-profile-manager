// Al arrancar, la interfaz pregunta al núcleo si hay una versión nueva (spec 035). El núcleo decide si sale a la red:
// con el interruptor apagado, o en desarrollo, responde que no sin mirar. Si la hay, se pregunta antes de instalar.
import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { Download } from "lucide-react";
import { buscarActualizacion, instalarActualizacion, type ActualizacionDisponible } from "../nucleo";
import { mensajeDeError } from "../textos/errores";
import estiloDialogo from "../estilo/Dialogo.module.css";
import css from "./AvisoRecuperacion.module.css";

type Estado =
  | { tipo: "nada" }
  | { tipo: "disponible"; version: ActualizacionDisponible }
  | { tipo: "instalando"; version: ActualizacionDisponible }
  | { tipo: "error"; version: ActualizacionDisponible; error: unknown };

export default function AvisoActualizacion() {
  const { t } = useTranslation();
  const [estado, setEstado] = useState<Estado>({ tipo: "nada" });
  // Una sola vez por arranque: en desarrollo, StrictMode repite el efecto.
  const pedida = useRef(false);

  useEffect(() => {
    if (pedida.current) {
      return;
    }
    pedida.current = true;
    // Sin conexión o sin versión nueva no hay nada que decir: el núcleo lo deja en el registro.
    buscarActualizacion()
      .then((version) => {
        if (version !== null) {
          setEstado({ tipo: "disponible", version });
        }
      })
      .catch(() => undefined);
  }, []);

  if (estado.tipo === "nada") {
    return null;
  }
  const { version } = estado;
  const instalar = () => {
    setEstado({ tipo: "instalando", version });
    // Si sale bien, la app se reinicia y esta promesa no vuelve.
    instalarActualizacion().catch((error: unknown) => setEstado({ tipo: "error", version, error }));
  };
  const instalando = estado.tipo === "instalando";
  return (
    <div className={css.avisos}>
      <div role="status" className={css.aviso}>
        <Download aria-hidden size={16} strokeWidth={1.75} />
        <div>
          <p>{t("actualizacion.disponible", { version: version.version })}</p>
          {estado.tipo === "error" && (
            <p role="alert" className={css.alerta}>
              {mensajeDeError(t, estado.error)}
            </p>
          )}
          <div className={estiloDialogo.acciones}>
            <button
              type="button"
              className={estiloDialogo.secundario}
              disabled={instalando}
              onClick={() => setEstado({ tipo: "nada" })}
            >
              {t("actualizacion.ahoraNo")}
            </button>
            <button type="button" className={estiloDialogo.principal} disabled={instalando} onClick={instalar}>
              {instalando ? t("actualizacion.instalando") : t("actualizacion.instalar")}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
