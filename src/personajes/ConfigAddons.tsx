// La configuración de addons en el diálogo de exportar (#56): una lista por addon con cada pieza, por qué va
// marcada o no y sus avisos, «marcar» por regla genérica y lo que no se puede exportar. Marcar lo decide el diálogo.
import { useId } from "react";
import { useTranslation } from "react-i18next";
import { TriangleAlert } from "lucide-react";
import Desplegable from "../comun/Desplegable";
import ListaQuitadas from "../comun/ListaQuitadas";
import {
  REGLAS_GENERICAS,
  esReglaGenerica,
  totalQuitadas,
  type NoExportable,
  type PiezaPropuesta,
} from "../nucleo";
import { porAddon } from "../comun/porAddon";
import { tamanoLegible } from "../textos/formato";
import css from "./ExportarPersonaje.module.css";

type Props = Readonly<{
  piezas: readonly PiezaPropuesta[];
  noExportables: readonly NoExportable[];
  marcadas: ReadonlySet<string>;
  alternar: (id: string) => void;
  marcar: (ids: readonly string[]) => void;
}>;

export default function ConfigAddons({ piezas, noExportables, marcadas, alternar, marcar }: Props) {
  const { t } = useTranslation();
  if (piezas.length === 0 && noExportables.length === 0) {
    return null;
  }
  return (
    <section className={css.addons} aria-label={t("exportar.addons.titulo")}>
      <h3 className={css.subtitulo}>{t("exportar.addons.titulo")}</h3>
      <p className={css.nota}>{t("exportar.addons.explicacion")}</p>
      <MarcarGenericas piezas={piezas} marcar={marcar} />
      {porAddon(piezas).map(([addon, delAddon]) => (
        <fieldset key={addon} className={css.addon}>
          <legend>{addon}</legend>
          {delAddon.map((pieza) => (
            <PiezaAddon key={pieza.id} pieza={pieza} marcada={marcadas.has(pieza.id)} alternar={alternar} />
          ))}
        </fieldset>
      ))}
      {noExportables.length > 0 && (
        <Desplegable resumen={t("exportar.addons.noExportables", { count: noExportables.length })}>
          <ul className={css.noExportables}>
            {noExportables.map((n) => (
              <li key={`${n.addon}-${n.ambito}-${n.variable ?? ""}-${n.motivo}`}>
                {n.variable === null ? n.addon : `${n.addon} · ${n.variable}`} ({t(`exportar.addons.ambito.${n.ambito}`)}):{" "}
                {t(`exportar.addons.noExportable.${n.motivo}`)}
              </li>
            ))}
          </ul>
        </Desplegable>
      )}
    </section>
  );
}

/** «Marcar» por regla genérica, sin las piezas con pistas: esas se marcan una a una, viendo su aviso. */
function MarcarGenericas({ piezas, marcar }: Readonly<{ piezas: readonly PiezaPropuesta[]; marcar: Props["marcar"] }>) {
  const { t } = useTranslation();
  const reglas = REGLAS_GENERICAS.map((regla) => ({
    regla,
    ids: piezas.filter((p) => p.regla === regla && p.pistas.length === 0).map((p) => p.id),
  })).filter(({ ids }) => ids.length > 0);
  if (reglas.length === 0) {
    return null;
  }
  return (
    <div className={css.marcarTodos}>
      {reglas.map(({ regla, ids }) => (
        <button key={regla} type="button" className={css.enlace} onClick={() => marcar(ids)}>
          {t(`exportar.addons.marcarTodos.${regla}`)}
        </button>
      ))}
      <p className={css.nota}>{t("exportar.addons.marcarTodosNota")}</p>
    </div>
  );
}

type PropsPieza = Readonly<{ pieza: PiezaPropuesta; marcada: boolean; alternar: (id: string) => void }>;

function PiezaAddon({ pieza, marcada, alternar }: PropsPieza) {
  const { t, i18n } = useTranslation();
  const idAvisos = useId();
  const avisos = avisosDe(pieza);
  return (
    <div>
      <label className={css.pieza}>
        <input
          type="checkbox"
          checked={marcada}
          onChange={() => alternar(pieza.id)}
          aria-describedby={avisos.length > 0 ? idAvisos : undefined}
        />
        <span className={css.categoria}>{nombreDePieza(t, pieza)}</span>
        <span className={css.tamano}>{tamanoLegible(pieza.bytes, i18n.language)}</span>
      </label>
      {avisos.length > 0 && (
        <ul id={idAvisos} className={css.avisos}>
          {avisos.map((aviso) => (
            <li key={aviso}>
              <TriangleAlert aria-hidden size={14} strokeWidth={1.75} />
              {t(`exportar.addons.avisos.${aviso}`)}
            </li>
          ))}
        </ul>
      )}
      {pieza.quitadas.length > 0 && (
        <Desplegable className={css.quitadas} resumen={t("exportar.quitadas", { count: totalQuitadas(pieza.quitadas) })}>
          <ListaQuitadas grupos={pieza.quitadas} />
        </Desplegable>
      )}
    </div>
  );
}

type Aviso = "generico" | "conPersonajes" | "versionNoMedida" | "guid" | "nombreReino";

function avisosDe(pieza: PiezaPropuesta): Aviso[] {
  const avisos: Aviso[] = [];
  if (esReglaGenerica(pieza.regla)) {
    avisos.push("generico");
  }
  if (pieza.regla === "genericoCuentaConPersonajes") {
    avisos.push("conPersonajes");
  }
  if (pieza.versionNoMedida) {
    avisos.push("versionNoMedida");
  }
  return [...avisos, ...pieza.pistas];
}

function nombreDePieza(t: ReturnType<typeof useTranslation>["t"], pieza: PiezaPropuesta): string {
  switch (pieza.regla) {
    case "perfilAcedb":
      return t("exportar.addons.pieza.perfilAcedb", { perfil: pieza.perfil ?? "" });
    case "adaptador":
      return pieza.perfil === null
        ? t("exportar.addons.pieza.adaptador", { variable: pieza.variable ?? "" })
        : t("exportar.addons.pieza.perfilAcedb", { perfil: pieza.perfil });
    case null:
      return t(`exportar.categorias.${pieza.categoria}`);
    default:
      return t(`exportar.addons.pieza.${pieza.regla}`);
  }
}
