// Vista «Exportar» del personaje elegido (#56, #73 y #107). Ocupa la pantalla Personajes, sin diálogo encima: pide al
// núcleo qué se puede llevar, lo enseña con lo que el núcleo marca por defecto y, con el nombre del perfil, le pide que
// arme el `.wowprofile`. El diálogo de guardar lo abre el núcleo: la interfaz solo pasa la identidad del personaje y lo
// marcado, nunca una ruta. Exportar no escribe en el WTF.
import { useEffect, useId, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { ArrowLeft, TriangleAlert } from "lucide-react";
import Desplegable from "../comun/Desplegable";
import ListaQuitadas from "../comun/ListaQuitadas";
import { agrupar, type Modo } from "../comun/modoSimple";
import SeleccionSimple from "../comun/SeleccionSimple";
import SelectorModo from "../comun/SelectorModo";
import CarpetasAddons from "./CarpetasAddons";
import ConfigAddons from "./ConfigAddons";
import {
  exportarPersonaje,
  pedirPropuestaExportacion,
  totalQuitadas,
  type PersonajePedido,
  type PiezaPropuesta,
  type PropuestaExportacion,
} from "../nucleo";
import { mensajeDeError } from "../textos/errores";
import { tamanoLegible } from "../textos/formato";
import estiloDialogo from "../estilo/Dialogo.module.css";
import css from "./ExportarPersonaje.module.css";

/** El personaje que se exporta, tal como lo enseña la ficha. */
export type DatosExportacion = Readonly<{
  pedido: PersonajePedido;
  nombre: string;
  /** Reino (si se enseña), cuenta e instalación, ya con sus etiquetas. */
  donde: string[];
}>;

type Props = DatosExportacion & Readonly<{ volver: () => void }>;

/** Los mismos límites que comprueba el núcleo (`exportacion.rs`). */
const MAX_NOMBRE = 80;
const MAX_DESCRIPCION = 500;

type Paso =
  /** Pidiendo qué se puede exportar. */
  | { tipo: "cargando" }
  | { tipo: "eligiendo"; propuesta: PropuestaExportacion; error: unknown }
  | { tipo: "exportando"; propuesta: PropuestaExportacion }
  | { tipo: "hecho"; fichero: string }
  /** Falló al pedir qué se puede exportar. */
  | { tipo: "error"; error: unknown };

export default function ExportarPersonaje({ pedido, nombre, donde, volver }: Props) {
  const { t } = useTranslation();
  const [paso, setPaso] = useState<Paso>({ tipo: "cargando" });
  const [marcadas, setMarcadas] = useState<ReadonlySet<string>>(new Set());
  // Siempre abre en simple (#105): el modo no se recuerda.
  const [modo, setModo] = useState<Modo>("simple");
  const [perfil, setPerfil] = useState("");
  const [descripcion, setDescripcion] = useState("");
  const idTitulo = useId();
  const titulo = useRef<HTMLHeadingElement>(null);
  const exportando = paso.tipo === "exportando";
  const propuesta = paso.tipo === "eligiendo" || paso.tipo === "exportando" ? paso.propuesta : null;
  const listo = perfil.trim() !== "" && marcadas.size > 0;

  // La vista sustituye a la pantalla: el foco va a su título, no se queda en el botón que ya no está.
  useEffect(() => titulo.current?.focus(), []);

  // Se pide al abrir la vista; si se vuelve antes de que llegue, la respuesta se ignora.
  useEffect(() => {
    let vigente = true;
    pedirPropuestaExportacion(pedido)
      .then((propuesta) => {
        if (vigente) {
          setMarcadas(new Set(propuesta.piezas.filter((p) => p.marcada).map((p) => p.id)));
          setPaso({ tipo: "eligiendo", propuesta, error: null });
        }
      })
      .catch((error: unknown) => {
        if (vigente) {
          setPaso({ tipo: "error", error });
        }
      });
    return () => {
      vigente = false;
    };
  }, [pedido]);

  const exportar = (propuesta: PropuestaExportacion) => {
    setPaso({ tipo: "exportando", propuesta });
    // Se mandan en el orden de la propuesta: el paquete sale igual marque el usuario en el orden que marque.
    const seleccion = propuesta.piezas.map((p) => p.id).filter((id) => marcadas.has(id));
    exportarPersonaje(pedido, seleccion, perfil, descripcion)
      .then((resultado) =>
        setPaso(
          resultado.resultado === "exportada"
            ? { tipo: "hecho", fichero: resultado.fichero }
            : { tipo: "eligiendo", propuesta, error: null },
        ),
      )
      .catch((error: unknown) => setPaso({ tipo: "eligiendo", propuesta, error }));
  };
  const alternar = (id: string) =>
    setMarcadas((antes) => {
      const nuevas = new Set(antes);
      if (!nuevas.delete(id)) {
        nuevas.add(id);
      }
      return nuevas;
    });
  const marcar = (ids: readonly string[]) => setMarcadas((antes) => new Set([...antes, ...ids]));
  const desmarcar = (ids: readonly string[]) =>
    setMarcadas((antes) => new Set([...antes].filter((id) => !ids.includes(id))));

  return (
    <section className={css.vista} aria-labelledby={idTitulo}>
      <header>
        <h1 id={idTitulo} ref={titulo} tabIndex={-1} className={css.titulo}>
          {t("exportar.titulo", { nombre })}
        </h1>
        <p className={estiloDialogo.donde}>{donde.join(" · ")}</p>
      </header>
      <div className={css.cuerpo}>
        {paso.tipo === "cargando" && <p role="status" className={css.nota}>{t("exportar.cargando")}</p>}
        {paso.tipo === "error" && (
          <p role="alert" className={estiloDialogo.alerta}>
            {mensajeDeError(t, paso.error)}
          </p>
        )}
        {propuesta !== null && (
          <Formulario
            propuesta={propuesta}
            marcadas={marcadas}
            cambiar={setMarcadas}
            alternar={alternar}
            marcar={marcar}
            desmarcar={desmarcar}
            modo={modo}
            setModo={setModo}
            perfil={perfil}
            setPerfil={setPerfil}
            descripcion={descripcion}
            setDescripcion={setDescripcion}
            bloqueado={exportando}
          />
        )}
        {paso.tipo === "eligiendo" && paso.error !== null && (
          <p role="alert" className={estiloDialogo.alerta}>
            {mensajeDeError(t, paso.error)}
          </p>
        )}
        {paso.tipo === "hecho" && (
          <p role="status" className={estiloDialogo.exito}>
            {t("exportar.hecho", { fichero: paso.fichero })}
          </p>
        )}
      </div>
      {/* Mientras se guarda no se vuelve: el diálogo de Windows sigue abierto encima. */}
      <div className={css.pie}>
        <button type="button" className={estiloDialogo.secundario} onClick={volver} disabled={exportando}>
          <ArrowLeft aria-hidden size={16} strokeWidth={1.75} />
          {t("exportar.volver")}
        </button>
        {propuesta !== null && (
          <button
            type="button"
            className={estiloDialogo.principal}
            onClick={() => exportar(propuesta)}
            disabled={exportando || !listo}
          >
            {exportando ? t("exportar.exportando") : t("exportar.confirmar")}
          </button>
        )}
      </div>
    </section>
  );
}

type PropsFormulario = Readonly<{
  propuesta: PropuestaExportacion;
  marcadas: ReadonlySet<string>;
  cambiar: (marcadas: ReadonlySet<string>) => void;
  alternar: (id: string) => void;
  marcar: (ids: readonly string[]) => void;
  desmarcar: (ids: readonly string[]) => void;
  modo: Modo;
  setModo: (modo: Modo) => void;
  perfil: string;
  setPerfil: (valor: string) => void;
  descripcion: string;
  setDescripcion: (valor: string) => void;
  bloqueado: boolean;
}>;

function Formulario({
  propuesta,
  marcadas,
  cambiar,
  alternar,
  marcar,
  desmarcar,
  modo,
  setModo,
  perfil,
  setPerfil,
  descripcion,
  setDescripcion,
  bloqueado,
}: PropsFormulario) {
  const { t, i18n } = useTranslation();
  const idPiezas = useId();
  const ficheros = propuesta.piezas.filter((p) => p.categoria !== "config-addon" && p.categoria !== "addon");
  const deAddons = propuesta.piezas.filter((p) => p.categoria === "config-addon");
  const carpetas = propuesta.piezas.filter((p) => p.categoria === "addon");
  const hayPiezas = propuesta.piezas.length > 0;
  return (
    <>
      {propuesta.juegoAbierto && (
        <p className={estiloDialogo.aviso}>
          <TriangleAlert aria-hidden size={16} strokeWidth={1.75} />
          {t("exportar.juegoAbierto")}
        </p>
      )}
      {hayPiezas ? (
        <SelectorModo modo={modo} cambiar={setModo} bloqueado={bloqueado} />
      ) : (
        <>
          <p className={css.nada}>{t("exportar.nada")}</p>
          <ConfigAddons
            piezas={[]}
            noExportables={propuesta.noExportables}
            marcadas={marcadas}
            alternar={alternar}
            marcar={marcar}
          />
          <CarpetasAddons
            piezas={[]}
            noExportables={propuesta.carpetasNoExportables}
            marcadas={marcadas}
            alternar={alternar}
            marcar={marcar}
            desmarcar={desmarcar}
          />
        </>
      )}
      {hayPiezas && modo === "simple" && (
        <SeleccionSimple
          grupos={agrupar(propuesta.piezas, (p) => p.marcada)}
          marcadas={marcadas}
          cambiar={cambiar}
          bloqueado={bloqueado}
          avisos={{
            carpetasAddons: (
              <p className={estiloDialogo.aviso}>
                <TriangleAlert aria-hidden size={16} strokeWidth={1.75} />
                {t("exportar.carpetas.aviso")}
              </p>
            ),
          }}
        />
      )}
      {hayPiezas && modo === "avanzado" && (
        <fieldset className={css.piezas} aria-describedby={idPiezas} disabled={bloqueado}>
          <legend>{t("exportar.piezas")}</legend>
          {ficheros.map((pieza) => (
            <div key={pieza.id}>
              <label className={css.pieza}>
                <input type="checkbox" checked={marcadas.has(pieza.id)} onChange={() => alternar(pieza.id)} />
                <span className={css.categoria}>
                  {t(`exportar.categorias.${pieza.categoria}`)}
                  <DetallePieza pieza={pieza} />
                </span>
                <span className={css.tamano}>{tamanoLegible(pieza.bytes, i18n.language)}</span>
              </label>
              {pieza.quitadas.length > 0 && (
                <Desplegable
                  className={css.quitadas}
                  resumen={t("exportar.quitadas", { count: totalQuitadas(pieza.quitadas) })}
                >
                  <ListaQuitadas grupos={pieza.quitadas} />
                </Desplegable>
              )}
            </div>
          ))}
          <ConfigAddons
            piezas={deAddons}
            noExportables={propuesta.noExportables}
            marcadas={marcadas}
            alternar={alternar}
            marcar={marcar}
          />
          <CarpetasAddons
            piezas={carpetas}
            noExportables={propuesta.carpetasNoExportables}
            marcadas={marcadas}
            alternar={alternar}
            marcar={marcar}
            desmarcar={desmarcar}
          />
        </fieldset>
      )}
      <p id={idPiezas} className={css.nota}>
        {t("exportar.barras")}
      </p>
      {propuesta.apartadas.length > 0 && (
        <p className={estiloDialogo.aviso}>
          <TriangleAlert aria-hidden size={16} strokeWidth={1.75} />
          {t("exportar.apartadas", { piezas: propuesta.apartadas.join(", ") })}
        </p>
      )}
      {hayPiezas && (
        <div className={css.campos}>
          <label className={css.campo}>
            {t("exportar.nombre")}
            <input
              type="text"
              value={perfil}
              maxLength={MAX_NOMBRE}
              required
              disabled={bloqueado}
              onChange={(evento) => setPerfil(evento.target.value)}
            />
          </label>
          <label className={css.campo}>
            {t("exportar.descripcion")}
            <textarea
              value={descripcion}
              maxLength={MAX_DESCRIPCION}
              rows={2}
              disabled={bloqueado}
              onChange={(evento) => setDescripcion(evento.target.value)}
            />
          </label>
        </div>
      )}
    </>
  );
}

/** Lo que distingue a una pieza: el diseño del Modo Edición que viaja, o que viene de la cuenta. */
function DetallePieza({ pieza }: Readonly<{ pieza: PiezaPropuesta }>) {
  const { t } = useTranslation();
  if (pieza.categoria === "modo-edicion") {
    return <span className={css.detalle}> · {t("exportar.diseno", { nombre: pieza.perfil ?? "" })}</span>;
  }
  return pieza.ambito === "cuenta" ? <span className={css.detalle}> · {t("exportar.deLaCuenta")}</span> : null;
}
