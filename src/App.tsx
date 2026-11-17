import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { HardDrive, ScrollText, type LucideIcon } from "lucide-react";
import AcercaDe from "./AcercaDe";
import css from "./App.module.css";
import Instalaciones from "./Instalaciones";
import { pedirInfoApp, type InfoApp } from "./nucleo";

type Estado =
  | { tipo: "cargando" }
  | { tipo: "listo"; info: InfoApp }
  | { tipo: "error"; mensaje: string };

/** Vistas de la ventana. Con dos basta un estado; cuando lleguen los flujos con pasos se medirá un router. */
type Vista = "instalaciones" | "acercaDe";

const ICONOS: Record<Vista, LucideIcon> = { instalaciones: HardDrive, acercaDe: ScrollText };

function App() {
  const { t } = useTranslation();
  const [estado, setEstado] = useState<Estado>({ tipo: "cargando" });
  const [vista, setVista] = useState<Vista>("instalaciones");

  useEffect(() => {
    pedirInfoApp()
      .then((info) => setEstado({ tipo: "listo", info }))
      .catch((error: unknown) => setEstado({ tipo: "error", mensaje: String(error) }));
  }, []);

  return (
    <div className={css.armazon}>
      <aside className={css.barra}>
        <header className={css.marca}>
          {estado.tipo === "cargando" && <p className={css.secundario}>{t("cargando")}</p>}
          {estado.tipo === "listo" && (
            <h1 className={css.titulo}>{t("cabecera", { nombre: estado.info.nombre, version: estado.info.version })}</h1>
          )}
          {estado.tipo === "error" && (
            <p role="alert" className={css.alerta}>
              {t("errorNucleo", { mensaje: estado.mensaje })}
            </p>
          )}
        </header>
        <nav className={css.navegacion} aria-label={t("nav.etiqueta")}>
          <BotonVista vista="instalaciones" actual={vista} elegir={setVista} />
          {estado.tipo === "listo" && <BotonVista vista="acercaDe" actual={vista} elegir={setVista} />}
        </nav>
      </aside>
      <main className={css.contenido}>
        {vista === "instalaciones" && <Instalaciones />}
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

export default App;
