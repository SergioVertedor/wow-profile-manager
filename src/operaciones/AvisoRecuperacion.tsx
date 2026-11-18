// Al arrancar, el núcleo termina las operaciones que se cortaron (la app se cerró o se fue la luz a mitad) y la
// interfaz dice qué ha hecho. Si no había nada que terminar, no enseña nada.
import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { TriangleAlert } from "lucide-react";
import { recuperarOperaciones, type Recuperacion } from "../nucleo";
import { mensajeDeError } from "../textos/errores";
import css from "./AvisoRecuperacion.module.css";

/** En este orden: primero lo que pide algo al usuario o le dice que algo quedó mal. */
const CASOS = ["sinRestaurar", "pendientes", "ilegibles", "restauradas", "canceladas"] as const;
const PIDEN_ATENCION: ReadonlySet<(typeof CASOS)[number]> = new Set(["sinRestaurar", "pendientes", "ilegibles"]);

type Estado = { tipo: "esperando" } | { tipo: "listo"; recuperacion: Recuperacion } | { tipo: "error"; error: unknown };

/** `alRestaurar`: si se devolvieron ficheros al WTF, la lista de personajes leída antes ya no vale. */
export default function AvisoRecuperacion({ alRestaurar }: Readonly<{ alRestaurar: () => Promise<void> }>) {
  const { t } = useTranslation();
  const [estado, setEstado] = useState<Estado>({ tipo: "esperando" });
  // Una sola vez por arranque: en desarrollo, StrictMode repite el efecto y la segunda respuesta (nada que hacer)
  // taparía la primera.
  const pedida = useRef(false);

  useEffect(() => {
    if (pedida.current) {
      return;
    }
    pedida.current = true;
    recuperarOperaciones()
      .then((recuperacion) => {
        setEstado({ tipo: "listo", recuperacion });
        if (recuperacion.restauradas > 0 || recuperacion.sinRestaurar > 0) {
          void alRestaurar();
        }
      })
      .catch((error: unknown) => setEstado({ tipo: "error", error }));
  }, [alRestaurar]);

  if (estado.tipo === "esperando") {
    return null;
  }
  if (estado.tipo === "error") {
    return (
      <p role="alert" className={css.alerta}>
        {mensajeDeError(t, estado.error)}
      </p>
    );
  }
  const casos = CASOS.filter((caso) => estado.recuperacion[caso] > 0);
  if (casos.length === 0) {
    return null;
  }
  return (
    <div className={css.avisos}>
      {casos.map((caso) => (
        <p key={caso} role={PIDEN_ATENCION.has(caso) ? "alert" : "status"} className={css.aviso}>
          <TriangleAlert aria-hidden size={16} strokeWidth={1.75} />
          {t(`recuperacion.${caso}`, { count: estado.recuperacion[caso] })}
        </p>
      ))}
    </div>
  );
}
