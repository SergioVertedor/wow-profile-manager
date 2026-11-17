// La lista de personajes de una cuenta (spec 017): cambio de cuenta si hay más de una, buscador si la lista es larga
// y reinos fuera de Forever. Se reutilizará para elegir el destino al importar.
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { ChevronDown, Search } from "lucide-react";
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
  elegirCuenta: (carpeta: string) => void;
  personaje: Personaje | null;
  elegirPersonaje: (clave: string) => void;
}>;

export default function Plantel(props: Props) {
  const { t } = useTranslation();
  const { cuenta, producto, personaje, elegirPersonaje } = props;
  const [consulta, setConsulta] = useState("");
  const visibles = filtrar(cuenta.personajes, consulta);
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
            <section key={grupo.reino} aria-label={grupo.reino}>
              <h3 className={css.reino}>
                {grupo.reino}
                <span>{grupo.personajes.length}</span>
              </h3>
              <Filas personajes={grupo.personajes} elegida={elegida} elegir={elegirPersonaje} />
            </section>
          ))
        ) : (
          <Filas personajes={visibles} elegida={elegida} elegir={elegirPersonaje} />
        )}
      </div>
    </section>
  );
}

/** Con una cuenta, su nombre; con varias, un desplegable para cambiar. */
function CabeceraCuenta({ cuentas, cuenta, etiquetaCuenta, elegirCuenta }: Props) {
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
