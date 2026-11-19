// Las carpetas de addons en el paso «Qué se importa» (spec 036): salen marcadas, con el aviso de que es código que el
// juego ejecuta. Solo se instalan las que el destino no tiene: eso lo dice la vista previa.
import { useId } from "react";
import { useTranslation } from "react-i18next";
import { TriangleAlert } from "lucide-react";
import type { EntradaPaquete } from "../nucleo";
import { tamanoLegible } from "../textos/formato";
import css from "./Importar.module.css";

type Props = Readonly<{
  entradas: readonly EntradaPaquete[];
  seleccion: ReadonlySet<string>;
  alternar: (id: string) => void;
  cambiarSeleccion: (seleccion: ReadonlySet<string>) => void;
}>;

export default function SeleccionCarpetas({ entradas, seleccion, alternar, cambiarSeleccion }: Props) {
  const { t, i18n } = useTranslation();
  const idAviso = useId();
  if (entradas.length === 0) {
    return null;
  }
  const ids = entradas.map((e) => e.id);
  const marcarTodas = () => cambiarSeleccion(new Set([...seleccion, ...ids]));
  const desmarcarTodas = () => cambiarSeleccion(new Set([...seleccion].filter((id) => !ids.includes(id))));
  return (
    <section className={css.addons} aria-label={t("importar.carpetas.titulo")}>
      <h3 className={css.grupo}>{t("importar.carpetas.titulo")}</h3>
      <p id={idAviso} className={css.aviso}>
        <TriangleAlert aria-hidden size={16} strokeWidth={1.75} />
        {t("importar.carpetas.aviso")}
      </p>
      <div className={css.acciones}>
        <button type="button" className={css.secundario} onClick={marcarTodas}>
          {t("importar.carpetas.marcarTodas")}
        </button>
        <button type="button" className={css.secundario} onClick={desmarcarTodas}>
          {t("importar.carpetas.desmarcarTodas")}
        </button>
      </div>
      <fieldset className={css.entradas}>
        <legend>{t("importar.carpetas.lista")}</legend>
        {entradas.map((entrada) => (
          <label key={entrada.id} className={css.entrada}>
            <input
              type="checkbox"
              checked={seleccion.has(entrada.id)}
              onChange={() => alternar(entrada.id)}
              aria-describedby={idAviso}
            />
            <span className={css.categoria}>{entrada.addon}</span>
            <span className={css.tamano}>{tamanoLegible(entrada.bytes, i18n.language)}</span>
          </label>
        ))}
      </fieldset>
    </section>
  );
}
