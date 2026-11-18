// «Exportar» del personaje elegido (specs 023 y 027). Pide al núcleo qué se puede llevar, lo enseña en un diálogo con lo
// que el núcleo marca por defecto, y con el nombre del perfil le pide que arme el `.wowprofile`. El diálogo de guardar lo abre el núcleo: la
// interfaz solo pasa la identidad del personaje y lo marcado, nunca una ruta. Exportar no escribe en el WTF.
import { useId, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { PackageOpen, TriangleAlert } from "lucide-react";
import Desplegable from "../comun/Desplegable";
import ListaQuitadas from "../comun/ListaQuitadas";
import ConfigAddons from "./ConfigAddons";
import {
  exportarPersonaje,
  pedirPropuestaExportacion,
  totalQuitadas,
  type PersonajePedido,
  type PropuestaExportacion,
} from "../nucleo";
import { mensajeDeError } from "../textos/errores";
import { tamanoLegible } from "../textos/formato";
import estiloDialogo from "../estilo/Dialogo.module.css";
import css from "./ExportarPersonaje.module.css";

type Props = Readonly<{
  pedido: PersonajePedido;
  nombre: string;
  /** Reino (si se enseña), cuenta e instalación, ya con sus etiquetas. */
  donde: string[];
}>;

/** Los mismos límites que comprueba el núcleo (`exportacion.rs`). */
const MAX_NOMBRE = 80;
const MAX_DESCRIPCION = 500;

type Paso =
  | { tipo: "inicio" }
  /** Pidiendo qué se puede exportar: el botón no responde, para no abrir dos veces. */
  | { tipo: "cargando" }
  | { tipo: "eligiendo"; propuesta: PropuestaExportacion; error: unknown }
  | { tipo: "exportando"; propuesta: PropuestaExportacion }
  | { tipo: "hecho"; fichero: string }
  /** Falló al pedir qué se puede exportar: se enseña junto al botón. */
  | { tipo: "error"; error: unknown };

export default function ExportarPersonaje({ pedido, nombre, donde }: Props) {
  const { t } = useTranslation();
  const dialogo = useRef<HTMLDialogElement>(null);
  const [paso, setPaso] = useState<Paso>({ tipo: "inicio" });
  const [marcadas, setMarcadas] = useState<ReadonlySet<string>>(new Set());
  const [perfil, setPerfil] = useState("");
  const [descripcion, setDescripcion] = useState("");
  const idTitulo = useId();
  const exportando = paso.tipo === "exportando";
  const propuesta = paso.tipo === "eligiendo" || paso.tipo === "exportando" ? paso.propuesta : null;
  const listo = perfil.trim() !== "" && marcadas.size > 0;

  const abrir = () => {
    setPaso({ tipo: "cargando" });
    pedirPropuestaExportacion(pedido)
      .then((propuesta) => {
        setMarcadas(new Set(propuesta.piezas.filter((p) => p.marcada).map((p) => p.id)));
        setPerfil("");
        setDescripcion("");
        setPaso({ tipo: "eligiendo", propuesta, error: null });
        dialogo.current?.showModal();
      })
      .catch((error: unknown) => setPaso({ tipo: "error", error }));
  };
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
  const cerrar = () => dialogo.current?.close();

  return (
    <>
      <button type="button" className={css.boton} onClick={abrir} disabled={paso.tipo === "cargando"}>
        <PackageOpen aria-hidden size={16} strokeWidth={1.75} />
        {t("exportar.boton")}
      </button>
      {paso.tipo === "error" && (
        <p role="alert" className={estiloDialogo.alerta}>
          {mensajeDeError(t, paso.error)}
        </p>
      )}
      <dialog
        ref={dialogo}
        className={estiloDialogo.dialogo}
        aria-labelledby={idTitulo}
        onClose={() => setPaso({ tipo: "inicio" })}
        // Mientras se guarda no se cierra: el diálogo de Windows sigue abierto encima.
        onCancel={(evento) => exportando && evento.preventDefault()}
      >
        <h2 id={idTitulo} className={estiloDialogo.titulo}>
          {t("exportar.titulo", { nombre })}
        </h2>
        <p className={estiloDialogo.donde}>{donde.join(" · ")}</p>
        {propuesta !== null && (
          <Formulario
            propuesta={propuesta}
            marcadas={marcadas}
            alternar={alternar}
            marcar={marcar}
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
        <div className={estiloDialogo.acciones}>
          {propuesta === null ? (
            <button type="button" className={estiloDialogo.secundario} onClick={cerrar}>
              {t("exportar.cerrar")}
            </button>
          ) : (
            <>
              <button type="button" className={estiloDialogo.secundario} onClick={cerrar} disabled={exportando}>
                {t("exportar.cancelar")}
              </button>
              <button
                type="button"
                className={estiloDialogo.principal}
                onClick={() => exportar(propuesta)}
                disabled={exportando || !listo}
              >
                {exportando ? t("exportar.exportando") : t("exportar.confirmar")}
              </button>
            </>
          )}
        </div>
      </dialog>
    </>
  );
}

type PropsFormulario = Readonly<{
  propuesta: PropuestaExportacion;
  marcadas: ReadonlySet<string>;
  alternar: (id: string) => void;
  marcar: (ids: readonly string[]) => void;
  perfil: string;
  setPerfil: (valor: string) => void;
  descripcion: string;
  setDescripcion: (valor: string) => void;
  bloqueado: boolean;
}>;

function Formulario({
  propuesta,
  marcadas,
  alternar,
  marcar,
  perfil,
  setPerfil,
  descripcion,
  setDescripcion,
  bloqueado,
}: PropsFormulario) {
  const { t, i18n } = useTranslation();
  const idPiezas = useId();
  const ficheros = propuesta.piezas.filter((p) => p.categoria !== "config-addon");
  const deAddons = propuesta.piezas.filter((p) => p.categoria === "config-addon");
  return (
    <>
      {propuesta.juegoAbierto && (
        <p className={estiloDialogo.aviso}>
          <TriangleAlert aria-hidden size={16} strokeWidth={1.75} />
          {t("exportar.juegoAbierto")}
        </p>
      )}
      {propuesta.piezas.length === 0 ? (
        <>
          <p className={css.nada}>{t("exportar.nada")}</p>
          <ConfigAddons
            piezas={[]}
            noExportables={propuesta.noExportables}
            marcadas={marcadas}
            alternar={alternar}
            marcar={marcar}
          />
        </>
      ) : (
        <fieldset className={css.piezas} aria-describedby={idPiezas} disabled={bloqueado}>
          <legend>{t("exportar.piezas")}</legend>
          {ficheros.map((pieza) => (
            <div key={pieza.id}>
              <label className={css.pieza}>
                <input type="checkbox" checked={marcadas.has(pieza.id)} onChange={() => alternar(pieza.id)} />
                <span className={css.categoria}>
                  {t(`exportar.categorias.${pieza.categoria}`)}
                  {pieza.ambito === "cuenta" && <span className={css.detalle}> · {t("exportar.deLaCuenta")}</span>}
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
      {propuesta.piezas.length > 0 && (
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
