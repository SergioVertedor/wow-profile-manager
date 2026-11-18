// Un bloque que se abre y se cierra (spec 026, decisión 7): `<details>` y `<summary>` del navegador, que ya son
// accesibles con teclado y lector de pantalla. Lo usan la ficha, exportar e importar.
import type { ReactNode } from "react";
import css from "./Desplegable.module.css";

type Props = Readonly<{
  /** Lo que se ve cerrado. */
  resumen: ReactNode;
  children: ReactNode;
  className?: string;
}>;

export default function Desplegable({ resumen, children, className }: Props) {
  const clases = className === undefined ? css.desplegable : `${css.desplegable} ${className}`;
  return (
    <details className={clases}>
      <summary className={css.resumen}>{resumen}</summary>
      <div className={css.cuerpo}>{children}</div>
    </details>
  );
}
