// Paso «Seleccionar» (#63): qué entradas se importan y, para las de cuenta, si se aplican en cada cuenta de los
// destinos. Una entrada de cuenta sin decidir llega a la vista previa como conflicto. Abre en el modo simple (#105).
import { useTranslation } from "react-i18next";
import { TriangleAlert } from "lucide-react";
import { agrupar, type Modo } from "../comun/modoSimple";
import SeleccionSimple from "../comun/SeleccionSimple";
import SelectorModo from "../comun/SelectorModo";
import type { ResumenPaquete } from "../nucleo";
import { tamanoLegible } from "../textos/formato";
import { etiquetaDeCategoria, etiquetaDePieza } from "./etiquetas";
import QueTrae from "./QueTrae";
import SeleccionAddons from "./SeleccionAddons";
import SeleccionCarpetas from "./SeleccionCarpetas";
import { claveDeDecision, decisionesPendientes, elegibleEnSimple, type Destino } from "./importacion";
import css from "./Importar.module.css";

type Props = Readonly<{
  resumen: ResumenPaquete;
  destinos: readonly Destino[];
  seleccion: ReadonlySet<string>;
  cambiarSeleccion: (seleccion: ReadonlySet<string>) => void;
  decisiones: ReadonlyMap<string, boolean>;
  cambiarDecisiones: (decisiones: ReadonlyMap<string, boolean>) => void;
}>;

type PropsPaso = Props & Readonly<{ modo: Modo; cambiarModo: (modo: Modo) => void }>;

export default function PasoSeleccion({ modo, cambiarModo, ...props }: PropsPaso) {
  const { t } = useTranslation();
  const { resumen, seleccion, cambiarSeleccion } = props;
  const teclas = resumen.entradas.some((e) => e.categoria === "teclas" && seleccion.has(e.id));
  return (
    <div className={css.columna}>
      <SelectorModo modo={modo} cambiar={cambiarModo} />
      {modo === "simple" ? (
        <SeleccionSimple
          grupos={agrupar(resumen.entradas, elegibleEnSimple)}
          marcadas={seleccion}
          cambiar={cambiarSeleccion}
          avisos={{
            carpetasAddons: (
              <p className={css.aviso}>
                <TriangleAlert aria-hidden size={16} strokeWidth={1.75} />
                {t("importar.carpetas.aviso")}
              </p>
            ),
          }}
        />
      ) : (
        <SeleccionAvanzada {...props} />
      )}
      {/* En los dos modos: lo de cuenta marcado en avanzado sigue pidiendo su decisión. */}
      <DecisionesDeCuenta {...props} />
      <p className={css.nota}>{t("exportar.barras")}</p>
      {teclas && <p className={css.nota}>{t("importar.teclasSinComprobar")}</p>}
    </div>
  );
}

/** El detalle de siempre: entrada a entrada, la configuración de addons por addon y las carpetas. */
function SeleccionAvanzada({ resumen, seleccion, cambiarSeleccion }: Props) {
  const { t, i18n } = useTranslation();
  const alternar = (id: string) => {
    const siguiente = new Set(seleccion);
    if (!siguiente.delete(id)) {
      siguiente.add(id);
    }
    cambiarSeleccion(siguiente);
  };
  // La configuración de addons que se puede importar va agrupada por addon; la que no, con el resto y su motivo.
  const esDeAddon = (e: (typeof resumen.entradas)[number]) => e.categoria === "config-addon" && e.incompatible === undefined;
  // Las carpetas de addons (#73), aparte y con su aviso.
  const esCarpeta = (e: (typeof resumen.entradas)[number]) => e.categoria === "addon" && e.incompatible === undefined;
  const deAddons = resumen.entradas.filter(esDeAddon);
  const carpetas = resumen.entradas.filter(esCarpeta);
  const resto = resumen.entradas.filter((e) => !esDeAddon(e) && !esCarpeta(e));
  return (
    <>
      <fieldset className={css.entradas}>
        <legend>{t("importar.queSeImporta")}</legend>
        {resto.map((entrada) => (
          <div key={entrada.id}>
            <label className={css.entrada} data-incompatible={entrada.incompatible !== undefined}>
              <input
                type="checkbox"
                checked={entrada.incompatible === undefined && seleccion.has(entrada.id)}
                disabled={entrada.incompatible !== undefined}
                onChange={() => alternar(entrada.id)}
              />
              <span className={css.categoria}>{etiquetaDeCategoria(t, entrada.categoria, entrada.deCuenta)}</span>
              {entrada.incompatible !== undefined && (
                <span className={css.secundarioTexto}>{t(`importar.incompatible.${entrada.incompatible}`)}</span>
              )}
              <span className={css.tamano}>{tamanoLegible(entrada.bytes, i18n.language)}</span>
            </label>
            <QueTrae entrada={entrada} />
          </div>
        ))}
      </fieldset>
      <SeleccionAddons entradas={deAddons} seleccion={seleccion} alternar={alternar} />
      <SeleccionCarpetas
        entradas={carpetas}
        seleccion={seleccion}
        alternar={alternar}
        cambiarSeleccion={cambiarSeleccion}
      />
    </>
  );
}

/** Cada entrada de cuenta marcada, en cada cuenta de los destinos: aplicarla o no. Empieza sin decidir. */
function DecisionesDeCuenta({ resumen, destinos, seleccion, decisiones, cambiarDecisiones }: Props) {
  const { t } = useTranslation();
  const pendientes = decisionesPendientes(resumen, seleccion, destinos);
  if (pendientes.length === 0) {
    return null;
  }
  const decidir = (clave: string, aceptar: boolean) => cambiarDecisiones(new Map(decisiones).set(clave, aceptar));
  return (
    <section className={css.decisiones} aria-label={t("importar.decisionesEtiqueta")}>
      <p className={css.nota}>{t("importar.decisionesExplicacion")}</p>
      {pendientes.map(({ cuenta, entrada }) => {
        const clave = claveDeDecision(cuenta, entrada);
        const datos = resumen.entradas.find((e) => e.id === entrada);
        const categoria = datos?.categoria ?? entrada;
        const nombre =
          datos?.categoria === "config-addon"
            ? `${datos.addon ?? ""}: ${etiquetaDePieza(t, datos)}`
            : etiquetaDeCategoria(t, categoria, true);
        const personajes = destinos.filter((d) => d.pedido.cuenta === cuenta).map((d) => d.nombre);
        const decidida = decisiones.get(clave);
        return (
          <fieldset key={clave} className={css.decision}>
            <legend>
              {t("importar.enCuenta", {
                categoria: nombre,
                cuenta,
                personajes: personajes.join(", "),
              })}
            </legend>
            <label>
              <input type="radio" name={clave} checked={decidida === true} onChange={() => decidir(clave, true)} />
              {t("importar.aplicar")}
            </label>
            <label>
              <input type="radio" name={clave} checked={decidida === false} onChange={() => decidir(clave, false)} />
              {t("importar.noAplicar")}
            </label>
            {datos !== undefined && <QueTrae entrada={datos} />}
          </fieldset>
        );
      })}
    </section>
  );
}
