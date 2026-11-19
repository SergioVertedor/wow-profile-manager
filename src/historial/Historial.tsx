// Vista «Historial» (#54 y #63): las operaciones de la app, de la más nueva a la más antigua, con lo que ocupan.
// Desde aquí se deshace una eliminación o una importación, se restaura una importación que se cortó a mitad, se abre la
// carpeta de una operación y se borra una para liberar espacio. Nada se borra solo (D-04). Una que no se pudo restaurar
// se señala mientras exista, con lo que falta y dónde está su copia.
import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { FolderOpen, TriangleAlert } from "lucide-react";
import { abrirCarpetaOperacion, pedirHistorial, type Historial as DatosHistorial, type Operacion } from "../nucleo";
import { nombreDeProducto } from "../instalaciones/nombres";
import { mensajeDeError } from "../textos/errores";
import { fechaLegible, tamanoLegible } from "../textos/formato";
import BorrarOperacion from "./BorrarOperacion";
import DeshacerOperacion from "./DeshacerOperacion";
import css from "./Historial.module.css";
import RestaurarOperacion from "./RestaurarOperacion";

/** Estados en los que la operación aún no ha terminado: no se borra. */
const A_MEDIAS: ReadonlySet<Operacion["estado"]> = new Set(["preparando", "copiada", "escribiendo"]);
/** Estados que piden atención: algo falló. */
const FALLIDAS: ReadonlySet<Operacion["estado"]> = new Set(["fallida-restaurada", "fallida-sin-restaurar"]);

type Estado = { tipo: "cargando" } | { tipo: "listo"; historial: DatosHistorial } | { tipo: "error"; error: unknown };

/** `alCambiarWtf`: tras deshacer, la lista de personajes leída antes ya no vale. */
export default function Historial({ alCambiarWtf }: Readonly<{ alCambiarWtf: () => Promise<void> }>) {
  const { t, i18n } = useTranslation();
  const [estado, setEstado] = useState<Estado>({ tipo: "cargando" });
  // Un fallo al abrir una carpeta: se enseña arriba, sin perder la lista.
  const [fallo, setFallo] = useState<unknown>(null);

  const recargar = useCallback(() => {
    pedirHistorial()
      .then((historial) => setEstado({ tipo: "listo", historial }))
      .catch((error: unknown) => setEstado({ tipo: "error", error }));
  }, []);
  useEffect(recargar, [recargar]);

  const abrir = (id: string) => {
    setFallo(null);
    abrirCarpetaOperacion(id).catch(setFallo);
  };
  const trasDeshacer = () => {
    recargar();
    void alCambiarWtf();
  };

  return (
    <section className={css.pantalla} aria-labelledby="titulo-historial">
      <div className={css.cabecera}>
        <h1 id="titulo-historial" className={css.titulo}>
          {t("historial.titulo")}
        </h1>
        {estado.tipo === "listo" && (
          <p className={css.total}>
            {t("historial.total", { tamano: tamanoLegible(estado.historial.bytes, i18n.language) })}
          </p>
        )}
      </div>
      <p className={css.nota}>{t("historial.explicacion")}</p>
      {fallo !== null && (
        <p role="alert" className={css.alerta}>
          {mensajeDeError(t, fallo)}
        </p>
      )}
      {estado.tipo === "cargando" && <p className={css.nota}>{t("historial.cargando")}</p>}
      {estado.tipo === "error" && (
        <p role="alert" className={css.alerta}>
          {mensajeDeError(t, estado.error)}
        </p>
      )}
      {estado.tipo === "listo" && (
        <Lista historial={estado.historial} abrir={abrir} recargar={recargar} trasDeshacer={trasDeshacer} />
      )}
    </section>
  );
}

type PropsLista = Readonly<{
  historial: DatosHistorial;
  abrir: (id: string) => void;
  recargar: () => void;
  trasDeshacer: () => void;
}>;

function Lista({ historial, abrir, recargar, trasDeshacer }: PropsLista) {
  const { t, i18n } = useTranslation();
  if (historial.operaciones.length === 0 && historial.ilegibles.length === 0) {
    return <p className={css.nota}>{t("historial.vacio")}</p>;
  }
  return (
    <>
      {historial.ilegibles.length > 0 && (
        <div role="alert" className={css.aviso}>
          <TriangleAlert aria-hidden size={16} strokeWidth={1.75} />
          <div>
            <p>{t("historial.ilegibles", { count: historial.ilegibles.length })}</p>
            <ul className={css.ilegibles}>
              {historial.ilegibles.map((ilegible) => (
                <li key={ilegible.id}>
                  {ilegible.id} · {tamanoLegible(ilegible.bytes, i18n.language)}
                  <BotonAbrir id={ilegible.id} abrir={abrir} />
                </li>
              ))}
            </ul>
          </div>
        </div>
      )}
      <ol className={css.lista}>
        {historial.operaciones.map((operacion) => (
          <Fila
            key={operacion.id}
            operacion={operacion}
            abrir={abrir}
            recargar={recargar}
            trasDeshacer={trasDeshacer}
          />
        ))}
      </ol>
    </>
  );
}

