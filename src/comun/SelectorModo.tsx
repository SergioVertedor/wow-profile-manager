// «Simple · Avanzado» (#105): cambiar de modo no toca lo marcado.
import { useId } from "react";
import { useTranslation } from "react-i18next";
import type { Modo } from "./modoSimple";
import css from "./ModoSimple.module.css";

const MODOS: readonly Modo[] = ["simple", "avanzado"];

type Props = Readonly<{ modo: Modo; cambiar: (modo: Modo) => void; bloqueado?: boolean }>;

export default function SelectorModo({ modo, cambiar, bloqueado = false }: Props) {
  const { t } = useTranslation();
  const nombre = useId();
  return (
    <fieldset className={css.selector} disabled={bloqueado}>
      <legend className={css.oculto}>{t("modo.etiqueta")}</legend>
      {MODOS.map((opcion) => (
        <label key={opcion} className={css.opcion} data-elegido={opcion === modo}>
          <input type="radio" name={nombre} checked={opcion === modo} onChange={() => cambiar(opcion)} />
          {t(`modo.${opcion}`)}
        </label>
      ))}
    </fieldset>
  );
}
