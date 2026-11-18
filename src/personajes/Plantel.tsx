// La lista de personajes de una cuenta (spec 017): cambio de cuenta si hay más de una, buscador si la lista es larga
// y reinos fuera de Forever, que se pliegan y despliegan (spec 019). Se reutilizará para elegir el destino al importar.
import { useId, useState } from "react";
import { useTranslation } from "react-i18next";
import { ChevronDown, ChevronRight, Search } from "lucide-react";
import NombreEditable from "../instalaciones/NombreEditable";
import type { Cuenta, Personaje } from "../nucleo";
import css from "./Plantel.module.css";
import {
  agruparPorReino,
  claveDePersonaje,
  filtrar,
  inicial,
  muestraReinos,
  nombreVisible,
  UMBRAL_BUSCADOR,
} from "./lista";

type Props = Readonly<{
  cuentas: Cuenta[];
  cuenta: Cuenta;
  producto: string;
  etiquetaCuenta: (carpeta: string) => string;
  /** «Cuenta N» o la carpeta: lo que se ve si la cuenta elegida no tiene nombre propio. */
  cuentaPorDefecto: string;
  renombrarCuenta: (nombre: string) => Promise<void>;
  elegirCuenta: (carpeta: string) => void;
  personaje: Personaje | null;
  elegirPersonaje: (clave: string) => void;
}>;

export default function Plantel(props: Props) {
  const { t } = useTranslation();
  const { cuenta, producto, personaje, elegirPersonaje } = props;
  const [consulta, setConsulta] = useState("");
  // Solo en memoria: al cambiar de cuenta el componente se remonta (`key` en Personajes) y todo vuelve a desplegarse.
  const [plegados, setPlegados] = useState<ReadonlySet<string>>(new Set());
  const visibles = filtrar(cuenta.personajes, consulta);
  const buscando = consulta.trim() !== "";
  const alternar = (reino: string) =>
    setPlegados((antes) => {
      const despues = new Set(antes);
      if (!despues.delete(reino)) {
        despues.add(reino);
      }
      return despues;
    });
  const elegida = personaje === null ? null : claveDePersonaje(personaje);
  return (
    <section className={css.plantel} aria-label={t("plantel.etiqueta")}>
      <div className={css.controles}>
        <CabeceraCuenta {...props} />
        <p className={css.datosCuenta}>
          <span>{cuenta.carpeta}</span>
          <span>{t("plantel.personajes", { count: cuenta.personajes.length })}</span>
        </p>
        {cuenta.reinosIgnorados > 0 && (
          <p className={css.nota}>{t("plantel.reinosIgnorados", { count: cuenta.reinosIgnorados })}</p>
        )}
        {cuenta.personajes.length > UMBRAL_BUSCADOR && (
          <div className={css.campo}>
            <Search aria-hidden size={15} strokeWidth={2} className={css.iconoCampo} />
            <input
              type="search"
              className={css.buscar}
              placeholder={t("plantel.buscar")}
              aria-label={t("plantel.buscar")}
              value={consulta}
              onChange={(evento) => setConsulta(evento.target.value)}
            />
          </div>
        )}
      </div>
      <div className={css.lista}>
        {cuenta.personajes.length === 0 && <p className={css.nota}>{t("plantel.sinPersonajes")}</p>}
        {cuenta.personajes.length > 0 && visibles.length === 0 && (
          <p className={css.nota}>{t("plantel.sinResultados", { consulta: consulta.trim() })}</p>
        )}
        {muestraReinos(producto) ? (
          agruparPorReino(visibles).map((grupo) => (
            <GrupoReino
              key={grupo.reino}
              reino={grupo.reino}
              personajes={grupo.personajes}
              // Mientras se busca, lo plegado no cuenta: si no, una búsqueda podría no enseñar nada sin decir por qué.
              desplegado={buscando || !plegados.has(grupo.reino)}
              alternar={() => alternar(grupo.reino)}
              elegida={elegida}
              elegir={elegirPersonaje}
            />
          ))
        ) : (
          <Filas personajes={visibles} elegida={elegida} elegir={elegirPersonaje} />
        )}
      </div>
    </section>
  );
}

/** Un reino: la cabecera fija es el botón que pliega y despliega sus personajes; el recuento se ve siempre. */
function GrupoReino({
  reino,
  personajes,
  desplegado,
  alternar,
  elegida,
  elegir,
}: Readonly<{
  reino: string;
  personajes: Personaje[];
  desplegado: boolean;
  alternar: () => void;
  elegida: string | null;
  elegir: (clave: string) => void;
}>) {
  const idFilas = useId();
  const Chevron = desplegado ? ChevronDown : ChevronRight;
  return (
    <section aria-label={reino}>
      <h3 className={css.reino}>
        <button type="button" aria-expanded={desplegado} aria-controls={idFilas} onClick={alternar}>
          <Chevron aria-hidden size={14} strokeWidth={2.25} />
          {reino}
          <span>{personajes.length}</span>
        </button>
      </h3>
      <div id={idFilas} hidden={!desplegado}>
        <Filas personajes={personajes} elegida={elegida} elegir={elegir} />
      </div>
    </section>
  );
}

/** Con una cuenta, su nombre; con varias, un desplegable para cambiar. Al lado, el lápiz para renombrarla (spec 018). */
function CabeceraCuenta(props: Props) {
  const { t } = useTranslation();
  const { cuenta, cuentaPorDefecto, renombrarCuenta } = props;
  return (
    <NombreEditable
      etiqueta={t("plantel.nombreCuenta")}
      valor={cuenta.nombre}
      porDefecto={cuentaPorDefecto}
      guardar={renombrarCuenta}
    >
      <ElegirCuenta {...props} />
    </NombreEditable>
  );
}

function ElegirCuenta({ cuentas, cuenta, etiquetaCuenta, elegirCuenta }: Props) {
  const { t } = useTranslation();
  if (cuentas.length === 1) {
    return <h2 className={css.cuenta}>{etiquetaCuenta(cuenta.carpeta)}</h2>;
  }
  return (
    <div className={css.campo}>
      <select
        className={css.selector}
        aria-label={t("plantel.elegirCuenta")}
        value={cuenta.carpeta}
        onChange={(evento) => elegirCuenta(evento.target.value)}
      >
        {cuentas.map((c) => (
          <option key={c.carpeta} value={c.carpeta}>
            {etiquetaCuenta(c.carpeta)}
          </option>
        ))}
      </select>
      <ChevronDown aria-hidden size={16} strokeWidth={2} className={css.iconoSelector} />
    </div>
  );
}

function Filas({
  personajes,
  elegida,
  elegir,
}: Readonly<{ personajes: Personaje[]; elegida: string | null; elegir: (clave: string) => void }>) {
  return (
    <ul className={css.filas}>
      {personajes.map((personaje) => {
        const clave = claveDePersonaje(personaje);
        const nombre = nombreVisible(personaje);
        return (
          <li key={clave}>
            <button type="button" className={css.fila} aria-pressed={clave === elegida} onClick={() => elegir(clave)}>
              <span className={css.monograma} aria-hidden>
                {inicial(nombre)}
              </span>
              <span className={css.nombre}>{nombre}</span>
            </button>
          </li>
        );
      })}
    </ul>
  );
}