type PropsFila = Readonly<{
  operacion: Operacion;
  abrir: (id: string) => void;
  recargar: () => void;
  trasDeshacer: () => void;
}>;

function Fila({ operacion, abrir, recargar, trasDeshacer }: PropsFila) {
  const { t, i18n } = useTranslation();
  const nombre = operacion.destino.nombreEnDatos ?? operacion.destino.carpetaPersonaje;
  const donde = [nombreDeProducto(t, operacion.destino.producto), operacion.destino.cuenta];
  const fallida = FALLIDAS.has(operacion.estado);
  const tipo = t(`historial.tipo.${operacion.tipo}`);
  return (
    <li className={css.tarjeta} aria-label={`${tipo}: ${nombre}`}>
      <div className={css.arriba}>
        <p className={css.nombre}>
          <span className={css.tipo}>{tipo}</span> {nombre}
        </p>
        <span className={fallida ? css.estadoFallido : css.estado}>{t(`historial.estado.${operacion.estado}`)}</span>
      </div>
      <p className={css.detalle}>
        {[
          fechaLegible(operacion.creada, i18n.language),
          ...donde,
          t("historial.ficheros", { count: operacion.ficheros }),
          tamanoLegible(operacion.bytes, i18n.language),
        ].join(" · ")}
      </p>
      {operacion.paquete !== null && (
        <p className={css.detalle}>{t("historial.perfil", { nombre: operacion.paquete.nombre })}</p>
      )}
      <Relacion operacion={operacion} />
      {fallida && operacion.error !== null && <p className={css.error}>{operacion.error}</p>}
      {operacion.sinDevolver.length > 0 && <SinDevolver operacion={operacion} />}
      <div className={css.acciones}>
        {operacion.deshacible && operacion.tipo !== "deshacer" && (
          <DeshacerOperacion
            id={operacion.id}
            tipo={operacion.tipo}
            nombre={nombre}
            donde={donde}
            alTerminar={trasDeshacer}
          />
        )}
        {operacion.porResolver && (
          <RestaurarOperacion id={operacion.id} nombre={nombre} donde={donde} alTerminar={trasDeshacer} />
        )}
        <BotonAbrir id={operacion.id} abrir={abrir} />
        {!A_MEDIAS.has(operacion.estado) && (
          <BorrarOperacion
            id={operacion.id}
            bytes={operacion.bytes}
            sinRestaurar={operacion.estado === "fallida-sin-restaurar"}
            esDeshacer={operacion.tipo === "deshacer"}
            alBorrar={recargar}
          />
        )}
      </div>
    </li>
  );
}

/** Qué deshace o quién la deshizo. */
function Relacion({ operacion }: Readonly<{ operacion: Operacion }>) {
  const { t } = useTranslation();
  if (operacion.deshaceBorrada) {
    return <p className={css.detalle}>{t("historial.deshaceBorrada")}</p>;
  }
  if (operacion.deshace !== null) {
    return <p className={css.detalle}>{t("historial.deshace", { id: operacion.deshace })}</p>;
  }
  if (operacion.deshechaPor !== null) {
    return <p className={css.detalle}>{t("historial.deshechaPor", { id: operacion.deshechaPor })}</p>;
  }
  return null;
}

function SinDevolver({ operacion }: Readonly<{ operacion: Operacion }>) {
  const { t } = useTranslation();
  return (
    <div role="alert" className={css.aviso}>
      <TriangleAlert aria-hidden size={16} strokeWidth={1.75} />
      <div>
        <p>{t(operacion.aMedias ? "historial.aMedias" : "historial.sinDevolver")}</p>
        <ul className={css.ficheros}>
          {operacion.sinDevolver.map(({ ruta, copia }) => (
            <li key={ruta}>
              {copia === null ? t("historial.sobra", { ruta }) : t("historial.copiaDe", { ruta, copia })}
            </li>
          ))}
        </ul>
      </div>
    </div>
  );
}

function BotonAbrir({ id, abrir }: Readonly<{ id: string; abrir: (id: string) => void }>) {
  const { t } = useTranslation();
  return (
    <button type="button" className={css.accion} onClick={() => abrir(id)}>
      <FolderOpen aria-hidden size={14} strokeWidth={1.75} />
      {t("historial.abrir")}
    </button>
  );
}
