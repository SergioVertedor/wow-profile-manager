// Qué trae una entrada del paquete (spec 026): una frase que dice qué es y, si el núcleo la da, la lista de lo que
// trae. Va en «Qué se importa» y en las decisiones de cuenta, para decidir viendo lo que se copia.
import { useTranslation } from "react-i18next";
import Desplegable from "../comun/Desplegable";
import type { Contenido, EntradaPaquete } from "../nucleo";
import { esCategoria } from "./etiquetas";
import css from "./Importar.module.css";

type Props = Readonly<{ entrada: EntradaPaquete }>;

/** Nada si la entrada no se importa: no hay qué enseñar de algo que no se va a escribir. */
export default function QueTrae({ entrada }: Props) {
  const { t } = useTranslation();
  if (entrada.incompatible !== undefined || !esCategoria(entrada.categoria)) {
    return null;
  }
  return (
    <Desplegable className={css.queTrae} resumen={t("importar.queTrae.boton")}>
      <p>{t(`importar.queTrae.${entrada.categoria}`)}</p>
      {entrada.contenido !== undefined && <ListaContenido contenido={entrada.contenido} />}
    </Desplegable>
  );
}

function ListaContenido({ contenido }: Readonly<{ contenido: Contenido }>) {
  const { t } = useTranslation();
  if (contenido.tipo === "lista") {
    return <Lista elementos={contenido.elementos} />;
  }
  return (
    <>
      {contenido.activados.length > 0 && (
        <>
          <p className={css.subtitulo}>
            {t("importar.queTrae.activados", {
              count: contenido.activados.length,
            })}
          </p>
          <Lista elementos={contenido.activados} />
        </>
      )}
      {contenido.desactivados.length > 0 && (
        <>
          <p className={css.subtitulo}>
            {t("importar.queTrae.desactivados", {
              count: contenido.desactivados.length,
            })}
          </p>
          <Lista elementos={contenido.desactivados} />
        </>
      )}
    </>
  );
}

function Lista({ elementos }: Readonly<{ elementos: readonly string[] }>) {
  return (
    <ul className={css.contenido}>
      {elementos.map((elemento, i) => (
        // Dos macros sin nombre pueden empezar por la misma línea: el índice distingue.
        <li key={`${i}-${elemento}`}>{elemento}</li>
      ))}
    </ul>
  );
}
