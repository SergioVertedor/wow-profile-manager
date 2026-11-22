// Las casillas del modo simple (#105): una por grupo con algo elegible. Una casilla con parte del grupo marcada desde
// el modo avanzado sale indeterminada.
import { useLayoutEffect, useRef, type ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { alternarGrupo, estadoDe, type Grupo, type GrupoSimple } from "./modoSimple";
import css from "./ModoSimple.module.css";

type Props = Readonly<{
  grupos: readonly Grupo[];
  marcadas: ReadonlySet<string>;
  cambiar: (marcadas: Set<string>) => void;
  /** Lo que va bajo una casilla, como el aviso de las carpetas de addon. */
  avisos?: Partial<Record<GrupoSimple, ReactNode>>;
  bloqueado?: boolean;
}>;

export default function SeleccionSimple({ grupos, marcadas, cambiar, avisos = {}, bloqueado = false }: Props) {
  const { t } = useTranslation();
  return (
    <fieldset className={css.grupos} disabled={bloqueado}>
      <legend>{t("modo.queSeLleva")}</legend>
      {grupos.map((grupo) => (
        <div key={grupo.grupo}>
          <Casilla grupo={grupo} marcadas={marcadas} cambiar={cambiar} />
          {avisos[grupo.grupo]}
        </div>
      ))}
    </fieldset>
  );
}

function Casilla({ grupo, marcadas, cambiar }: Readonly<{ grupo: Grupo } & Pick<Props, "marcadas" | "cambiar">>) {
  const { t } = useTranslation();
  const casilla = useRef<HTMLInputElement>(null);
  const estado = estadoDe(grupo, marcadas);
  // `indeterminate` no es un atributo de HTML: solo se pone por la propiedad del elemento.
  useLayoutEffect(() => {
    if (casilla.current !== null) {
      casilla.current.indeterminate = estado === "indeterminado";
    }
  }, [estado]);
  return (
    <label className={css.grupo}>
      <input
        ref={casilla}
        type="checkbox"
        checked={estado === "marcado"}
        onChange={() => cambiar(alternarGrupo(grupo, marcadas))}
      />
      <span>{t(`modo.grupos.${grupo.grupo}`)}</span>
    </label>
  );
}
