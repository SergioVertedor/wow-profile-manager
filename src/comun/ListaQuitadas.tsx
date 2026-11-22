// Lo que no viaja, agrupado por motivo (#65): cada grupo con la frase que lo explica y sus claves. Exportar e
// importar lo comparten; importar cambia la frase de los addons, porque allí sí se escriben (#63, decisión 9).
import { useTranslation } from "react-i18next";
import type { Motivo, Quitadas } from "../nucleo";
import css from "./ListaQuitadas.module.css";

/** El texto de un motivo: el común o, al importar, el de los addons que faltan. */
export type TextoDeMotivo = `motivos.${Motivo}` | "importar.addonsQueFaltan";

type Props = Readonly<{
  grupos: readonly Quitadas[];
  /** La clave del texto de cada motivo. Por omisión, `motivos.<motivo>`. */
  textoDe?: (motivo: Motivo) => TextoDeMotivo;
}>;

const TEXTO_COMUN = (motivo: Motivo): TextoDeMotivo => `motivos.${motivo}`;

export default function ListaQuitadas({ grupos, textoDe = TEXTO_COMUN }: Props) {
  const { t } = useTranslation();
  return (
    <>
      {grupos.map((grupo) => (
        <div key={grupo.motivo} className={css.grupo}>
          <p>{t(textoDe(grupo.motivo))}</p>
          <ul className={css.claves}>
            {grupo.claves.map((clave) => (
              <li key={clave}>{clave}</li>
            ))}
          </ul>
        </div>
      ))}
    </>
  );
}
