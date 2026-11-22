// La configuración de addons en el paso «Qué se importa» (#72): una lista por addon con cada pieza. Los ficheros
// enteros salen desmarcados y con su aviso: el paquete no dice quién lo hizo ni qué llevan dentro.
import { useId } from "react";
import { useTranslation } from "react-i18next";
import { TriangleAlert } from "lucide-react";
import type { EntradaPaquete } from "../nucleo";
import { porAddon } from "../comun/porAddon";
import { tamanoLegible } from "../textos/formato";
import { etiquetaDePieza } from "./etiquetas";
import css from "./Importar.module.css";

type Props = Readonly<{
  entradas: readonly EntradaPaquete[];
  seleccion: ReadonlySet<string>;
  alternar: (id: string) => void;
}>;

export default function SeleccionAddons({ entradas, seleccion, alternar }: Props) {
  const { t } = useTranslation();
  if (entradas.length === 0) {
    return null;
  }
  return (
    <section className={css.addons} aria-label={t("importar.addons.titulo")}>
      <h3 className={css.grupo}>{t("importar.addons.titulo")}</h3>
      <p className={css.nota}>{t("importar.addons.explicacion")}</p>
      {porAddon(entradas).map(([addon, delAddon]) => (
        <fieldset key={addon} className={css.entradas}>
          <legend>{addon}</legend>
          {delAddon.map((entrada) => (
            <Pieza key={entrada.id} entrada={entrada} marcada={seleccion.has(entrada.id)} alternar={alternar} />
          ))}
        </fieldset>
      ))}
    </section>
  );
}

function Pieza({
  entrada,
  marcada,
  alternar,
}: Readonly<{ entrada: EntradaPaquete; marcada: boolean; alternar: (id: string) => void }>) {
  const { t, i18n } = useTranslation();
  const aviso = useId();
  return (
    <div>
      <label className={css.entrada}>
        <input
          type="checkbox"
          checked={marcada}
          onChange={() => alternar(entrada.id)}
          aria-describedby={entrada.generica ? aviso : undefined}
        />
        <span className={css.categoria}>{etiquetaDePieza(t, entrada)}</span>
        {entrada.deCuenta && <span className={css.secundarioTexto}>{t("importar.afectaCuenta")}</span>}
        <span className={css.tamano}>{tamanoLegible(entrada.bytes, i18n.language)}</span>
      </label>
      {entrada.generica && (
        <p id={aviso} className={css.aviso}>
          <TriangleAlert aria-hidden size={16} strokeWidth={1.75} />
          {t("importar.addons.generico")}
        </p>
      )}
    </div>
  );
}
