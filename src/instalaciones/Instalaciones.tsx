// Vista «Instalaciones» (#34): las instalaciones de WoW que conoce la app, de dónde salieron y sus versiones.
// Aquí se añade una carpeta a mano, se pone nombre a cada instalación y se quitan las añadidas a mano. El selector de
// carpetas lo abre el núcleo: la interfaz no elige rutas.
import { useState, type ReactNode } from "react";
import type { TFunction } from "i18next";
import { useTranslation } from "react-i18next";
import { FolderPlus, TriangleAlert } from "lucide-react";
import {
  anadirCarpeta,
  nombrarInstalacion,
  quitarCarpeta,
  type Alta,
  type CarpetaPerdida,
  type Deteccion,
  type Instalacion,
} from "../nucleo";
import { mensajeDeError } from "../textos/errores";
import css from "./Instalaciones.module.css";
import NombreEditable from "./NombreEditable";
import { conNombreDeInstalacion, nombreDeProducto, nombrePorDefecto, normalizarNombre } from "./nombres";
import type { UsoDeteccion } from "./useDeteccion";

/** Lo último que pasó al añadir o quitar, para decirlo. `cancelada` no se dice: el usuario ya sabe que cerró. */
type Resultado = { tipo: "alta"; alta: Exclude<Alta, "cancelada"> } | { tipo: "error"; error: unknown } | null;

export default function Instalaciones({ estado, recargar, actualizar }: UsoDeteccion) {
  const { t } = useTranslation();
  const [ocupado, setOcupado] = useState(false);
  const [resultado, setResultado] = useState<Resultado>(null);
  // Raíces que se están quitando: un segundo clic no vuelve a pedirlo.
  const [quitando, setQuitando] = useState<ReadonlySet<string>>(new Set());

  const anadir = () => {
    setOcupado(true);
    setResultado(null);
    anadirCarpeta()
      .then(async (alta) => {
        if (alta === "anadida") {
          await recargar();
        }
        setResultado(alta === "cancelada" ? null : { tipo: "alta", alta });
      })
      .catch((error: unknown) => setResultado({ tipo: "error", error }))
      .finally(() => setOcupado(false));
  };
  const quitar = (raiz: string) => {
    if (quitando.has(raiz)) {
      return;
    }
    setResultado(null);
    setQuitando((antes) => new Set(antes).add(raiz));
    quitarCarpeta(raiz)
      .then(recargar)
      .catch((error: unknown) => setResultado({ tipo: "error", error }))
      .finally(() =>
        setQuitando((antes) => {
          const despues = new Set(antes);
          despues.delete(raiz);
          return despues;
        }),
      );
  };
  const nombrar = (raiz: string) => (nombre: string) =>
    nombrarInstalacion(raiz, nombre).then(() =>
      actualizar((d) => conNombreDeInstalacion(d, raiz, normalizarNombre(nombre))),
    );

  return (
    <section className={css.pantalla}>
      <header className={css.cabecera}>
        <h1 className={css.titulo}>{t("instalaciones.titulo")}</h1>
        <button type="button" className={css.anadir} onClick={anadir} disabled={ocupado}>
          <FolderPlus aria-hidden size={17} strokeWidth={2} />
          {t("instalaciones.anadir")}
        </button>
      </header>
      <p className={css.nota}>{t("instalaciones.explicacion")}</p>
      <Mensaje resultado={resultado} />
      {estado.tipo === "cargando" && <p className={css.nota}>{t("personajes.buscando")}</p>}
      {estado.tipo === "error" && (
        <p role="alert" className={css.alerta}>
          {mensajeDeError(t, estado.error)}
        </p>
      )}
      {estado.tipo === "listo" && <Lista deteccion={estado.deteccion} nombrar={nombrar} quitar={quitar} />}
    </section>
  );
}

