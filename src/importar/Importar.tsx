// Vista «Importar» (specs 024 y 025): abrir un `.wowprofile`, elegir uno o varios personajes destino, elegir qué se
// importa, ver la vista previa, aplicar y ver el resultado, con «Deshacer» por personaje.
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { FolderOpen } from "lucide-react";
import { nombreDeProducto } from "../instalaciones/nombres";
import type { UsoDeteccion } from "../instalaciones/useDeteccion";
import {
  abrirPaquete,
  cerrarPaquete,
  type DecisionPerfil,
  type Deteccion,
  type ResultadoImportacion,
  type ResumenPaquete,
} from "../nucleo";
import { mensajeDeError } from "../textos/errores";
import { fechaLegible } from "../textos/formato";
import { decidirPerfil, pedidoDe, seleccionPorDefecto, type Destino } from "./importacion";
import css from "./Importar.module.css";
import PasoDestino from "./PasoDestino";
import PasoSeleccion from "./PasoSeleccion";
import Resultado from "./Resultado";
import VistaPrevia from "./VistaPrevia";

type Props = UsoDeteccion & Readonly<{ irAHistorial: () => void }>;

export default function Importar({ estado, irAHistorial }: Props) {
  const { t } = useTranslation();
  const [abierto, setAbierto] = useState<ResumenPaquete | null>(null);
  const [abriendo, setAbriendo] = useState(false);
  const [error, setError] = useState<unknown>(null);

  // Al salir de la vista, el núcleo olvida el paquete (spec 024, decisión 10). Si falla, no hay nada que hacer: el
  // siguiente que se abra lo sustituye igual.
  useEffect(
    () => () => {
      cerrarPaquete().catch(() => undefined);
    },
    [],
  );

  const abrir = () => {
    setAbriendo(true);
    setError(null);
    abrirPaquete()
      .then((apertura) => {
        if (apertura.resultado === "abierta") {
          setAbierto(apertura.resumen);
        }
      })
      .catch((motivo: unknown) => {
        // El núcleo ya olvidó el anterior: aquí también.
        setAbierto(null);
        setError(motivo);
      })
      .finally(() => setAbriendo(false));
  };

  return (
    <section className={css.pantalla}>
      <header className={css.cabecera}>
        <h1 className={css.titulo}>{t("importar.titulo")}</h1>
        <button type="button" className={css.boton} onClick={abrir} disabled={abriendo}>
          <FolderOpen aria-hidden size={16} strokeWidth={2} />
          {t(abierto === null ? "importar.abrir" : "importar.abrirOtro")}
        </button>
      </header>
      {error !== null && (
        <p role="alert" className={css.alerta}>
          {mensajeDeError(t, error)}
        </p>
      )}
      {abierto === null && <p className={css.nota}>{t("importar.explicacion")}</p>}
      {abierto !== null && <Contenido resumen={abierto} estado={estado} irAHistorial={irAHistorial} />}
    </section>
  );
}

function Contenido({
  resumen,
  estado,
  irAHistorial,
}: Readonly<{ resumen: ResumenPaquete; estado: UsoDeteccion["estado"]; irAHistorial: () => void }>) {
  const { t } = useTranslation();
  if (estado.tipo === "error") {
    return (
      <p role="alert" className={css.alerta}>
        {mensajeDeError(t, estado.error)}
      </p>
    );
  }
  if (estado.tipo !== "listo") {
    return <p className={css.nota}>{t("personajes.buscando")}</p>;
  }
  // Abrir otro paquete empieza de cero.
  return (
    <Asistente key={resumen.sha256} resumen={resumen} deteccion={estado.deteccion} irAHistorial={irAHistorial} />
  );
}

type Paso = "destino" | "seleccion" | "vistaPrevia" | "resultado";
const PASOS: readonly Paso[] = ["destino", "seleccion", "vistaPrevia", "resultado"];

type PropsAsistente = Readonly<{ resumen: ResumenPaquete; deteccion: Deteccion; irAHistorial: () => void }>;

