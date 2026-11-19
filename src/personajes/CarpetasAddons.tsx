// Las carpetas de los addons en el diálogo de exportar (#73): salen marcadas, con el aviso de que son código que
// el juego ejecuta y de que muchos addons no permiten redistribuirse. Las que no se pueden llevar se dicen aparte, con
// su motivo: quien importe verá que le faltan. Marcar lo decide el diálogo.
import { useId } from "react";
import { useTranslation } from "react-i18next";
import { TriangleAlert } from "lucide-react";
import Desplegable from "../comun/Desplegable";
import type { CarpetaNoExportable, PiezaPropuesta } from "../nucleo";
import { tamanoLegible } from "../textos/formato";
import estiloDialogo from "../estilo/Dialogo.module.css";
import css from "./ExportarPersonaje.module.css";

type Props = Readonly<{
  piezas: readonly PiezaPropuesta[];
  noExportables: readonly CarpetaNoExportable[];
  marcadas: ReadonlySet<string>;
  alternar: (id: string) => void;
  marcar: (ids: readonly string[]) => void;
  desmarcar: (ids: readonly string[]) => void;
}>;

export default function CarpetasAddons({ piezas, noExportables, marcadas, alternar, marcar, desmarcar }: Props) {
  const { t, i18n } = useTranslation();
  const idAviso = useId();
  if (piezas.length === 0 && noExportables.length === 0) {
    return null;
  }
  const ids = piezas.map((p) => p.id);
  return (
    <section className={css.addons} aria-label={t("exportar.carpetas.titulo")}>
      <h3 className={css.subtitulo}>{t("exportar.carpetas.titulo")}</h3>
      <p id={idAviso} className={estiloDialogo.aviso}>
        <TriangleAlert aria-hidden size={16} strokeWidth={1.75} />
        {t("exportar.carpetas.aviso")}
      </p>
      {piezas.length > 0 && (
        <div className={css.marcarTodos}>
          <button type="button" className={css.enlace} onClick={() => marcar(ids)}>
            {t("exportar.carpetas.marcarTodas")}
          </button>
          <button type="button" className={css.enlace} onClick={() => desmarcar(ids)}>
            {t("exportar.carpetas.desmarcarTodas")}
          </button>
        </div>
      )}
      {piezas.map((pieza) => (
        <label key={pieza.id} className={css.pieza}>
          <input
            type="checkbox"
            checked={marcadas.has(pieza.id)}
            onChange={() => alternar(pieza.id)}
            aria-describedby={idAviso}
          />
          <span className={css.categoria}>{pieza.addon}</span>
          <span className={css.tamano}>{tamanoLegible(pieza.bytes, i18n.language)}</span>
        </label>
      ))}
      {noExportables.length > 0 && (
        <Desplegable resumen={t("exportar.carpetas.noExportables", { count: noExportables.length })}>
          <ul className={css.noExportables}>
            {noExportables.map((n) => (
              <li key={n.addon}>
                {n.addon}: {t(`exportar.carpetas.motivo.${n.motivo.codigo}`, { detalle: n.motivo.detalle ?? "" })}
              </li>
            ))}
          </ul>
        </Desplegable>
      )}
    </section>
  );
}