function Mensaje({ resultado }: Readonly<{ resultado: Resultado }>) {
  const { t } = useTranslation();
  if (resultado === null) {
    return null;
  }
  if (resultado.tipo === "error") {
    return (
      <p role="alert" className={css.alerta}>
        {mensajeDeError(t, resultado.error)}
      </p>
    );
  }
  const exito = resultado.alta === "anadida";
  // Que no sea una instalación es un error que hay que corregir: se anuncia en el acto. Lo demás es informativo.
  return (
    <p role={resultado.alta === "noEsInstalacion" ? "alert" : "status"} className={exito ? css.exito : css.aviso}>
      {!exito && <TriangleAlert aria-hidden size={16} strokeWidth={1.75} />}
      {t(`instalaciones.alta.${resultado.alta}`)}
    </p>
  );
}

type PropsLista = Readonly<{
  deteccion: Deteccion;
  nombrar: (raiz: string) => (nombre: string) => Promise<void>;
  quitar: (raiz: string) => void;
}>;

function Lista({ deteccion, nombrar, quitar }: PropsLista) {
  const { t } = useTranslation();
  const vacia = deteccion.instalaciones.length === 0 && deteccion.noEncontradas.length === 0;
  return (
    <>
      {deteccion.preferencias !== null && (
        <p role="alert" className={css.aviso}>
          <TriangleAlert aria-hidden size={16} strokeWidth={1.75} />
          {mensajeDeError(t, deteccion.preferencias)}
        </p>
      )}
      {vacia && <p className={css.nota}>{t("instalaciones.ninguna")}</p>}
      <ul className={css.lista}>
        {deteccion.instalaciones.map((instalacion) => (
          <Tarjeta key={instalacion.raiz} deteccion={deteccion} item={instalacion} nombrar={nombrar} quitar={quitar}>
            <p className={css.detalle}>
              {t(`instalaciones.fuente.${instalacion.fuente}`)} · {versiones(t, instalacion)}
            </p>
          </Tarjeta>
        ))}
        {deteccion.noEncontradas.map((perdida) => (
          <Tarjeta key={perdida.raiz} deteccion={deteccion} item={perdida} nombrar={nombrar} quitar={quitar} manual>
            <p className={css.perdida}>
              <TriangleAlert aria-hidden size={15} strokeWidth={1.75} />
              {t("instalaciones.noEncontrada")}
            </p>
          </Tarjeta>
        ))}
      </ul>
    </>
  );
}

type PropsTarjeta = Readonly<{
  deteccion: Deteccion;
  item: Instalacion | CarpetaPerdida;
  /** Una no encontrada siempre es a mano: el registro solo da las que existen. */
  manual?: boolean;
  nombrar: PropsLista["nombrar"];
  quitar: PropsLista["quitar"];
  children: ReactNode;
}>;

function Tarjeta({ deteccion, item, manual = false, nombrar, quitar, children }: PropsTarjeta) {
  const { t } = useTranslation();
  const porDefecto = nombrePorDefecto(t, deteccion, item.raiz);
  const nombre = item.nombre ?? porDefecto;
  const quitable = manual || ("fuente" in item && item.fuente === "manual");
  return (
    <li className={css.tarjeta}>
      <div className={css.arriba}>
        <NombreEditable
          etiqueta={t("instalaciones.nombre")}
          valor={item.nombre}
          porDefecto={porDefecto}
          guardar={nombrar(item.raiz)}
        >
          <h2 className={css.nombre}>{nombre}</h2>
        </NombreEditable>
        {quitable && (
          <button
            type="button"
            className={css.quitar}
            aria-label={t("instalaciones.quitarEtiqueta", { nombre })}
            onClick={() => quitar(item.raiz)}
          >
            {t("instalaciones.quitar")}
          </button>
        )}
      </div>
      <p className={css.ruta}>{item.raiz}</p>
      {children}
    </li>
  );
}

/** Las versiones con su nombre legible y cuántos personajes tiene cada una. */
function versiones(t: TFunction, instalacion: Instalacion): string {
  if (instalacion.versiones.length === 0) {
    return t("instalaciones.sinVersiones");
  }
  return instalacion.versiones.map((version) => nombreDeProducto(t, version.producto)).join(", ");
}
