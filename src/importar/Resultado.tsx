// Paso «Resultado» (spec 025): qué pasó en cada personaje al aplicar, con «Deshacer» en los que se escribieron y el
// siguiente paso, entrar al juego y comprobarlo.
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { History } from "lucide-react";
import DeshacerOperacion from "../historial/DeshacerOperacion";
import { nombreDeProducto } from "../instalaciones/nombres";
import type { ResultadoImportacion } from "../nucleo";
import { mensajeDeError } from "../textos/errores";
import css from "./Importar.module.css";

type Props = Readonly<{
  resultados: ResultadoImportacion[];
  producto: string;
  nombreDe: (cuenta: string, personaje: string) => string;
  irAHistorial: () => void;
}>;

export default function Resultado({ resultados, producto, nombreDe, irAHistorial }: Props) {
  const { t } = useTranslation();
  const [deshechas, setDeshechas] = useState<ReadonlySet<string>>(new Set());
  const algunoEscrito = resultados.some((r) => r.resultado === "completa");
  return (
    <div className={css.columna}>
      <ul className={css.resultados}>
        {resultados.map((r) => {
          const nombre = nombreDe(r.cuenta, r.personaje);
          return (
            <li key={`${r.cuenta}|${r.reino}|${r.personaje}`} className={css.personaje}>
              <h2 className={css.nombrePersonaje}>{nombre}</h2>
              <Estado resultado={r} deshecha={r.resultado === "completa" && deshechas.has(r.operacion)} />
              {r.resultado === "completa" && !deshechas.has(r.operacion) && (
                <DeshacerOperacion
                  id={r.operacion}
                  tipo="importacion"
                  nombre={nombre}
                  donde={[nombreDeProducto(t, producto), r.cuenta]}
                  alTerminar={(hecho) => {
                    if (hecho) {
                      setDeshechas((antes) => new Set(antes).add(r.operacion));
                    }
                  }}
                />
              )}
            </li>
          );
        })}
      </ul>
      {algunoEscrito && <p className={css.nota}>{t("importar.siguientePaso")}</p>}
      <button type="button" className={css.secundario} onClick={irAHistorial}>
        <History aria-hidden size={16} strokeWidth={2} />
        {t("importar.verHistorial")}
      </button>
    </div>
  );
}

function Estado({ resultado, deshecha }: Readonly<{ resultado: ResultadoImportacion; deshecha: boolean }>) {
  const { t } = useTranslation();
  switch (resultado.resultado) {
    case "completa":
      return (
        <p role="status" className={css.nota}>
          {deshecha ? t("importar.resultado.deshecha") : t("importar.resultado.completa", { count: resultado.ficheros })}
        </p>
      );
    case "fallida":
      return (
        <p role="alert" className={css.alerta}>
          {t("importar.resultado.fallida")} {mensajeDeError(t, resultado.error)}
        </p>
      );
    default:
      return <p className={css.nota}>{t(`importar.resultado.${resultado.resultado}`)}</p>;
  }
}