function Asistente({ resumen, deteccion, irAHistorial }: PropsAsistente) {
  const { t } = useTranslation();
  const [paso, setPaso] = useState<Paso>("destino");
  const [resultados, setResultados] = useState<ResultadoImportacion[]>([]);
  const [destinos, setDestinos] = useState<ReadonlyMap<string, Destino>>(new Map());
  const [seleccion, setSeleccion] = useState<ReadonlySet<string>>(() => seleccionPorDefecto(resumen));
  const [decisiones, setDecisiones] = useState<ReadonlyMap<string, boolean>>(new Map());
  const [perfiles, setPerfiles] = useState<readonly DecisionPerfil[]>([]);
  const lista = [...destinos.values()];
  const indice = PASOS.indexOf(paso);
  const puedeSeguir = (paso === "destino" && destinos.size > 0) || (paso === "seleccion" && seleccion.size > 0);

  return (
    <>
      <CabeceraPaquete resumen={resumen} />
      <ol className={css.pasos} aria-label={t("importar.pasosEtiqueta")}>
        {PASOS.map((p) => (
          <li key={p} aria-current={p === paso ? "step" : undefined}>
            {t(`importar.pasos.${p}`)}
          </li>
        ))}
      </ol>
      <div className={css.paso}>
        {paso === "destino" && (
          <PasoDestino resumen={resumen} deteccion={deteccion} destinos={destinos} cambiar={setDestinos} />
        )}
        {paso === "seleccion" && (
          <PasoSeleccion
            resumen={resumen}
            destinos={lista}
            seleccion={seleccion}
            cambiarSeleccion={setSeleccion}
            decisiones={decisiones}
            cambiarDecisiones={setDecisiones}
          />
        )}
        {paso === "vistaPrevia" && (
          <VistaPrevia
            resumen={resumen}
            destinos={lista}
            pedido={pedidoDe(resumen, seleccion, lista, decisiones, perfiles)}
            decidirPerfil={(decision) => setPerfiles((actuales) => decidirPerfil(actuales, decision))}
            alAplicar={(hechos) => {
              setResultados(hechos);
              setPaso("resultado");
            }}
          />
        )}
        {paso === "resultado" && (
          <Resultado
            resultados={resultados}
            producto={resumen.manifiesto.origen.producto}
            nombreDe={(cuenta, personaje) =>
              lista.find((d) => d.pedido.cuenta === cuenta && d.pedido.personaje === personaje)?.nombre ?? personaje
            }
            irAHistorial={irAHistorial}
          />
        )}
      </div>
      <nav className={css.botones} aria-label={t("importar.pasosEtiqueta")}>
        {indice > 0 && paso !== "resultado" && (
          <button type="button" className={css.secundario} onClick={() => setPaso(PASOS[indice - 1])}>
            {t("importar.atras")}
          </button>
        )}
        {indice < PASOS.indexOf("vistaPrevia") && (
          <button
            type="button"
            className={css.boton}
            disabled={!puedeSeguir}
            onClick={() => setPaso(PASOS[indice + 1])}
          >
            {t("importar.siguiente")}
          </button>
        )}
      </nav>
    </>
  );
}

/** Lo que dice el paquete de sí mismo. */
function CabeceraPaquete({ resumen }: Readonly<{ resumen: ResumenPaquete }>) {
  const { t, i18n } = useTranslation();
  const { perfil, origen } = resumen.manifiesto;
  return (
    <section className={css.paquete} aria-label={t("importar.paquete")}>
      <h2 className={css.nombrePerfil}>{perfil.nombre}</h2>
      {perfil.descripcion !== undefined && <p>{perfil.descripcion}</p>}
      <p className={css.datos}>
        <span>{fechaLegible(perfil.creado, i18n.language)}</span>
        <span>
          {nombreDeProducto(t, origen.producto)} {origen.build}
        </span>
        <span>{t("importar.entradas", { count: resumen.entradas.length })}</span>
      </p>
    </section>
  );
}
