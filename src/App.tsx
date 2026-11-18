import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { HardDrive, History, Info, Users, type LucideIcon } from "lucide-react";
import AcercaDe from "./AcercaDe";
import css from "./App.module.css";
import Instalaciones from "./instalaciones/Instalaciones";
import Historial from "./historial/Historial";
import { useDeteccion } from "./instalaciones/useDeteccion";
import { pedirInfoApp, type InfoApp } from "./nucleo";
import AvisoRecuperacion from "./operaciones/AvisoRecuperacion";
import Personajes from "./personajes/Personajes";

type Estado =
  | { tipo: "cargando" }
  | { tipo: "listo"; info: InfoApp }
  | { tipo: "error"; mensaje: string };

/** Vistas de la ventana. Con cuatro basta un estado; cuando lleguen los flujos con pasos se medirá un router. */
type Vista = "personajes" | "instalaciones" | "historial" | "acercaDe";

const ICONOS: Record<Vista, LucideIcon> = {
  personajes: Users,
  instalaciones: HardDrive,
  historial: History,
  acercaDe: Info,
};

function App() {
  const { t } = useTranslation();
  const [estado, setEstado] = useState<Estado>({ tipo: "cargando" });
  const [vista, setVista] = useState<Vista>("personajes");
  // La detección vive aquí para que Personajes e Instalaciones vean lo mismo y cambiar de vista no relea el disco.
  const deteccion = useDeteccion();

  useEffect(() => {
    pedirInfoApp()
      .then((info) => setEstado({ tipo: "listo", info }))
      .catch((error: unknown) => setEstado({ tipo: "error", mensaje: String(error) }));
  }, []);

  return (
    <div className={css.armazon}>
      <aside className={css.barra}>
        <div className={css.marca}>
          <Emblema />
          {estado.tipo === "cargando" && <p className={css.secundario}>{t("cargando")}</p>}
          {estado.tipo === "listo" && <p className={css.nombreApp}>{estado.info.nombre}</p>}
          {estado.tipo === "error" && (
            <p role="alert" className={css.alerta}>
              {t("errorNucleo", { mensaje: estado.mensaje })}
            </p>
          )}
        </div>
        <nav className={css.navegacion} aria-label={t("nav.etiqueta")}>
          <BotonVista vista="personajes" actual={vista} elegir={setVista} />
          <BotonVista vista="instalaciones" actual={vista} elegir={setVista} />
          <BotonVista vista="historial" actual={vista} elegir={setVista} />
          {estado.tipo === "listo" && <BotonVista vista="acercaDe" actual={vista} elegir={setVista} />}
        </nav>
        {estado.tipo === "listo" && <p className={css.pie}>{t("versionApp", { version: estado.info.version })}</p>}
      </aside>
      <main className={css.contenido}>
        <AvisoRecuperacion alRestaurar={deteccion.recargar} />
        {vista === "personajes" && <Personajes {...deteccion} />}
        {vista === "instalaciones" && <Instalaciones {...deteccion} />}
        {vista === "historial" && <Historial alCambiarWtf={deteccion.recargar} />}
        {vista === "acercaDe" && estado.tipo === "listo" && <AcercaDe info={estado.info} />}
      </main>
    </div>
  );
}

function BotonVista({
  vista,
  actual,
  elegir,
}: Readonly<{ vista: Vista; actual: Vista; elegir: (vista: Vista) => void }>) {
  const { t } = useTranslation();
  const Icono = ICONOS[vista];
  return (
    <button type="button" className={css.pestana} aria-pressed={vista === actual} onClick={() => elegir(vista)}>
      <Icono aria-hidden size={18} strokeWidth={1.75} />
      {t(`nav.${vista}`)}
    </button>
  );
}

/** Id del degradado del emblema: sale una sola vez en la ventana, así que basta uno fijo. */
const DEGRADADO = "emblema-oro";

/** Emblema propio de la app: un escudo con un galón. Dibujo original, nada de material de Blizzard. */
function Emblema() {
  return (
    <svg className={css.emblema} width="34" height="34" viewBox="0 0 34 34" aria-hidden>
      <defs>
        <linearGradient id={DEGRADADO} x1="0" y1="0" x2="0" y2="1">
          <stop offset="0" className={css.emblemaAlto} />
          <stop offset="1" className={css.emblemaBajo} />
        </linearGradient>
      </defs>
      <path d="M17 2 L30 8 V17 C30 25 24 30 17 32 C10 30 4 25 4 17 V8 Z" stroke={`url(#${DEGRADADO})`} strokeWidth="2" />
      <path
        d="M11 13 L17 23 L23 13"
        stroke={`url(#${DEGRADADO})`}
        strokeWidth="2.2"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
}

export default App;
